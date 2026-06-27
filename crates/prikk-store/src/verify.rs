//! Repository verification routines.
//!
//! Verification remains read-only in PR-014. It checks object identity, object-type
//! placement, envelope decoding, sealed block references, ref pointer/log consistency, and active
//! WAL replay checksums. Repair/truncation belongs to a later `doctor` increment.

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use prikk_error::{PrikkError, Result};
use prikk_object::{BlockPayload, ObjectId, ObjectType};

use crate::file_codec::decode_envelope_file;
use crate::layout::{RepositoryLayout, persisted_object_types};
use crate::object_store::FileObjectStore;
use crate::refs::verify_refs;
use crate::rollback_verify::{verify_rollback_draft_wal_records, verify_rollback_patch_envelope};
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
    /// Rollback-marked Patch references verified for this object when it is a Block.
    pub rollback_patch_count: usize,
}

/// Repository verification summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryVerification {
    /// Number of persisted object files checked successfully.
    pub checked_objects: usize,
    /// Number of active WAL records replayed successfully.
    pub checked_wal_records: usize,
    /// Number of persisted block objects whose references were checked.
    pub checked_blocks: usize,
    /// Number of persisted Block objects classified as rollback blocks.
    pub checked_rollback_blocks: usize,
    /// Number of sealed rollback-marked Patch objects referenced by verified Blocks.
    pub checked_sealed_rollback_patches: usize,
    /// Number of active WAL patch records that already exist as persisted patch objects.
    pub persisted_wal_patches: usize,
    /// Number of ref pointer files checked successfully.
    pub checked_refs: usize,
    /// Number of inline ref-log records checked successfully.
    pub checked_ref_log_records: usize,
    /// Number of active WAL records classified and decoded as rollback drafts.
    pub checked_rollback_draft_records: usize,
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
    let object_store = FileObjectStore::new(layout.clone());
    let object_summary = verify_objects(layout, &object_store)?;
    let ref_verification = verify_refs(layout)?;
    let wal = Wal::new(layout.default_queue_wal_path());
    let replay = wal.replay()?;
    let persisted_wal_patches = verify_wal_persistence(&object_store, &replay.records)?;
    let checked_rollback_draft_records = verify_rollback_draft_wal_records(&replay.records)?;
    Ok(RepositoryVerification {
        checked_objects: object_summary.object_count,
        checked_wal_records: replay.records.len(),
        checked_blocks: object_summary.block_count,
        checked_rollback_blocks: object_summary.rollback_block_count,
        checked_sealed_rollback_patches: object_summary.rollback_patch_count,
        persisted_wal_patches,
        checked_refs: ref_verification.pointer_count,
        checked_ref_log_records: ref_verification.log_record_count,
        checked_rollback_draft_records,
        trailing_partial_wal_bytes: replay.trailing_partial_bytes,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObjectSummary {
    object_count: usize,
    block_count: usize,
    rollback_block_count: usize,
    rollback_patch_count: usize,
}

impl ObjectSummary {
    const fn empty() -> Self {
        Self {
            object_count: 0,
            block_count: 0,
            rollback_block_count: 0,
            rollback_patch_count: 0,
        }
    }

    fn add(&mut self, other: Self) -> Result<()> {
        self.object_count = self
            .object_count
            .checked_add(other.object_count)
            .ok_or_else(|| {
                PrikkError::Integrity("object verification count overflow".to_string())
            })?;
        self.block_count = self
            .block_count
            .checked_add(other.block_count)
            .ok_or_else(|| {
                PrikkError::Integrity("block verification count overflow".to_string())
            })?;
        self.rollback_block_count = self
            .rollback_block_count
            .checked_add(other.rollback_block_count)
            .ok_or_else(|| PrikkError::Integrity("rollback block count overflow".to_string()))?;
        self.rollback_patch_count = self
            .rollback_patch_count
            .checked_add(other.rollback_patch_count)
            .ok_or_else(|| PrikkError::Integrity("rollback patch count overflow".to_string()))?;
        Ok(())
    }
}

fn verify_objects(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
) -> Result<ObjectSummary> {
    let mut summary = ObjectSummary::empty();
    for object_type in persisted_object_types() {
        let type_summary = verify_object_type(layout, object_store, object_type)?;
        summary.add(type_summary)?;
    }
    Ok(summary)
}

fn verify_object_type(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    object_type: ObjectType,
) -> Result<ObjectSummary> {
    let dir = layout.object_type_dir(object_type);
    if !dir.exists() {
        return Ok(ObjectSummary::empty());
    }
    let mut summary = ObjectSummary::empty();
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
        let prefix_summary = verify_prefix_dir(layout, object_store, object_type, &prefix_path)?;
        summary.add(prefix_summary)?;
    }
    Ok(summary)
}

fn verify_prefix_dir(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    object_type: ObjectType,
    prefix_path: &Path,
) -> Result<ObjectSummary> {
    let mut summary = ObjectSummary::empty();
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
        let object = verify_object_file(layout, object_store, object_type, &path)?;
        summary.object_count = summary.object_count.checked_add(1).ok_or_else(|| {
            PrikkError::Integrity("object verification count overflow".to_string())
        })?;
        if object.object_type == ObjectType::Block {
            summary.block_count = summary.block_count.checked_add(1).ok_or_else(|| {
                PrikkError::Integrity("block verification count overflow".to_string())
            })?;
            if object.rollback_patch_count != 0 {
                summary.rollback_block_count =
                    summary.rollback_block_count.checked_add(1).ok_or_else(|| {
                        PrikkError::Integrity("rollback block count overflow".to_string())
                    })?;
                summary.rollback_patch_count = summary
                    .rollback_patch_count
                    .checked_add(object.rollback_patch_count)
                    .ok_or_else(|| {
                        PrikkError::Integrity("rollback patch count overflow".to_string())
                    })?;
            }
        }
    }
    Ok(summary)
}

