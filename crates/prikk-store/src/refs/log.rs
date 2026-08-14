//! Ref-update log codec and replay.

use prikk_error::{PrikkError, Result};
use prikk_hash::sha256;
use prikk_object::{ObjectEnvelope, ObjectType, RefUpdatePayload};

use crate::byte_cursor::ByteCursor;
use crate::file_codec::{decode_envelope_file, encode_envelope_file, push_u16, push_u64};
use crate::frame_resync::resync_to_next_magic;
use crate::fsutil::{
    append_file_required, ensure_directory_required, len_to_u64, read_file_if_exists,
    truncate_existing_file_required,
};
use crate::layout::RepositoryLayout;
use crate::refs::require_signed_type;

const REF_LOG_MAGIC: &[u8; 8] = b"PREFLOG1";
const REF_LOG_VERSION: u16 = 1;
const REF_LOG_HEADER_LEN: usize = 8 + 2 + 8 + 32;

/// One decoded ref-log record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefLogRecord {
    /// Exact signed RefUpdate envelope stored in the log.
    pub envelope: ObjectEnvelope,
}

/// Outcome of attempting to decode one ref-log record frame (RFC 102 Stage 2: isolate-and-continue
/// reading). Mirrors `wal::WalRecordOutcome`; see its doc for the reasoning this shares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefLogRecordStatus {
    /// The frame at this offset was read and validated successfully.
    Evaluated,
    /// The frame at this offset failed to validate (bad magic/version, checksum mismatch, or a
    /// malformed/unsigned envelope) -- resync moved past it byte-wise to find the next candidate.
    Failed {
        /// The error this frame's own validation raised.
        message: String,
    },
}

/// One attempted ref-log record frame's resolved outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefLogRecordOutcome {
    /// The byte offset within this ref log the frame attempt started at.
    pub offset: usize,
    /// How this frame's own read/validation resolved.
    pub status: RefLogRecordStatus,
}

/// Ref-log replay result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefLogReplay {
    /// Valid records read from the log, in file order -- includes records found after a damaged
    /// one (RFC 102 Stage 2), not merely a prefix up to the first failure.
    pub records: Vec<RefLogRecord>,
    /// Number of trailing bytes ignored as an incomplete final record. Zero when the log's end was
    /// reached via resync after unrecoverable corruption rather than a genuinely incomplete tail.
    pub trailing_partial_bytes: usize,
    /// One outcome per attempted frame, in scan order -- both `Evaluated` and `Failed`.
    pub record_outcomes: Vec<RefLogRecordOutcome>,
}

impl RefLogReplay {
    /// Return true when any attempted frame failed to validate.
    #[must_use]
    pub fn has_item_failure(&self) -> bool {
        self.record_outcomes
            .iter()
            .any(|outcome| matches!(outcome.status, RefLogRecordStatus::Failed { .. }))
    }
}

/// Append one signed RefUpdate envelope to the ref log and fsync it.
pub(crate) fn append_log_record(
    layout: &RepositoryLayout,
    ref_name: &str,
    envelope: &ObjectEnvelope,
) -> Result<()> {
    layout.require_current_format()?;
    crate::format::validate_object_envelope(layout.format(), envelope)?;
    validate_log_record(envelope)?;
    let path = layout.repository_relative(&layout.ref_log_path(ref_name))?;
    let Some(parent) = path.parent() else {
        return Err(PrikkError::Io(
            "ref log path has no parent directory".to_string(),
        ));
    };
    ensure_directory_required(layout.repository_mutation_root(), parent)?;
    let record = encode_log_record(envelope)?;
    let replay = replay_log(layout, ref_name)?;
    if replay.trailing_partial_bytes != 0 {
        return Err(PrikkError::Integrity(format!(
            "cannot append ref log {ref_name} after an incomplete tail"
        )));
    }
    if replay
        .records
        .last()
        .is_some_and(|last| last.envelope == *envelope)
    {
        return append_file_required(layout.repository_mutation_root(), &path, &[]);
    }
    append_file_required(layout.repository_mutation_root(), &path, &record)
}

/// Replay the inline ref-update log for a ref name.
pub(crate) fn replay_log(layout: &RepositoryLayout, ref_name: &str) -> Result<RefLogReplay> {
    let path = layout.repository_relative(&layout.ref_log_path(ref_name))?;
    let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &path)? else {
        return Ok(RefLogReplay {
            records: Vec::new(),
            trailing_partial_bytes: 0,
            record_outcomes: Vec::new(),
        });
    };
    decode_log_file_bytes(layout.format(), &bytes)
}

