//! Repository verification routines.
//!
//! Verification is intentionally read-only in PR-007. It checks object identity, object-type
//! placement, envelope decoding, ref pointer/log consistency, and active WAL replay checksums.
//! Repair/truncation belongs to a later `doctor` increment.

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectId, ObjectType};

use crate::file_codec::decode_envelope_file;
use crate::layout::{persisted_object_types, RepositoryLayout};
use crate::refs::verify_refs;
use crate::wal::Wal;

/// Verification summary for a single persisted object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectVerification {
    /// The object ID parsed from the object filename.
    pub object_id: ObjectId,
    /// The object type implied by the directory being scanned.
    pub object_type: ObjectType,
    /// The object file path that was checked.
    pub path: PathBuf,
}

/// Repository verification summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryVerification {
    /// Number of persisted object files checked successfully.
    pub checked_objects: usize,
    /// Number of active WAL records replayed successfully.
    pub checked_wal_records: usize,
    /// Number of ref pointer files checked successfully.
    pub checked_refs: usize,
    /// Number of inline ref-log records checked successfully.
    pub checked_ref_log_records: usize,
    /// Number of trailing bytes in the active WAL that look like an incomplete final record.
    pub trailing_partial_wal_bytes: usize,
}

impl RepositoryVerification {
    /// Return true if the active WAL contained an incomplete trailing record.
    #[must_use]
    pub const fn has_trailing_partial_wal(&self) -> bool {
        self.trailing_partial_wal_bytes != 0
    }
}

/// Verify a repository layout without modifying it.
pub fn verify_repository(layout: &RepositoryLayout) -> Result<RepositoryVerification> {
    let checked_objects = verify_objects(layout)?;
    let ref_verification = verify_refs(layout)?;
    let wal = Wal::new(layout.default_queue_wal_path());
    let replay = wal.replay()?;
    Ok(RepositoryVerification {
        checked_objects,
        checked_wal_records: replay.records.len(),
        checked_refs: ref_verification.pointer_count,
        checked_ref_log_records: ref_verification.log_record_count,
        trailing_partial_wal_bytes: replay.trailing_partial_bytes,
    })
}

fn verify_objects(layout: &RepositoryLayout) -> Result<usize> {
    let mut checked = 0_usize;
    for object_type in persisted_object_types() {
        checked = checked
            .checked_add(verify_object_type(layout, object_type)?)
            .ok_or_else(|| {
                PrikkError::Integrity("object verification count overflow".to_string())
            })?;
    }
    Ok(checked)
}

fn verify_object_type(layout: &RepositoryLayout, object_type: ObjectType) -> Result<usize> {
    let dir = layout.object_type_dir(object_type);
    if !dir.exists() {
        return Ok(0);
    }
    let mut checked = 0_usize;
    for prefix_entry in fs::read_dir(&dir)? {
        let prefix_entry = prefix_entry?;
        let prefix_path = prefix_entry.path();
        if !prefix_path.is_dir() {
            if is_temporary_path(&prefix_path) {
                continue;
            }
            return Err(PrikkError::Integrity(format!(
                "unexpected non-directory in object type directory: {}",
                prefix_path.display()
            )));
        }
        checked = checked
            .checked_add(verify_prefix_dir(layout, object_type, &prefix_path)?)
            .ok_or_else(|| {
                PrikkError::Integrity("object verification count overflow".to_string())
            })?;
    }
    Ok(checked)
}

fn verify_prefix_dir(
    layout: &RepositoryLayout,
    object_type: ObjectType,
    prefix_path: &Path,
) -> Result<usize> {
    let mut checked = 0_usize;
    for file_entry in fs::read_dir(prefix_path)? {
        let file_entry = file_entry?;
        let path = file_entry.path();
        if path.is_dir() {
            return Err(PrikkError::Integrity(format!(
                "unexpected directory in object prefix directory: {}",
                path.display()
            )));
        }
        if is_temporary_path(&path) {
            continue;
        }
        verify_object_file(layout, object_type, &path)?;
        checked = checked
            .checked_add(1)
            .ok_or_else(|| {
                PrikkError::Integrity("object verification count overflow".to_string())
            })?;
    }
    Ok(checked)
}

fn verify_object_file(
    layout: &RepositoryLayout,
    object_type: ObjectType,
    path: &Path,
) -> Result<ObjectVerification> {
    let object_id = object_id_from_path(path)?;
    let expected_path = layout.object_path(object_type, object_id);
    if path != expected_path {
        return Err(PrikkError::Integrity(format!(
            "object path {} does not match canonical path {}",
            path.display(),
            expected_path.display()
        )));
    }
    let bytes = fs::read(path)?;
    let envelope = decode_envelope_file(&bytes)?;
    if envelope.object_type != object_type {
        return Err(PrikkError::Integrity(format!(
            "object file {} is under type {} but envelope type is {}",
            path.display(),
            object_type,
            envelope.object_type
        )));
    }
    let computed = envelope.object_id();
    if computed != object_id {
        return Err(PrikkError::Integrity(format!(
            "object file {} has id {} but computed id is {}",
            path.display(),
            object_id,
            computed
        )));
    }
    Ok(ObjectVerification { object_id, object_type, path: path.to_path_buf() })
}

fn object_id_from_path(path: &Path) -> Result<ObjectId> {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return Err(PrikkError::Integrity(format!(
            "object file path is not valid UTF-8: {}",
            path.display()
        )));
    };
    let Some(hex) = file_name.strip_suffix(".pobj") else {
        return Err(PrikkError::Integrity(format!(
            "object file does not use .pobj extension: {}",
            path.display()
        )));
    };
    ObjectId::from_str(hex)
}

fn is_temporary_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.contains(".tmp."))
        .unwrap_or(false)
}
