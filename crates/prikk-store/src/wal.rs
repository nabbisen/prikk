//! Write-ahead log for active patch envelopes.

use std::fs;
use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};
use prikk_hash::sha256;
use prikk_object::{ObjectEnvelope, ObjectType};

use crate::byte_cursor::ByteCursor;
use crate::file_codec::{decode_envelope_file, encode_envelope_file, push_u16, push_u64};
use crate::fsutil::{
    MutationRoot, append_file_required, ensure_directory_required, len_to_u64, read_file_if_exists,
    truncate_existing_file_required, truncate_file_empty_required,
};
use crate::layout::RepositoryLayout;

const WAL_RECORD_MAGIC: &[u8; 8] = b"PWALR001";
const WAL_RECORD_VERSION: u16 = 1;
const WAL_HEADER_LEN: usize = 8 + 2 + 8 + 8 + 32;

/// One durable WAL record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalRecord {
    /// Monotonic WAL sequence.
    pub seq: u64,
    /// Exact signed object envelope stored at commit time.
    pub envelope: ObjectEnvelope,
}

/// WAL replay result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalReplay {
    /// Valid records read from the start of the WAL.
    pub records: Vec<WalRecord>,
    /// Number of trailing bytes ignored as an incomplete final record.
    pub trailing_partial_bytes: usize,
}

/// Result of a safe WAL tail truncation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalRepair {
    /// Number of valid records preserved after repair.
    pub preserved_records: usize,
    /// Number of trailing partial bytes truncated.
    pub truncated_bytes: usize,
}

/// File-backed active-session WAL.
#[derive(Debug, Clone)]
pub struct Wal {
    path: PathBuf,
    mutation: Option<(MutationRoot, PathBuf)>,
}

impl Wal {
    /// Create a WAL handle for a path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            mutation: None,
        }
    }

    /// Create a WAL handle authorized by a validated repository layout.
    #[must_use]
    pub fn for_layout(layout: &RepositoryLayout) -> Self {
        let path = layout.default_queue_wal_path();
        let relative = PathBuf::from("active/default/queue.wal");
        Self {
            path,
            mutation: Some((layout.repository_mutation_root().clone(), relative)),
        }
    }

    /// Return the WAL path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append a signed patch envelope and fsync the WAL file.
    pub fn append_patch(&self, envelope: &ObjectEnvelope) -> Result<u64> {
        if envelope.object_type != ObjectType::Patch {
            return Err(PrikkError::ObjectTypeMismatch {
                expected: ObjectType::Patch.to_string(),
                actual: envelope.object_type.to_string(),
            });
        }
        if envelope.signatures.is_empty() {
            return Err(PrikkError::InvalidSignature(
                "commit WAL entries must store signed patch envelopes".to_string(),
            ));
        }
        let replay = self.replay()?;
        if replay.trailing_partial_bytes != 0 {
            return Err(PrikkError::Integrity(
                "cannot append after an incomplete WAL tail".to_string(),
            ));
        }
        let (root, relative) = self.mutation()?;
        if let Some(last) = replay.records.last()
            && last.envelope == *envelope
        {
            append_file_required(root, relative, &[])?;
            return Ok(last.seq);
        }
        let next_seq = replay.records.last().map_or(Ok(1), |last| {
            last.seq
                .checked_add(1)
                .ok_or_else(|| PrikkError::MalformedData("WAL sequence overflow".to_string()))
        })?;
        let record = WalRecord {
            seq: next_seq,
            envelope: envelope.clone(),
        };
        let bytes = encode_record(&record)?;
        let Some(parent) = relative.parent() else {
            return Err(PrikkError::Io(
                "WAL path has no parent directory".to_string(),
            ));
        };
        ensure_directory_required(root, parent)?;
        append_file_required(root, relative, &bytes)?;
        Ok(next_seq)
    }

    /// Replay valid WAL records from the beginning.
    pub fn replay(&self) -> Result<WalReplay> {
        let Some(bytes) = self.read_bytes()? else {
            return Ok(WalReplay {
                records: Vec::new(),
                trailing_partial_bytes: 0,
            });
        };
        decode_records(&bytes)
    }

    /// Safely truncate an incomplete trailing WAL record, if one exists.
    ///
    /// This repairs only the case that FDD-02 defines as safe: valid records followed by an
    /// incomplete final record. Checksum mismatches in complete records still return an error and
    /// are not modified.
    pub fn truncate_trailing_partial(&self) -> Result<WalRepair> {
        let Some(bytes) = self.read_bytes()? else {
            return Ok(WalRepair {
                preserved_records: 0,
                truncated_bytes: 0,
            });
        };
        let replay = decode_records(&bytes)?;
        if replay.trailing_partial_bytes == 0 {
            return Ok(WalRepair {
                preserved_records: replay.records.len(),
                truncated_bytes: 0,
            });
        }
        let current_len = u64::try_from(bytes.len())
            .map_err(|_| PrikkError::MalformedData("WAL length does not fit u64".to_string()))?;
        let trailing = u64::try_from(replay.trailing_partial_bytes).map_err(|_| {
            PrikkError::MalformedData("trailing WAL byte count does not fit u64".to_string())
        })?;
        let repaired_len = current_len.checked_sub(trailing).ok_or_else(|| {
            PrikkError::MalformedData("trailing WAL byte count exceeds file length".to_string())
        })?;
        let (root, relative) = self.mutation()?;
        truncate_existing_file_required(root, relative, repaired_len)?;
        Ok(WalRepair {
            preserved_records: replay.records.len(),
            truncated_bytes: replay.trailing_partial_bytes,
        })
    }

    /// Truncate the WAL after a successful publication that made all entries durable elsewhere.
    pub fn truncate_empty(&self) -> Result<()> {
        let (root, relative) = self.mutation()?;
        let Some(parent) = relative.parent() else {
            return Err(PrikkError::Io(
                "WAL path has no parent directory".to_string(),
            ));
        };
        ensure_directory_required(root, parent)?;
        truncate_file_empty_required(root, relative)
    }

    /// Return the next sequence number for append.
    pub fn next_sequence(&self) -> Result<u64> {
        let replay = self.replay()?;
        let Some(last) = replay.records.last() else {
            return Ok(1);
        };
        last.seq
            .checked_add(1)
            .ok_or_else(|| PrikkError::MalformedData("WAL sequence overflow".to_string()))
    }

    fn mutation(&self) -> Result<(&MutationRoot, &Path)> {
        self.mutation
            .as_ref()
            .map(|(root, relative)| (root, relative.as_path()))
            .ok_or_else(|| {
                PrikkError::Io(
                    "WAL mutation requires a validated repository layout capability".to_string(),
                )
            })
    }

    fn read_bytes(&self) -> Result<Option<Vec<u8>>> {
        if let Some((root, relative)) = &self.mutation {
            read_file_if_exists(root, relative)
        } else {
            match fs::read(&self.path) {
                Ok(bytes) => Ok(Some(bytes)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(error) => Err(error.into()),
            }
        }
    }
}

