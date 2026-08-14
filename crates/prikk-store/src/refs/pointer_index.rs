//! Ref-pointer index: append-only, rebuildable, off the durability path (RFC 102 Stage 4, Step 0
//! §13.4, ruled in design-v1.md §13.4). Same *pattern* as the object index (`index.rs`) -- append-only,
//! last-entry-wins lookup, rebuildable by scan -- but its own type and its own container: the ruling
//! is explicit that `index.rs`'s already-shipped object-index schema is never widened for this.
//!
//! One framed record per entry: `ref_name_key` (`sha256(ref_name)`, `layout::ref_name_key_bytes` --
//! the ruling's own point: this key already exists, naming every ref pointer/log file on disk today,
//! and did not need inventing), the human-readable `ref_name` itself, and the RefState id it names.
//! **`ref_name` is carried here even though `ref_name_key` alone would suffice for lookup** --
//! preserving `refs/pointer.rs`'s own existing coherence check (`read_one_pointer`'s `payload.
//! ref_name != pointer.ref_name`, `refs/verify/scan.rs`): the pointer's own claimed name must agree
//! with the RefState object's claimed name, an independent cross-check this entry would silently
//! lose if it carried only the hash. **No `update_seq` field** -- it would duplicate the RefState
//! object's own `RefStatePayload.update_seq`, which every consumer already cross-validates against
//! (`refs/verify.rs`'s `PointerState.payload`), so a second, independently-writable copy here would
//! only be a place for the two to silently disagree, not a fact anything reads.
//!
//! **This is the crash-safety-critical half of publication.** A single durable append here *is* the
//! publish (Step 0 §13.3: "an append-only record has no candidate value to stage -- the append is the
//! publish"), appended **before** the corresponding ref-log container record (`refs/container.rs`) --
//! unchanged from today's pointer-first ordering (Step 0's own "must not change" list), just backed by
//! a container append instead of a candidate-write-then-promote file dance. `refs/tmp/`'s candidate
//! mechanism has no equivalent here because there is nothing left for it to stage.

use prikk_error::{PrikkError, Result};
use prikk_hash::sha256;
use prikk_object::ObjectId;

use crate::byte_cursor::ByteCursor;
use crate::file_codec::{push_bytes_u64, push_u16};
use crate::frame_resync::resync_to_next_magic;
use crate::fsutil::{append_file_required, len_to_u64, read_file_if_exists};
use crate::layout::RepositoryLayout;

const POINTER_INDEX_MAGIC: &[u8; 8] = b"PREFPTI1";
const POINTER_INDEX_VERSION: u16 = 1;
const POINTER_INDEX_HEADER_LEN: usize = 8 + 2 + 8 + 32;

/// One ref-pointer-index entry: the published RefState id for one `ref_name_key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PointerIndexEntry {
    pub(crate) ref_name_key: [u8; 32],
    pub(crate) ref_name: String,
    pub(crate) ref_state_id: ObjectId,
}

/// Outcome of attempting to decode one pointer-index record frame. Mirrors `index::IndexRecordStatus`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PointerIndexRecordStatus {
    Evaluated,
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PointerIndexRecordOutcome {
    pub(crate) offset: usize,
    pub(crate) status: PointerIndexRecordStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PointerIndexReplay {
    pub(crate) entries: Vec<PointerIndexEntry>,
    pub(crate) trailing_partial_bytes: usize,
    pub(crate) record_outcomes: Vec<PointerIndexRecordOutcome>,
}

impl PointerIndexReplay {
    #[must_use]
    pub(crate) fn has_item_failure(&self) -> bool {
        self.record_outcomes
            .iter()
            .any(|outcome| matches!(outcome.status, PointerIndexRecordStatus::Failed { .. }))
    }
}

fn encode_entry_body(entry: &PointerIndexEntry) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    body.extend_from_slice(&entry.ref_name_key);
    push_bytes_u64(&mut body, entry.ref_name.as_bytes())?;
    body.extend_from_slice(entry.ref_state_id.as_bytes());
    Ok(body)
}

fn decode_entry_body(body: &[u8]) -> Result<PointerIndexEntry> {
    let mut cursor = ByteCursor::new(body);
    let ref_name_key = cursor.read_array::<32>()?;
    let ref_name_bytes = cursor.read_bytes_u64()?;
    let ref_name = String::from_utf8(ref_name_bytes)
        .map_err(|err| PrikkError::MalformedData(format!("invalid ref name utf-8: {err}")))?;
    let ref_state_id = ObjectId::from_bytes(cursor.read_array::<32>()?);
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData(
            "trailing bytes in pointer index entry body".to_string(),
        ));
    }
    Ok(PointerIndexEntry {
        ref_name_key,
        ref_name,
        ref_state_id,
    })
}

