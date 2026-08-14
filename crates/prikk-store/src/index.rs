//! Object index: append-only, rebuildable, off the durability path (RFC 102 Stage 3, design-v1.md
//! §4), and the write protocol that keeps it sound (design §5, handoff §3).
//!
//! One framed record per entry: object id, container (type + slot), offset, length, and the
//! container record's own frame checksum. Framing reuses the same shape as `container.rs` (magic,
//! version, body_len, checksum, body) with its own magic and a fixed-width body -- five fixed-size
//! fields, no length-prefixing needed inside the body itself.
//!
//! **The write protocol is the part no framing can enforce, and is stated here at the call site, not
//! only in the design doc**: [`write_object_to_container`] appends the object record to its
//! container and makes it durable *first*; only then does it append the index entry. A crash between
//! the two leaves an object present and unindexed -- recovered by [`rebuild_index_from_containers`],
//! the safe direction. The reverse order would let a reader see a valid, checksummed index entry
//! pointing at bytes that are not there, which must never happen.
//!
//! **Read validation (design §12/§10.3's ruling)**: an ordinary lookup trusts the index for
//! location -- one index read, one seek into the container, no container scan. The bytes found are
//! then validated by recomputing `ObjectEnvelope::object_id()` from the decoded content, which is
//! free of extra I/O since the object must be decoded anyway; a mismatch is a reported defect, never
//! a silent fallback to scanning. `verify` (a different call path, not this one) is what does the
//! full container scan.

// RFC 102 Stage 3: complete and independently tested but not yet wired into any production caller --
// `object_store.rs`'s rewrite onto containers+index is the next step in this same increment. Remove
// once that wiring lands.
#![allow(dead_code)]

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectId, ObjectType};

use crate::byte_cursor::ByteCursor;
use crate::container::{self, ContainerRecordStatus, container_magic};
use crate::file_codec::push_u16;
use crate::frame_resync::resync_to_next_magic;
use crate::fsutil::{append_file_required, len_to_u64, read_file_if_exists};
use crate::layout::{ContainerSlot, RepositoryLayout, persisted_object_types};
use prikk_hash::sha256;

const INDEX_MAGIC: &[u8; 8] = b"PIDXENT1";
const INDEX_VERSION: u16 = 1;
const INDEX_HEADER_LEN: usize = 8 + 2 + 8 + 32;
/// object_id(32) + object_type code(2) + slot(1) + offset(8) + length(8) + container_checksum(32).
const INDEX_BODY_LEN: usize = 32 + 2 + 1 + 8 + 8 + 32;

/// One index entry: where one object's container record lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IndexEntry {
    pub(crate) object_id: ObjectId,
    pub(crate) object_type: ObjectType,
    pub(crate) slot: ContainerSlot,
    pub(crate) offset: u64,
    pub(crate) length: u64,
    pub(crate) container_checksum: [u8; 32],
}

/// Outcome of attempting to decode one index record frame. Mirrors `container::ContainerRecordStatus`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IndexRecordStatus {
    Evaluated,
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexRecordOutcome {
    pub(crate) offset: usize,
    pub(crate) status: IndexRecordStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexReplay {
    pub(crate) entries: Vec<IndexEntry>,
    pub(crate) trailing_partial_bytes: usize,
    pub(crate) record_outcomes: Vec<IndexRecordOutcome>,
}

impl IndexReplay {
    #[must_use]
    pub(crate) fn has_item_failure(&self) -> bool {
        self.record_outcomes
            .iter()
            .any(|outcome| matches!(outcome.status, IndexRecordStatus::Failed { .. }))
    }
}

fn slot_code(slot: ContainerSlot) -> u8 {
    match slot {
        ContainerSlot::A => 0,
        ContainerSlot::B => 1,
    }
}

fn slot_from_code(code: u8) -> Result<ContainerSlot> {
    match code {
        0 => Ok(ContainerSlot::A),
        1 => Ok(ContainerSlot::B),
        other => Err(PrikkError::MalformedData(format!(
            "unrecognized container slot code {other}"
        ))),
    }
}

fn encode_entry_body(entry: &IndexEntry) -> Vec<u8> {
    let mut body = Vec::with_capacity(INDEX_BODY_LEN);
    body.extend_from_slice(entry.object_id.as_bytes());
    push_u16(&mut body, entry.object_type.code());
    body.push(slot_code(entry.slot));
    body.extend_from_slice(&entry.offset.to_be_bytes());
    body.extend_from_slice(&entry.length.to_be_bytes());
    body.extend_from_slice(&entry.container_checksum);
    body
}

fn decode_entry_body(body: &[u8]) -> Result<IndexEntry> {
    let mut cursor = ByteCursor::new(body);
    let object_id = ObjectId::from_bytes(cursor.read_array::<32>()?);
    let object_type = ObjectType::from_code(cursor.read_u16()?)?;
    let slot = slot_from_code(cursor.read_array::<1>()?[0])?;
    let offset = cursor.read_u64()?;
    let length = cursor.read_u64()?;
    let container_checksum = cursor.read_array::<32>()?;
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData(
            "trailing bytes in index entry body".to_string(),
        ));
    }
    Ok(IndexEntry {
        object_id,
        object_type,
        slot,
        offset,
        length,
        container_checksum,
    })
}

