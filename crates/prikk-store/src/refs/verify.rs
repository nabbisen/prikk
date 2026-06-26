//! Ref pointer and ref-log verification.

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use prikk_error::{PrikkError, Result};
use prikk_object::ObjectType;

use crate::layout::RepositoryLayout;
use crate::object_store::FileObjectStore;
use crate::refs::log::decode_log_file_bytes;
use crate::refs::pointer::read_ref_pointer;

/// Ref verification counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RefVerification {
    /// Number of checked ref pointer files.
    pub pointer_count: usize,
    /// Number of checked ref-log records.
    pub log_record_count: usize,
}

/// Verify all ref pointer files and log records for a repository.
pub(crate) fn verify_refs(layout: &RepositoryLayout) -> Result<RefVerification> {
    let pointer_count = verify_ref_pointers(layout)?;
    let log_count = verify_ref_logs(layout)?;
    Ok(RefVerification { pointer_count, log_record_count: log_count })
}

fn verify_ref_pointers(layout: &RepositoryLayout) -> Result<usize> {
    let dir = layout.refs_dir().join("by-id");
    if !dir.exists() {
        return Ok(0);
    }
    let mut checked = 0_usize;
    let object_store = FileObjectStore::new(layout.clone());
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            return Err(PrikkError::Integrity(format!(
                "unexpected directory in ref pointer directory: {}",
                path.display()
            )));
        }
        if is_temporary_path(&path) {
            continue;
        }
        let pointer = read_ref_pointer(&path)?;
        let expected_path = layout.ref_pointer_path(&pointer.ref_name);
        if path != expected_path {
            return Err(PrikkError::Integrity(format!(
                "ref pointer {} does not match canonical path {}",
                path.display(),
                expected_path.display()
            )));
        }
        let Some(envelope) = object_store.read_typed(pointer.ref_state_id, ObjectType::RefState)? else {
            return Err(PrikkError::Integrity(format!(
                "ref pointer {} references missing RefState {}",
                path.display(),
                pointer.ref_state_id
            )));
        };
        if envelope.signatures.is_empty() {
            return Err(PrikkError::Integrity(format!(
                "ref pointer {} references unsigned RefState {}",
                path.display(),
                pointer.ref_state_id
            )));
        }
        checked = checked
            .checked_add(1)
            .ok_or_else(|| PrikkError::Integrity("ref pointer count overflow".to_string()))?;
    }
    Ok(checked)
}

fn verify_ref_logs(layout: &RepositoryLayout) -> Result<usize> {
    let dir = layout.refs_dir().join("logs");
    if !dir.exists() {
        return Ok(0);
    }
    let mut checked = 0_usize;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            return Err(PrikkError::Integrity(format!(
                "unexpected directory in ref-log directory: {}",
                path.display()
            )));
        }
        if is_temporary_path(&path) {
            continue;
        }
        ensure_log_path_shape(&path)?;
        let mut bytes = Vec::new();
        File::open(path)?.read_to_end(&mut bytes)?;
        let replay = decode_log_file_bytes(&bytes)?;
        if replay.trailing_partial_bytes != 0 {
            return Err(PrikkError::Integrity(
                "ref log contains trailing partial record".to_string(),
            ));
        }
        checked = checked
            .checked_add(replay.records.len())
            .ok_or_else(|| PrikkError::Integrity("ref-log count overflow".to_string()))?;
    }
    Ok(checked)
}

fn ensure_log_path_shape(path: &Path) -> Result<()> {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return Err(PrikkError::Integrity(format!(
            "ref-log path is not valid UTF-8: {}",
            path.display()
        )));
    };
    if file_name.len() != 68 || !file_name.ends_with(".log") {
        return Err(PrikkError::Integrity(format!(
            "ref-log path does not use sha256hex.log shape: {}",
            path.display()
        )));
    }
    Ok(())
}

fn is_temporary_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.contains(".tmp"))
        .unwrap_or(false)
}