pub(crate) fn encode_pointer_index_record(entry: &PointerIndexEntry) -> Result<Vec<u8>> {
    let body = encode_entry_body(entry)?;
    let body_len = len_to_u64(body.len())?;
    let checksum = record_checksum(body_len, &body);
    let mut out = Vec::with_capacity(POINTER_INDEX_HEADER_LEN + body.len());
    out.extend_from_slice(POINTER_INDEX_MAGIC);
    push_u16(&mut out, POINTER_INDEX_VERSION);
    out.extend_from_slice(&body_len.to_be_bytes());
    out.extend_from_slice(&checksum);
    out.extend_from_slice(&body);
    Ok(out)
}

fn record_checksum(body_len: u64, body: &[u8]) -> [u8; 32] {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(POINTER_INDEX_MAGIC);
    preimage.extend_from_slice(&POINTER_INDEX_VERSION.to_be_bytes());
    preimage.extend_from_slice(&body_len.to_be_bytes());
    preimage.extend_from_slice(body);
    sha256(&preimage)
}

struct PointerIndexHeader {
    body_len: u64,
    checksum: [u8; 32],
}

fn parse_header(header: &[u8]) -> Result<PointerIndexHeader> {
    let mut cursor = ByteCursor::new(header);
    let magic = cursor.read_array::<8>()?;
    if &magic != POINTER_INDEX_MAGIC {
        return Err(PrikkError::MalformedData(
            "invalid pointer index record magic".to_string(),
        ));
    }
    let version = cursor.read_u16()?;
    if version != POINTER_INDEX_VERSION {
        return Err(PrikkError::UnsupportedFormatVersion(u32::from(version)));
    }
    let body_len = cursor.read_u64()?;
    let checksum = cursor.read_array::<32>()?;
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData(
            "trailing bytes in pointer index header".to_string(),
        ));
    }
    Ok(PointerIndexHeader { body_len, checksum })
}