fn encode_index_record(entry: &IndexEntry) -> Result<Vec<u8>> {
    let body = encode_entry_body(entry);
    let body_len = len_to_u64(body.len())?;
    let checksum = index_record_checksum(body_len, &body);
    let mut out = Vec::with_capacity(INDEX_HEADER_LEN + body.len());
    out.extend_from_slice(INDEX_MAGIC);
    push_u16(&mut out, INDEX_VERSION);
    out.extend_from_slice(&body_len.to_be_bytes());
    out.extend_from_slice(&checksum);
    out.extend_from_slice(&body);
    Ok(out)
}

fn index_record_checksum(body_len: u64, body: &[u8]) -> [u8; 32] {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(INDEX_MAGIC);
    preimage.extend_from_slice(&INDEX_VERSION.to_be_bytes());
    preimage.extend_from_slice(&body_len.to_be_bytes());
    preimage.extend_from_slice(body);
    sha256(&preimage)
}

struct IndexHeader {
    body_len: u64,
    checksum: [u8; 32],
}

fn parse_index_header(header: &[u8]) -> Result<IndexHeader> {
    let mut cursor = ByteCursor::new(header);
    let magic = cursor.read_array::<8>()?;
    if &magic != INDEX_MAGIC {
        return Err(PrikkError::MalformedData(
            "invalid index record magic".to_string(),
        ));
    }
    let version = cursor.read_u16()?;
    if version != INDEX_VERSION {
        return Err(PrikkError::UnsupportedFormatVersion(u32::from(version)));
    }
    let body_len = cursor.read_u64()?;
    let checksum = cursor.read_array::<32>()?;
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData(
            "trailing bytes in index header".to_string(),
        ));
    }
    Ok(IndexHeader { body_len, checksum })
}

enum FrameAttempt {
    Record {
        entry: IndexEntry,
        next_offset: usize,
    },
    TrailingPartial {
        remaining: usize,
    },
    Invalid {
        message: String,
    },
}

fn parse_frame_at(bytes: &[u8], offset: usize) -> FrameAttempt {
    let remaining = bytes.len().saturating_sub(offset);
    if remaining < INDEX_HEADER_LEN {
        return FrameAttempt::TrailingPartial { remaining };
    }
    let header_end = offset + INDEX_HEADER_LEN;
    let Some(header) = bytes.get(offset..header_end) else {
        return FrameAttempt::TrailingPartial { remaining };
    };
    let header_values = match parse_index_header(header) {
        Ok(values) => values,
        Err(err) => {
            return FrameAttempt::Invalid {
                message: err.to_string(),
            };
        }
    };
    let Ok(body_len) = usize::try_from(header_values.body_len) else {
        return FrameAttempt::Invalid {
            message: "index body length does not fit usize".to_string(),
        };
    };
    let Some(body_end) = header_end.checked_add(body_len) else {
        return FrameAttempt::Invalid {
            message: "index body end overflow".to_string(),
        };
    };
    let Some(body) = bytes.get(header_end..body_end) else {
        return FrameAttempt::TrailingPartial { remaining };
    };
    let expected = index_record_checksum(header_values.body_len, body);
    if expected != header_values.checksum {
        return FrameAttempt::Invalid {
            message: format!("index checksum mismatch at byte offset {offset}"),
        };
    }
    match decode_entry_body(body) {
        Ok(entry) => FrameAttempt::Record {
            entry,
            next_offset: body_end,
        },
        Err(err) => FrameAttempt::Invalid {
            message: err.to_string(),
        },
    }
}

