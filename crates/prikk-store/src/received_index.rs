//! Received-ref index framing: the encode/decode/corruption-isolation half of `received.rs`'s
//! container migration (RFC 102 Stage 5, design-v1.md §14, Step 0 item 2's ruling that `received.rs`
//! belongs on the refs container+pointer-index pattern -- the same unbounded, minted-after-`init`
//! per-name shape that forced Stage 4's ref pointer index, §13.2's argument applied to a second
//! subsystem). Mirrors `refs/pointer_index.rs` deliberately closely: same magic-framed,
//! checksum-verified, resync-on-corruption record shape, and the identical entry fields --
//! `received.rs`'s own doc already states its semantics are "no CAS and no merge... this is what I
//! have now," which is exactly last-entry-wins, the same publish model the ref pointer index already
//! proved sound.
//!
//! **Deliberately incomplete.** This module holds only the pure codec -- encode, decode, frame
//! parsing, resync-on-corruption -- with no dependency on `RepositoryLayout` or any container path.
//! The read/append functions that would give this a place to live on disk, the `init`-time name
//! allocation, and `require_current_format`'s guard all wait on a still-open question: whether adding
//! this container name (and the trust key container alongside it) requires a `RepositoryFormat` bump,
//! the same way Stage 3 and Stage 4 each bumped format for an equivalent on-disk shape change. See
//! `.git-exclude/review-request/prikk-rfc102-stage5-format-bump-question-v1.md` and its ruling. This
//! file is safe to land independent of that answer; nothing below touches a real repository.

// Deliberately incomplete (see module doc): nothing outside this module's own tests calls any of
// this yet, since the I/O layer that would give it a real caller waits on the format-bump ruling.
// Temporary -- removed the moment `replay_received_index`/`append_received_index_entry`/`lookup_
// received_pointer` land and give this a production caller.
#![allow(dead_code)]

use prikk_error::{PrikkError, Result};
use prikk_hash::sha256;
use prikk_object::ObjectId;

use crate::byte_cursor::ByteCursor;
use crate::file_codec::{push_bytes_u64, push_u16};
use crate::frame_resync::resync_to_next_magic;
use crate::fsutil::len_to_u64;

const RECEIVED_INDEX_MAGIC: &[u8; 8] = b"PRECVIX1";
const RECEIVED_INDEX_VERSION: u16 = 1;
const RECEIVED_INDEX_HEADER_LEN: usize = 8 + 2 + 8 + 32;

/// One received-ref-index entry: the last-imported `RefState` id for one `ref_name_key`
/// (`layout::ref_name_key_bytes`, the same fixed-width key the ref pointer index uses -- `received.
/// rs`'s existing per-file naming already keys on its hex form, `ref_name_storage_key`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReceivedIndexEntry {
    pub(crate) ref_name_key: [u8; 32],
    pub(crate) ref_name: String,
    pub(crate) ref_state_id: ObjectId,
}

/// Outcome of attempting to decode one received-index record frame. Mirrors `PointerIndexRecordStatus`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReceivedIndexRecordStatus {
    Evaluated,
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReceivedIndexRecordOutcome {
    pub(crate) offset: usize,
    pub(crate) status: ReceivedIndexRecordStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReceivedIndexReplay {
    pub(crate) entries: Vec<ReceivedIndexEntry>,
    pub(crate) trailing_partial_bytes: usize,
    pub(crate) record_outcomes: Vec<ReceivedIndexRecordOutcome>,
}

impl ReceivedIndexReplay {
    #[must_use]
    pub(crate) fn has_item_failure(&self) -> bool {
        self.record_outcomes
            .iter()
            .any(|outcome| matches!(outcome.status, ReceivedIndexRecordStatus::Failed { .. }))
    }
}

fn encode_entry_body(entry: &ReceivedIndexEntry) -> Result<Vec<u8>> {
    let mut body = Vec::new();
    body.extend_from_slice(&entry.ref_name_key);
    push_bytes_u64(&mut body, entry.ref_name.as_bytes())?;
    body.extend_from_slice(entry.ref_state_id.as_bytes());
    Ok(body)
}

fn decode_entry_body(body: &[u8]) -> Result<ReceivedIndexEntry> {
    let mut cursor = ByteCursor::new(body);
    let ref_name_key = cursor.read_array::<32>()?;
    let ref_name_bytes = cursor.read_bytes_u64()?;
    let ref_name = String::from_utf8(ref_name_bytes)
        .map_err(|err| PrikkError::MalformedData(format!("invalid ref name utf-8: {err}")))?;
    let ref_state_id = ObjectId::from_bytes(cursor.read_array::<32>()?);
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData(
            "trailing bytes in received index entry body".to_string(),
        ));
    }
    Ok(ReceivedIndexEntry {
        ref_name_key,
        ref_name,
        ref_state_id,
    })
}