pub(crate) fn decode_log_file_bytes(
    format: crate::layout::RepositoryFormat,
    bytes: &[u8],
) -> Result<RefLogReplay> {
    let replay = decode_log_records(bytes)?;
    for record in &replay.records {
        crate::format::validate_read_schema(format, &record.envelope)?;
    }
    Ok(replay)
}

/// Truncate only a structurally incomplete final frame and required-sync the retained log.
pub(crate) fn truncate_incomplete_tail(layout: &RepositoryLayout, ref_name: &str) -> Result<usize> {
    let path = layout.repository_relative(&layout.ref_log_path(ref_name))?;
    let bytes = read_file_if_exists(layout.repository_mutation_root(), &path)?.unwrap_or_default();
    let replay = decode_log_records(&bytes)?;
    if replay.trailing_partial_bytes == 0 {
        return Ok(0);
    }
    let retained = bytes
        .len()
        .checked_sub(replay.trailing_partial_bytes)
        .ok_or_else(|| PrikkError::Integrity("ref-log retained length underflow".to_string()))?;
    truncate_existing_file_required(
        layout.repository_mutation_root(),
        &path,
        u64::try_from(retained)
            .map_err(|_| PrikkError::Integrity("ref-log length exceeds u64".to_string()))?,
    )?;
    Ok(replay.trailing_partial_bytes)
}

/// Return whether the incomplete suffix is an exact prefix of the expected next record.
pub(crate) fn incomplete_tail_matches(
    layout: &RepositoryLayout,
    ref_name: &str,
    expected: &ObjectEnvelope,
) -> Result<bool> {
    let path = layout.repository_relative(&layout.ref_log_path(ref_name))?;
    let bytes = read_file_if_exists(layout.repository_mutation_root(), &path)?.unwrap_or_default();
    let replay = decode_log_records(&bytes)?;
    if replay.trailing_partial_bytes == 0 {
        return Ok(false);
    }
    let retained = bytes
        .len()
        .checked_sub(replay.trailing_partial_bytes)
        .ok_or_else(|| PrikkError::Integrity("ref-log retained length underflow".to_string()))?;
    let expected_record = encode_log_record(expected)?;
    let suffix = bytes.get(retained..).ok_or_else(|| {
        PrikkError::Integrity("ref-log incomplete suffix range overflow".to_string())
    })?;
    Ok(expected_record.starts_with(suffix))
}

fn encode_log_record(envelope: &ObjectEnvelope) -> Result<Vec<u8>> {
    validate_log_record(envelope)?;
    let body = encode_envelope_file(envelope)?;
    frame_log_record(&body)
}