/// Isolate-and-continue reading, matching the WAL/ref-log/container read path exactly (RFC 102
/// Stage 2's reader, reused via `frame_resync::resync_to_next_magic`, not re-derived): a damaged
/// index entry is named at its own offset and the scan continues past it.
pub(crate) fn decode_index_records(bytes: &[u8]) -> Result<IndexReplay> {
    let mut entries = Vec::new();
    let mut record_outcomes = Vec::new();
    let mut offset = 0_usize;
    loop {
        match parse_frame_at(bytes, offset) {
            FrameAttempt::Record { entry, next_offset } => {
                record_outcomes.push(IndexRecordOutcome {
                    offset,
                    status: IndexRecordStatus::Evaluated,
                });
                entries.push(entry);
                offset = next_offset;
            }
            FrameAttempt::TrailingPartial { remaining } => {
                return Ok(IndexReplay {
                    entries,
                    trailing_partial_bytes: remaining,
                    record_outcomes,
                });
            }
            FrameAttempt::Invalid { message } => {
                record_outcomes.push(IndexRecordOutcome {
                    offset,
                    status: IndexRecordStatus::Failed { message },
                });
                match resync_to_next_magic(bytes, offset + 1, INDEX_MAGIC.as_slice()) {
                    Some(next) => offset = next,
                    None => {
                        return Ok(IndexReplay {
                            entries,
                            trailing_partial_bytes: 0,
                            record_outcomes,
                        });
                    }
                }
            }
        }
    }
}

/// Read and replay the on-disk index, off the durability path (design §4) -- a missing file replays
/// as empty, the same reader-equivalence rule Stage 1 established for the WAL.
pub(crate) fn replay_index(layout: &RepositoryLayout) -> Result<IndexReplay> {
    let relative = layout.repository_relative(&layout.container_index_path())?;
    let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &relative)? else {
        return Ok(IndexReplay {
            entries: Vec::new(),
            trailing_partial_bytes: 0,
            record_outcomes: Vec::new(),
        });
    };
    decode_index_records(&bytes)
}

/// Look up one object's container location. Trusts the index for location (design §12/§10.3): one
/// index read, then a linear search of its (already-decoded) entries -- no container scan. Refuses
/// if the index itself has a damaged entry, rather than silently searching around it: an index this
/// read depends on being sound is not the same question as which entries in it are damaged.
pub(crate) fn lookup_object_location(
    layout: &RepositoryLayout,
    object_id: ObjectId,
) -> Result<Option<IndexEntry>> {
    let replay = replay_index(layout)?;
    if replay.has_item_failure() {
        return Err(PrikkError::Integrity(
            "object index has a damaged entry; run doctor before reading".to_string(),
        ));
    }
    Ok(replay
        .entries
        .into_iter()
        .rev()
        .find(|entry| entry.object_id == object_id))
}