enum FrameAttempt {
    Record {
        entry: PointerIndexEntry,
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
    if remaining < POINTER_INDEX_HEADER_LEN {
        return FrameAttempt::TrailingPartial { remaining };
    }
    let header_end = offset + POINTER_INDEX_HEADER_LEN;
    let Some(header) = bytes.get(offset..header_end) else {
        return FrameAttempt::TrailingPartial { remaining };
    };
    let header_values = match parse_header(header) {
        Ok(values) => values,
        Err(err) => {
            return FrameAttempt::Invalid {
                message: err.to_string(),
            };
        }
    };
    let Ok(body_len) = usize::try_from(header_values.body_len) else {
        return FrameAttempt::Invalid {
            message: "pointer index body length does not fit usize".to_string(),
        };
    };
    let Some(body_end) = header_end.checked_add(body_len) else {
        return FrameAttempt::Invalid {
            message: "pointer index body end overflow".to_string(),
        };
    };
    let Some(body) = bytes.get(header_end..body_end) else {
        return FrameAttempt::TrailingPartial { remaining };
    };
    let expected = record_checksum(header_values.body_len, body);
    if expected != header_values.checksum {
        return FrameAttempt::Invalid {
            message: format!("pointer index checksum mismatch at byte offset {offset}"),
        };
    }
    match decode_entry_body(body) {
        Ok(entry) => FrameAttempt::Record { entry, next_offset: body_end },
        Err(err) => FrameAttempt::Invalid {
            message: err.to_string(),
        },
    }
}

/// Isolate-and-continue reading, matching every other reader in the codebase (`frame_resync::
/// resync_to_next_magic`, not re-derived): a damaged pointer-index entry is named at its own offset
/// and the scan continues past it.
pub(crate) fn decode_pointer_index_records(bytes: &[u8]) -> Result<PointerIndexReplay> {
    let mut entries = Vec::new();
    let mut record_outcomes = Vec::new();
    let mut offset = 0_usize;
    loop {
        match parse_frame_at(bytes, offset) {
            FrameAttempt::Record { entry, next_offset } => {
                record_outcomes.push(PointerIndexRecordOutcome {
                    offset,
                    status: PointerIndexRecordStatus::Evaluated,
                });
                entries.push(entry);
                offset = next_offset;
            }
            FrameAttempt::TrailingPartial { remaining } => {
                return Ok(PointerIndexReplay {
                    entries,
                    trailing_partial_bytes: remaining,
                    record_outcomes,
                });
            }
            FrameAttempt::Invalid { message } => {
                record_outcomes.push(PointerIndexRecordOutcome {
                    offset,
                    status: PointerIndexRecordStatus::Failed { message },
                });
                match resync_to_next_magic(bytes, offset + 1, POINTER_INDEX_MAGIC.as_slice()) {
                    Some(next) => offset = next,
                    None => {
                        return Ok(PointerIndexReplay {
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

/// Read and replay the on-disk pointer index, off the durability path -- a missing file replays as
/// empty, the same reader-equivalence rule Stage 1 established for the WAL.
pub(crate) fn replay_pointer_index(layout: &RepositoryLayout) -> Result<PointerIndexReplay> {
    let relative = layout.repository_relative(&layout.ref_pointer_index_path())?;
    let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &relative)? else {
        return Ok(PointerIndexReplay {
            entries: Vec::new(),
            trailing_partial_bytes: 0,
            record_outcomes: Vec::new(),
        });
    };
    decode_pointer_index_records(&bytes)
}

/// Look up one ref's current published pointer: the last entry matching `ref_name_key`, matching
/// `index::lookup_object_location`'s own "last entry wins" reverse search exactly. Refuses if the
/// index itself has a damaged entry, rather than silently searching around it.
pub(crate) fn lookup_ref_pointer(
    layout: &RepositoryLayout,
    ref_name_key: [u8; 32],
) -> Result<Option<PointerIndexEntry>> {
    let replay = replay_pointer_index(layout)?;
    if replay.has_item_failure() {
        return Err(PrikkError::Integrity(
            "ref pointer index has a damaged entry; run doctor before reading".to_string(),
        ));
    }
    Ok(replay
        .entries
        .into_iter()
        .rev()
        .find(|entry| entry.ref_name_key == ref_name_key))
}

/// Test-only convenience matching the retired `refs/pointer.rs::write_ref_pointer_candidate`'s own
/// 3-argument call shape exactly, for fixtures that need to plant a specific pointer state directly
/// without going through a real publish. Computes `ref_name_key` itself.
#[cfg(test)]
pub(crate) fn write_ref_pointer_candidate_for_test(
    layout: &RepositoryLayout,
    ref_name: &str,
    ref_state_id: ObjectId,
) -> Result<()> {
    append_ref_pointer_entry(
        layout,
        &PointerIndexEntry {
            ref_name_key: crate::layout::ref_name_key_bytes(ref_name),
            ref_name: ref_name.to_string(),
            ref_state_id,
        },
    )
}

/// Durably append one new pointer entry -- the publish moment itself (see module doc). Never checks
/// for an existing entry first: unlike the object index's same-id-different-bytes idempotency guard
/// (`write_object_to_container`), a duplicate pointer entry is not wasteful the way a duplicate
/// signed object would be (this record is small, though not fixed-width -- `ref_name` varies), and
/// "last entry wins" already makes a benign duplicate harmless -- so the caller's own CAS check
/// (`expected_previous_ref_state_id` against the current lookup) is what refuses a genuine conflict,
/// not this function.
pub(crate) fn append_ref_pointer_entry(
    layout: &RepositoryLayout,
    entry: &PointerIndexEntry,
) -> Result<()> {
    let record = encode_pointer_index_record(entry)?;
    let relative = layout.repository_relative(&layout.ref_pointer_index_path())?;
    append_file_required(layout.repository_mutation_root(), &relative, &record)
}

/// Splice out every entry matching `ref_name_key` -- the container-native way to simulate "this
/// ref's pointer is genuinely missing" under an append-only model, mirroring `index.rs`'s own
/// `remove_index_entry_for_test` (RFC 102 Stage 3) exactly in spirit. Entries here are
/// variable-width (`ref_name`), so each record's own span is derived from consecutive outcome
/// offsets rather than a fixed frame length.
#[cfg(test)]
pub(crate) fn remove_pointer_entries_for_test(
    layout: &RepositoryLayout,
    ref_name_key: [u8; 32],
) -> Result<()> {
    let path = layout.ref_pointer_index_path();
    let bytes = std::fs::read(&path)?;
    let replay = decode_pointer_index_records(&bytes)?;
    let mut entries = replay.entries.iter();
    let mut retained = Vec::new();
    for (index, outcome) in replay.record_outcomes.iter().enumerate() {
        let end = replay
            .record_outcomes
            .get(index + 1)
            .map_or(bytes.len() - replay.trailing_partial_bytes, |next| next.offset);
        let span = bytes.get(outcome.offset..end).unwrap_or_default();
        match &outcome.status {
            PointerIndexRecordStatus::Evaluated => {
                let Some(entry) = entries.next() else {
                    return Err(PrikkError::Integrity(
                        "pointer index replay outcome/entry count mismatch".to_string(),
                    ));
                };
                if entry.ref_name_key == ref_name_key {
                    continue;
                }
            }
            PointerIndexRecordStatus::Failed { .. } => {}
        }
        retained.extend_from_slice(span);
    }
    retained.extend_from_slice(
        bytes
            .get(bytes.len() - replay.trailing_partial_bytes..)
            .unwrap_or_default(),
    );
    std::fs::write(&path, retained)?;
    Ok(())
}

#[cfg(test)]
mod tests;
