//! Ref-update log codec and replay.

use prikk_error::{PrikkError, Result};
use prikk_hash::sha256;
use prikk_object::{ObjectEnvelope, ObjectType};

use crate::byte_cursor::ByteCursor;
use crate::file_codec::{decode_envelope_file, encode_envelope_file, push_u16, push_u64};
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

/// Ref-log replay result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefLogReplay {
    /// Valid records read from the start of the log.
    pub records: Vec<RefLogRecord>,
    /// Number of trailing bytes ignored as an incomplete final record.
    pub trailing_partial_bytes: usize,
}

/// Append one signed RefUpdate envelope to the ref log and fsync it.
pub(crate) fn append_log_record(
    layout: &RepositoryLayout,
    ref_name: &str,
    envelope: &ObjectEnvelope,
) -> Result<()> {
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
        });
    };
    decode_log_records(&bytes)
}

pub(crate) fn decode_log_file_bytes(bytes: &[u8]) -> Result<RefLogReplay> {
    decode_log_records(bytes)
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
    require_signed_type(envelope, ObjectType::RefUpdate)?;
    let body = encode_envelope_file(envelope)?;
    let body_len = len_to_u64(body.len())?;
    let checksum = log_record_checksum(body_len, &body);
    let mut out = Vec::new();
    out.extend_from_slice(REF_LOG_MAGIC);
    push_u16(&mut out, REF_LOG_VERSION);
    push_u64(&mut out, body_len);
    out.extend_from_slice(&checksum);
    out.extend_from_slice(&body);
    Ok(out)
}

fn decode_log_records(bytes: &[u8]) -> Result<RefLogReplay> {
    let mut records = Vec::new();
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let remaining = bytes.len().saturating_sub(offset);
        if remaining < REF_LOG_HEADER_LEN {
            return Ok(RefLogReplay {
                records,
                trailing_partial_bytes: remaining,
            });
        }
        let header_end = offset
            .checked_add(REF_LOG_HEADER_LEN)
            .ok_or_else(|| PrikkError::MalformedData("ref-log header overflow".to_string()))?;
        let header = bytes.get(offset..header_end).ok_or_else(|| {
            PrikkError::MalformedData("ref-log header range overflow".to_string())
        })?;
        let header_values = parse_log_header(header)?;
        let body_len = usize::try_from(header_values.body_len).map_err(|_| {
            PrikkError::MalformedData("ref-log body length does not fit usize".to_string())
        })?;
        let body_end = header_end
            .checked_add(body_len)
            .ok_or_else(|| PrikkError::MalformedData("ref-log body end overflow".to_string()))?;
        let Some(body) = bytes.get(header_end..body_end) else {
            return Ok(RefLogReplay {
                records,
                trailing_partial_bytes: remaining,
            });
        };
        let expected = log_record_checksum(header_values.body_len, body);
        if expected != header_values.checksum {
            return Err(PrikkError::Integrity(format!(
                "ref-log checksum mismatch at byte offset {offset}"
            )));
        }
        let envelope = decode_envelope_file(body)?;
        require_signed_type(&envelope, ObjectType::RefUpdate)?;
        records.push(RefLogRecord { envelope });
        offset = body_end;
    }
    Ok(RefLogReplay {
        records,
        trailing_partial_bytes: 0,
    })
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