fn verify_object_file(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
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
    let rollback_patch_count = if object_type == ObjectType::Block {
        verify_block_payload(object_store, object_id, &envelope.canonical_payload)?
    } else {
        0
    };
    Ok(ObjectVerification {
        object_id,
        object_type,
        path: path.to_path_buf(),
        rollback_patch_count,
    })
}

fn verify_block_payload(
    object_store: &FileObjectStore,
    block_id: ObjectId,
    canonical_payload: &[u8],
) -> Result<usize> {
    let payload = BlockPayload::decode_canonical(canonical_payload)?;
    for parent in &payload.parent_block_ids {
        ensure_object_exists(
            object_store,
            ObjectType::Block,
            *parent,
            "parent block",
            block_id,
        )?;
    }
    let mut rollback_patch_count = 0_usize;
    for patch in &payload.patch_ids {
        let Some(envelope) = object_store.read_typed(*patch, ObjectType::Patch)? else {
            return Err(PrikkError::Integrity(format!(
                "object {block_id} references missing block patch {patch}"
            )));
        };
        let context = format!("sealed Block {block_id} Patch {patch}");
        if verify_rollback_patch_envelope(&envelope, &context)? {
            rollback_patch_count = rollback_patch_count.checked_add(1).ok_or_else(|| {
                PrikkError::Integrity("sealed rollback patch count overflow".to_string())
            })?;
        }
    }
    if let Some(snapshot) = payload.snapshot_blob_ref {
        ensure_object_exists(
            object_store,
            ObjectType::Blob,
            snapshot,
            "snapshot blob",
            block_id,
        )?;
    }
    Ok(rollback_patch_count)
}

fn ensure_object_exists(
    object_store: &FileObjectStore,
    object_type: ObjectType,
    object_id: ObjectId,
    role: &str,
    owner: ObjectId,
) -> Result<()> {
    let exists = object_store.read_typed(object_id, object_type)?.is_some();
    if exists {
        return Ok(());
    }
    Err(PrikkError::Integrity(format!(
        "object {owner} references missing {role} {object_id}"
    )))
}

fn verify_wal_persistence(
    object_store: &FileObjectStore,
    records: &[crate::WalRecord],
) -> Result<usize> {
    let mut persisted = 0_usize;
    for record in records {
        if record.envelope.object_type != ObjectType::Patch {
            return Err(PrikkError::Integrity(format!(
                "active WAL record {} contains {}, expected patch",
                record.seq, record.envelope.object_type
            )));
        }
        if object_store.contains_object(ObjectType::Patch, record.envelope.object_id()) {
            persisted = persisted.checked_add(1).ok_or_else(|| {
                PrikkError::Integrity("persisted WAL patch count overflow".to_string())
            })?;
        }
    }
    Ok(persisted)
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