/// The write protocol (design §5, handoff §3): append the object record to its container and make
/// it durable, **then and only then** append the index entry. Never `atomic_replace` -- both appends
/// go through `append_file_required` (`durable_append`), matching every other durability-bearing
/// write in this codebase.
pub(crate) fn write_object_to_container(
    layout: &RepositoryLayout,
    object_type: ObjectType,
    envelope: &ObjectEnvelope,
) -> Result<ObjectId> {
    let object_id = envelope.object_id();
    if let Some(existing) = lookup_object_location(layout, object_id)? {
        if existing.object_type != object_type {
            return Err(PrikkError::Integrity(format!(
                "existing index entry for {object_id} has type {}, expected {object_type}",
                existing.object_type
            )));
        }
        return Ok(object_id);
    }

    let record_bytes = container::encode_container_record(object_type, envelope)?;
    let container_relative =
        layout.repository_relative(&layout.container_slot_path(object_type, ContainerSlot::A))?;
    let existing_len = read_file_if_exists(layout.repository_mutation_root(), &container_relative)?
        .map_or(0, |bytes| bytes.len());
    let offset = len_to_u64(existing_len)?;
    let length = len_to_u64(record_bytes.len())?;
    let container_checksum = frame_checksum(object_type, &record_bytes)?;

    // Step 1: append the object record to its container. Must be durable before step 2 -- a crash
    // here leaves nothing indexed yet, which is not a problem: nothing durable claims this object
    // exists, so there is nothing for a reader to find prematurely.
    append_file_required(
        layout.repository_mutation_root(),
        &container_relative,
        &record_bytes,
    )?;

    // Step 2: only now append the index entry. A crash between step 1 and here leaves the object
    // present-but-unindexed -- the safe direction (design §5): `rebuild_index_from_containers`
    // recovers it. The reverse order is never used, anywhere in this module.
    let entry = IndexEntry {
        object_id,
        object_type,
        slot: ContainerSlot::A,
        offset,
        length,
        container_checksum,
    };
    let index_bytes = encode_index_record(&entry)?;
    let index_relative = layout.repository_relative(&layout.container_index_path())?;
    append_file_required(
        layout.repository_mutation_root(),
        &index_relative,
        &index_bytes,
    )?;

    Ok(object_id)
}

/// Extract a just-encoded container record's own frame checksum (the 32 bytes immediately following
/// magic + version + body_len), without re-parsing the whole frame -- `record_bytes` was built by
/// `container::encode_container_record` moments ago, so its shape is already known.
fn frame_checksum(object_type: ObjectType, record_bytes: &[u8]) -> Result<[u8; 32]> {
    let magic = container_magic(object_type)?;
    let checksum_start = magic.len() + 2 + 8;
    let checksum_end = checksum_start + 32;
    let checksum_bytes = record_bytes
        .get(checksum_start..checksum_end)
        .ok_or_else(|| {
            PrikkError::Integrity("just-encoded container record is too short".to_string())
        })?;
    let mut checksum = [0_u8; 32];
    checksum.copy_from_slice(checksum_bytes);
    Ok(checksum)
}

/// Rebuild the index by scanning every container. Not a new operation (design §4): the same
/// per-record content-hash check `verify` already performs, iterated over a scan of every
/// `persisted_object_types()` container's slot A (the only slot Stage 3 ever writes -- rebuilding
/// slot B is Stage 6's concern once compaction can produce one). Returns every sound record found,
/// skipping damaged ones (they are `verify`'s job to report, not rebuild's job to paper over) --
/// callers that need to know about damage should inspect the container replay themselves, not rely
/// on this function's silence about it.
pub(crate) fn rebuild_index_from_containers(layout: &RepositoryLayout) -> Result<Vec<IndexEntry>> {
    let mut entries = Vec::new();
    for object_type in persisted_object_types() {
        let relative = layout
            .repository_relative(&layout.container_slot_path(object_type, ContainerSlot::A))?;
        let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &relative)? else {
            continue;
        };
        let replay = container::decode_container_records(object_type, &bytes)?;
        // `records` holds only sound frames, in the same order `record_outcomes` visits its
        // `Evaluated` entries -- both are built in lockstep by `decode_container_records`, so
        // advancing this iterator once per `Evaluated` outcome pairs each with its own envelope.
        let mut records = replay.records.iter();
        for outcome in &replay.record_outcomes {
            let ContainerRecordStatus::Evaluated {
                frame_len,
                checksum,
            } = &outcome.status
            else {
                continue;
            };
            let Some(record) = records.next() else {
                return Err(PrikkError::Integrity(
                    "container replay outcome/record count mismatch".to_string(),
                ));
            };
            entries.push(IndexEntry {
                object_id: record.envelope.object_id(),
                object_type,
                slot: ContainerSlot::A,
                offset: len_to_u64(outcome.offset)?,
                length: len_to_u64(*frame_len)?,
                container_checksum: *checksum,
            });
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests;