fn encode_record(record: &WalRecord) -> Result<Vec<u8>> {
    let body = encode_envelope_file(&record.envelope)?;
    let body_len = len_to_u64(body.len())?;
    let checksum = record_checksum(record.seq, body_len, &body);
    let mut out = Vec::with_capacity(WAL_HEADER_LEN + body.len());
    out.extend_from_slice(WAL_RECORD_MAGIC);
    push_u16(&mut out, WAL_RECORD_VERSION);
    push_u64(&mut out, record.seq);
    push_u64(&mut out, body_len);
    out.extend_from_slice(&checksum);
    out.extend_from_slice(&body);
    Ok(out)
}

fn decode_records(bytes: &[u8]) -> Result<WalReplay> {
    let mut records = Vec::new();
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let remaining = bytes.len().saturating_sub(offset);
        if remaining < WAL_HEADER_LEN {
            return Ok(WalReplay {
                records,
                trailing_partial_bytes: remaining,
            });
        }
        let header_end = offset + WAL_HEADER_LEN;
        let header = bytes
            .get(offset..header_end)
            .ok_or_else(|| PrikkError::MalformedData("WAL header range overflow".to_string()))?;
        let header_values = parse_header(header)?;
        let body_len = usize::try_from(header_values.body_len).map_err(|_| {
            PrikkError::MalformedData("WAL body length does not fit usize".to_string())
        })?;
        let body_end = header_end
            .checked_add(body_len)
            .ok_or_else(|| PrikkError::MalformedData("WAL body end overflow".to_string()))?;
        let Some(body) = bytes.get(header_end..body_end) else {
            return Ok(WalReplay {
                records,
                trailing_partial_bytes: remaining,
            });
        };
        let expected = record_checksum(header_values.seq, header_values.body_len, body);
        if expected != header_values.checksum {
            return Err(PrikkError::Integrity(format!(
                "WAL checksum mismatch at byte offset {offset}"
            )));
        }
        let envelope = decode_envelope_file(body)?;
        records.push(WalRecord {
            seq: header_values.seq,
            envelope,
        });
        offset = body_end;
    }
    Ok(WalReplay {
        records,
        trailing_partial_bytes: 0,
    })
}

struct WalHeader {
    seq: u64,
    body_len: u64,
    checksum: [u8; 32],
}

fn parse_header(header: &[u8]) -> Result<WalHeader> {
    let mut cursor = ByteCursor::new(header);
    let magic = cursor.read_array::<8>()?;
    if &magic != WAL_RECORD_MAGIC {
        return Err(PrikkError::MalformedData(
            "invalid WAL record magic".to_string(),
        ));
    }
    let version = cursor.read_u16()?;
    if version != WAL_RECORD_VERSION {
        return Err(PrikkError::UnsupportedFormatVersion(u32::from(version)));
    }
    let seq = cursor.read_u64()?;
    let body_len = cursor.read_u64()?;
    let checksum = cursor.read_array::<32>()?;
    if !cursor.is_finished() {
        return Err(PrikkError::MalformedData(
            "trailing bytes in WAL header".to_string(),
        ));
    }
    Ok(WalHeader {
        seq,
        body_len,
        checksum,
    })
}

fn record_checksum(seq: u64, body_len: u64, body: &[u8]) -> [u8; 32] {
    let mut preimage = Vec::with_capacity(8 + 2 + 8 + 8 + body.len());
    preimage.extend_from_slice(WAL_RECORD_MAGIC);
    preimage.extend_from_slice(&WAL_RECORD_VERSION.to_be_bytes());
    preimage.extend_from_slice(&seq.to_be_bytes());
    preimage.extend_from_slice(&body_len.to_be_bytes());
    preimage.extend_from_slice(body);
    sha256(&preimage)
}

#[cfg(test)]
mod tests;