pub(crate) fn encode_received_index_record(entry: &ReceivedIndexEntry) -> Result<Vec<u8>> {
    let body = encode_entry_body(entry)?;
    let body_len = len_to_u64(body.len())?;
    let checksum = record_checksum(body_len, &body);
    let mut out = Vec::with_capacity(RECEIVED_INDEX_HEADER_LEN + body.len());
    out.extend_from_slice(RECEIVED_INDEX_MAGIC);
    push_u16(&mut out, RECEIVED_INDEX_VERSION);
    out.extend_from_slice(&body_len.to_be_bytes());
    out.extend_from_slice(&checksum);
    out.extend_from_slice(&body);
    Ok(out)
}

fn record_checksum(body_len: u64, body: &[u8]) -> [u8; 32] {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(RECEIVED_INDEX_MAGIC);
    preimage.extend_from_slice(&RECEIVED_INDEX_VERSION.to_be_bytes());
    preimage.extend_from_slice(&body_len.to_be_bytes());
    preimage.extend_from_slice(body);
    sha256(&preimage)
}

struct ReceivedIndexHeader {
    body_len: u64,
    checksum: [u8; 32],
}

fn parse_header(header: &[u8]) -> Result<ReceivedIndexHeader> {
    let mut cursor = ByteCursor::new(header);
    let magic = cursor.read_array::<8>()?;
    if &magic != RECEIVED_INDEX_MAGIC {
        return Err(PrikkError::MalformedData(
            "invalid received index record magic".to_string(),
        ));
    }
    let version = cursor.read_u16()?;
    if version != RECEIVED_INDEX_VERSION {
        return Err(PrikkError::UnsupportedFormatVersion(u32::from(version)));
    }
    let body_len = cursor.read_u64()?;
    let checksum = cursor.read_array::<32>()?;
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData(
            "trailing bytes in received index header".to_string(),
        ));
    }
    Ok(ReceivedIndexHeader { body_len, checksum })
}

enum FrameAttempt {
    Record {
        entry: ReceivedIndexEntry,
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
    if remaining < RECEIVED_INDEX_HEADER_LEN {
        return FrameAttempt::TrailingPartial { remaining };
    }
    let header_end = offset + RECEIVED_INDEX_HEADER_LEN;
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
            message: "received index body length does not fit usize".to_string(),
        };
    };
    let Some(body_end) = header_end.checked_add(body_len) else {
        return FrameAttempt::Invalid {
            message: "received index body end overflow".to_string(),
        };
    };
    let Some(body) = bytes.get(header_end..body_end) else {
        return FrameAttempt::TrailingPartial { remaining };
    };
    let expected = record_checksum(header_values.body_len, body);
    if expected != header_values.checksum {
        return FrameAttempt::Invalid {
            message: format!("received index checksum mismatch at byte offset {offset}"),
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

/// Isolate-and-continue reading, matching `pointer_index.rs::decode_pointer_index_records` and
/// `frame_resync::resync_to_next_magic` exactly: a damaged received-index entry is named at its own
/// offset and the scan continues past it.
pub(crate) fn decode_received_index_records(bytes: &[u8]) -> Result<ReceivedIndexReplay> {
    let mut entries = Vec::new();
    let mut record_outcomes = Vec::new();
    let mut offset = 0_usize;
    loop {
        match parse_frame_at(bytes, offset) {
            FrameAttempt::Record { entry, next_offset } => {
                record_outcomes.push(ReceivedIndexRecordOutcome {
                    offset,
                    status: ReceivedIndexRecordStatus::Evaluated,
                });
                entries.push(entry);
                offset = next_offset;
            }
            FrameAttempt::TrailingPartial { remaining } => {
                return Ok(ReceivedIndexReplay {
                    entries,
                    trailing_partial_bytes: remaining,
                    record_outcomes,
                });
            }
            FrameAttempt::Invalid { message } => {
                record_outcomes.push(ReceivedIndexRecordOutcome {
                    offset,
                    status: ReceivedIndexRecordStatus::Failed { message },
                });
                match resync_to_next_magic(bytes, offset + 1, RECEIVED_INDEX_MAGIC.as_slice()) {
                    Some(next) => offset = next,
                    None => {
                        return Ok(ReceivedIndexReplay {
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

#[cfg(test)]
mod tests;