fn validate_log_record(envelope: &ObjectEnvelope) -> Result<()> {
    require_signed_type(envelope, ObjectType::RefUpdate)?;
    envelope.validate_strict()?;
    let update = RefUpdatePayload::decode_canonical(&envelope.canonical_payload)?;
    if envelope.schema_version == 1 && update.created_at != 0 {
        return Err(PrikkError::Integrity(
            "schema-1 RefUpdate mutation requires created_at == 0".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn encode_log_record_for_test(envelope: &ObjectEnvelope) -> Result<Vec<u8>> {
    require_signed_type(envelope, ObjectType::RefUpdate)?;
    let body = crate::file_codec::encode_envelope_file_structural(envelope)?;
    frame_log_record(&body)
}

fn frame_log_record(body: &[u8]) -> Result<Vec<u8>> {
    let body_len = len_to_u64(body.len())?;
    let checksum = log_record_checksum(body_len, body);
    let mut out = Vec::new();
    out.extend_from_slice(REF_LOG_MAGIC);
    push_u16(&mut out, REF_LOG_VERSION);
    push_u64(&mut out, body_len);
    out.extend_from_slice(&checksum);
    out.extend_from_slice(body);
    Ok(out)
}

/// Result of attempting to parse one ref-log frame at a given offset. Mirrors `wal::FrameAttempt`.
enum LogFrameAttempt {
    Record {
        record: RefLogRecord,
        next_offset: usize,
    },
    TrailingPartial {
        remaining: usize,
    },
    Invalid {
        message: String,
    },
}

/// Attempt to parse one ref-log frame at `offset`. Mirrors `wal::parse_frame_at` -- never trusts a
/// not-yet-checksum-validated header's own `body_len` for anything beyond locating where its
/// claimed body would end.
fn parse_log_frame_at(bytes: &[u8], offset: usize) -> LogFrameAttempt {
    let remaining = bytes.len().saturating_sub(offset);
    if remaining < REF_LOG_HEADER_LEN {
        return LogFrameAttempt::TrailingPartial { remaining };
    }
    let header_end = offset + REF_LOG_HEADER_LEN;
    // In range by construction: `remaining >= REF_LOG_HEADER_LEN` was just checked above --
    // `.get()` used anyway to satisfy `clippy::indexing_slicing`, not because this can fail.
    let Some(header) = bytes.get(offset..header_end) else {
        return LogFrameAttempt::TrailingPartial { remaining };
    };
    let header_values = match parse_log_header(header) {
        Ok(values) => values,
        Err(err) => {
            return LogFrameAttempt::Invalid {
                message: err.to_string(),
            };
        }
    };
    let Ok(body_len) = usize::try_from(header_values.body_len) else {
        return LogFrameAttempt::Invalid {
            message: "ref-log body length does not fit usize".to_string(),
        };
    };
    let Some(body_end) = header_end.checked_add(body_len) else {
        return LogFrameAttempt::Invalid {
            message: "ref-log body end overflow".to_string(),
        };
    };
    let Some(body) = bytes.get(header_end..body_end) else {
        return LogFrameAttempt::TrailingPartial { remaining };
    };
    let expected = log_record_checksum(header_values.body_len, body);
    if expected != header_values.checksum {
        return LogFrameAttempt::Invalid {
            message: format!("ref-log checksum mismatch at byte offset {offset}"),
        };
    }
    let envelope = match decode_envelope_file(body) {
        Ok(envelope) => envelope,
        Err(err) => {
            return LogFrameAttempt::Invalid {
                message: err.to_string(),
            };
        }
    };
    if let Err(err) = require_signed_type(&envelope, ObjectType::RefUpdate) {
        return LogFrameAttempt::Invalid {
            message: err.to_string(),
        };
    }
    LogFrameAttempt::Record {
        record: RefLogRecord { envelope },
        next_offset: body_end,
    }
}

/// RFC 102 Stage 2: isolate-and-continue reading, mirroring `wal::decode_records`. A frame that
/// fails to validate no longer aborts replay -- its offset and error are recorded as a `Failed`
/// outcome, and `frame_resync::resync_to_next_magic` (RFC 102 Stage 3: shared with `wal.rs` and the
/// container read path, not a third copy) finds the next candidate frame so every subsequent sound
/// record is still read.
fn decode_log_records(bytes: &[u8]) -> Result<RefLogReplay> {
    let mut records = Vec::new();
    let mut record_outcomes = Vec::new();
    let mut offset = 0_usize;
    loop {
        match parse_log_frame_at(bytes, offset) {
            LogFrameAttempt::Record {
                record,
                next_offset,
            } => {
                record_outcomes.push(RefLogRecordOutcome {
                    offset,
                    status: RefLogRecordStatus::Evaluated,
                });
                records.push(record);
                offset = next_offset;
            }
            LogFrameAttempt::TrailingPartial { remaining } => {
                return Ok(RefLogReplay {
                    records,
                    trailing_partial_bytes: remaining,
                    record_outcomes,
                });
            }
            LogFrameAttempt::Invalid { message } => {
                record_outcomes.push(RefLogRecordOutcome {
                    offset,
                    status: RefLogRecordStatus::Failed { message },
                });
                match resync_to_next_magic(bytes, offset + 1, REF_LOG_MAGIC.as_slice()) {
                    Some(next) => offset = next,
                    None => {
                        return Ok(RefLogReplay {
                            records,
                            trailing_partial_bytes: 0,
                            record_outcomes,
                        });
                    }
                }
            }
        }
    }
}

struct RefLogHeader {
    body_len: u64,
    checksum: [u8; 32],
}

fn parse_log_header(header: &[u8]) -> Result<RefLogHeader> {
    let mut cursor = ByteCursor::new(header);
    let magic = cursor.read_array::<8>()?;
    if &magic != REF_LOG_MAGIC {
        return Err(PrikkError::MalformedData(
            "invalid ref-log record magic".to_string(),
        ));
    }
    let version = cursor.read_u16()?;
    if version != REF_LOG_VERSION {
        return Err(PrikkError::UnsupportedFormatVersion(u32::from(version)));
    }
    let body_len = cursor.read_u64()?;
    let checksum = cursor.read_array::<32>()?;
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData(
            "trailing bytes in ref-log header".to_string(),
        ));
    }
    Ok(RefLogHeader { body_len, checksum })
}

fn log_record_checksum(body_len: u64, body: &[u8]) -> [u8; 32] {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(REF_LOG_MAGIC);
    preimage.extend_from_slice(&REF_LOG_VERSION.to_be_bytes());
    preimage.extend_from_slice(&body_len.to_be_bytes());
    preimage.extend_from_slice(body);
    sha256(&preimage)
}

#[cfg(test)]
mod tests;
