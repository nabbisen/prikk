//! Ref pointer and ref-log verification.

use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectId, ObjectType, RefStatePayload, RefUpdatePayload};

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
    let object_store = FileObjectStore::new(layout.clone());
    let pointer_count = verify_ref_pointers(layout, &object_store)?;
    let log_count = verify_ref_logs(layout, &object_store)?;
    Ok(RefVerification {
        pointer_count,
        log_record_count: log_count,
    })
}

fn verify_ref_pointers(layout: &RepositoryLayout, object_store: &FileObjectStore) -> Result<usize> {
    let dir = layout.refs_dir().join("by-id");
    if !dir.exists() {
        return Ok(0);
    }
    let mut checked = 0_usize;
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
        ensure_ref_path_shape(&path, ".ref")?;
        let Some(pointer) = read_ref_pointer(layout, &path)? else {
            continue;
        };
        let expected_path = layout.ref_pointer_path(&pointer.ref_name);
        if path != expected_path {
            return Err(PrikkError::Integrity(format!(
                "ref pointer {} does not match canonical path {}",
                path.display(),
                expected_path.display()
            )));
        }
        let payload = verified_ref_state_payload(object_store, pointer.ref_state_id)?;
        if payload.ref_name != pointer.ref_name {
            return Err(PrikkError::Integrity(format!(
                "RefState {} name mismatch: pointer {}, payload {}",
                pointer.ref_state_id, pointer.ref_name, payload.ref_name
            )));
        }
        ensure_block_exists(object_store, payload.target_object_id, pointer.ref_state_id)?;
        checked = checked
            .checked_add(1)
            .ok_or_else(|| PrikkError::Integrity("ref pointer count overflow".to_string()))?;
    }
    Ok(checked)
}

fn verify_ref_logs(layout: &RepositoryLayout, object_store: &FileObjectStore) -> Result<usize> {
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
        ensure_ref_path_shape(&path, ".log")?;
        let mut bytes = Vec::new();
        File::open(path)?.read_to_end(&mut bytes)?;
        let replay = decode_log_file_bytes(&bytes)?;
        if replay.trailing_partial_bytes != 0 {
            return Err(PrikkError::Integrity(
                "ref log contains trailing partial record".to_string(),
            ));
        }
        let mut previous_ref_state_id = None;
        for record in &replay.records {
            let payload = RefUpdatePayload::decode_canonical(&record.envelope.canonical_payload)?;
            if payload.old_ref_state_id != previous_ref_state_id {
                return Err(PrikkError::Integrity(format!(
                    "ref-log chain mismatch for {} at update {}",
                    payload.ref_name, payload.update_seq
                )));
            }
            let ref_state_payload =
                verified_ref_state_payload(object_store, payload.new_ref_state_id)?;
            if ref_state_payload.ref_name != payload.ref_name {
                return Err(PrikkError::Integrity(format!(
                    "RefUpdate points to RefState with different ref name: {} vs {}",
                    payload.ref_name, ref_state_payload.ref_name
                )));
            }
            if ref_state_payload.previous_ref_state_id != payload.old_ref_state_id {
                return Err(PrikkError::Integrity(format!(
                    "RefState previous link disagrees with RefUpdate for {}",
                    payload.ref_name
                )));
            }
            if ref_state_payload.target_object_id != payload.new_target_object_id {
                return Err(PrikkError::Integrity(format!(
                    "RefState target disagrees with RefUpdate for {}",
                    payload.ref_name
                )));
            }
            ensure_block_exists(
                object_store,
                payload.new_target_object_id,
                payload.new_ref_state_id,
            )?;
            previous_ref_state_id = Some(payload.new_ref_state_id);
        }
        checked = checked
            .checked_add(replay.records.len())
            .ok_or_else(|| PrikkError::Integrity("ref-log count overflow".to_string()))?;
    }
    Ok(checked)
}

fn verified_ref_state_payload(
    object_store: &FileObjectStore,
    ref_state_id: ObjectId,
) -> Result<RefStatePayload> {
    let Some(envelope) = object_store.read_typed(ref_state_id, ObjectType::RefState)? else {
        return Err(PrikkError::Integrity(format!(
            "missing RefState object: {ref_state_id}"
        )));
    };
    if envelope.signatures.is_empty() {
        return Err(PrikkError::Integrity(format!(
            "RefState {ref_state_id} is unsigned"
        )));
    }
    RefStatePayload::decode_canonical(&envelope.canonical_payload)
}

fn ensure_block_exists(
    object_store: &FileObjectStore,
    block_id: ObjectId,
    owner: ObjectId,
) -> Result<()> {
    let exists = object_store
        .read_typed(block_id, ObjectType::Block)?
        .is_some();
    if exists {
        return Ok(());
    }
    Err(PrikkError::Integrity(format!(
        "ref object {owner} targets missing block {block_id}"
    )))
}

fn ensure_ref_path_shape(path: &Path, extension: &str) -> Result<()> {
    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return Err(PrikkError::Integrity(format!(
            "ref path is not valid UTF-8: {}",
            path.display()
        )));
    };
    let expected_len = 64_usize
        .checked_add(extension.len())
        .ok_or_else(|| PrikkError::Integrity("ref extension length overflow".to_string()))?;
    if file_name.len() != expected_len || !file_name.ends_with(extension) {
        return Err(PrikkError::Integrity(format!(
            "ref path does not use sha256hex{} shape: {}",
            extension,
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
