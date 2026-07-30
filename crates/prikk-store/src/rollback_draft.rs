//! Mutating rollback draft append for the supported patch subset.
//!
//! This module deliberately keeps rollback publication and worktree mutation out of scope. It
//! validates the same supported inverse plan used by rollback preview, requires an empty active
//! WAL, marks the inverse Patch payload with `PatchPurpose::RollbackDraft`, signs it with a real
//! role-bound Ed25519 AUTHOR signer, and appends that Patch envelope to the active WAL under the
//! active-session lock. The existing seal path is still responsible for publishing refs later.

use prikk_error::{PrikkError, Result};
use prikk_object::{
    CanonicalEncode, ObjectEnvelope, ObjectId, ObjectType, PatchPurpose, RefStatePayload,
};

use crate::active::prepare_empty_active_ref_for_append;
use crate::author_signing::{AuthorSigner, author_signature};
use crate::layout::RepositoryLayout;
use crate::lock::ActiveLock;
use crate::object_store::FileObjectStore;
use crate::patch_inverse::{PatchInverseOperationSummary, prepare_patch_inverse_plan};
use crate::refs::RefStore;
use crate::rollback_preview::{RollbackPreviewChange, prepare_rollback_preview};
use crate::wal::Wal;
use crate::{
    ActiveRefMetadata, read_active_ref_metadata, remove_active_ref_metadata,
    validate_local_branch_ref,
};

/// Return true when a Patch envelope carries the rollback-draft payload purpose.
pub(crate) fn is_rollback_draft_envelope(envelope: &ObjectEnvelope) -> Result<bool> {
    if envelope.object_type != ObjectType::Patch {
        return Ok(false);
    }
    Ok(
        PatchPurpose::decode_from_patch_payload(&envelope.canonical_payload)?
            == PatchPurpose::RollbackDraft,
    )
}

/// Result of appending a supported inverse Patch draft to the active WAL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackDraftReport {
    /// Ref used as the rollback-draft target.
    pub ref_name: String,
    /// Published block that the inverse Patch draft targets.
    pub target_block_id: ObjectId,
    /// Signed inverse Patch ID appended to the active WAL.
    pub inverse_patch_id: ObjectId,
    /// Real AUTHOR key id recorded in the rollback draft signature.
    pub author_key_id: String,
    /// WAL sequence assigned to the signed inverse Patch envelope.
    pub wal_sequence: u64,
    /// Number of blocks inspected while deriving the inverse Patch.
    pub block_count: usize,
    /// Number of patch objects inspected while deriving the inverse Patch.
    pub patch_count: usize,
    /// Number of inverse operations appended.
    pub inverse_operation_count: usize,
    /// Number of file-level preview changes compared with the latest snapshot baseline.
    pub preview_change_count: usize,
    /// Number of files rollback would create or restore.
    pub would_create_files: usize,
    /// Number of files rollback would delete.
    pub would_delete_files: usize,
    /// Number of files rollback would replace.
    pub would_replace_files: usize,
    /// Operation summaries in inverse Patch application order.
    pub operations: Vec<PatchInverseOperationSummary>,
    /// Preview changes reported before the inverse Patch was appended.
    pub preview_changes: Vec<RollbackPreviewChange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RollbackTargetTip {
    ref_state_id: ObjectId,
    target_block_id: ObjectId,
}

/// Append a signed inverse Patch draft to an empty active WAL.
///
/// This function is intentionally conservative: it refuses an empty message, unpublished refs,
/// unsupported patch operations, partial WAL tails, and non-empty active WALs. It writes no object
/// files, publishes no refs, and makes no worktree changes.
pub fn append_rollback_draft(
    layout: &RepositoryLayout,
    ref_name: &str,
    message: &str,
    signer: &impl AuthorSigner,
) -> Result<RollbackDraftReport> {
    layout.require_current_format()?;
    crate::refs::ensure_no_incomplete_publication(layout)?;
    let canonical_ref = validate_local_branch_ref(ref_name)?;
    if message.trim().is_empty() {
        return Err(PrikkError::InvalidName(
            "rollback draft message must not be empty".to_string(),
        ));
    }

    let planned_tip = read_target_tip(layout, &canonical_ref)?;
    let mut inverse = prepare_patch_inverse_plan(layout, &canonical_ref)?;
    if inverse.inverse_operation_count == 0 {
        return Err(PrikkError::InvalidName(
            "rollback draft has no supported inverse operations to append".to_string(),
        ));
    }
    if inverse.target_block_id != planned_tip.target_block_id {
        return Err(PrikkError::Integrity(format!(
            "rollback inverse target {} does not match current ref target {}",
            inverse.target_block_id, planned_tip.target_block_id
        )));
    }
    let preview = prepare_rollback_preview(layout, &canonical_ref)?;
    if preview.target_block_id != inverse.target_block_id {
        return Err(PrikkError::Integrity(format!(
            "rollback preview target {} does not match inverse target {}",
            preview.target_block_id, inverse.target_block_id
        )));
    }

    inverse.inverse_payload.purpose = PatchPurpose::RollbackDraft;
    let canonical_payload = inverse.inverse_payload.to_canonical_bytes()?;
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, canonical_payload);
    let signature = author_signature(signer, envelope.object_id())?;
    envelope.add_signature(signature)?;
    let inverse_patch_id = envelope.object_id();

    let wal = Wal::for_layout(layout);
    let _lock = ActiveLock::acquire(layout)?;
    crate::refs::ensure_no_incomplete_publication(layout)?;
    let replay = wal.replay()?;
    if replay.trailing_partial_bytes != 0 {
        return Err(PrikkError::Integrity(format!(
            "active WAL has {} trailing partial bytes; run doctor before rollback-draft",
            replay.trailing_partial_bytes
        )));
    }
    if !replay.records.is_empty() {
        return Err(PrikkError::LockConflict(
            "rollback-draft requires an empty active WAL".to_string(),
        ));
    }
    let current_tip = read_target_tip(layout, &canonical_ref)?;
    if current_tip != planned_tip {
        return Err(PrikkError::LockConflict(
            "rollback-draft target ref changed during planning; retry rollback-draft".to_string(),
        ));
    }
    match read_active_ref_metadata(layout)? {
        ActiveRefMetadata::Missing => {}
        ActiveRefMetadata::Valid(_) | ActiveRefMetadata::Invalid(_) => {
            remove_active_ref_metadata(layout)?;
        }
    }
    prepare_empty_active_ref_for_append(layout, &canonical_ref)?;
    let wal_sequence = wal.append_patch(&envelope)?;

    Ok(RollbackDraftReport {
        ref_name: canonical_ref,
        target_block_id: inverse.target_block_id,
        inverse_patch_id,
        author_key_id: signer.key_id().to_string(),
        wal_sequence,
        block_count: inverse.block_count,
        patch_count: inverse.patch_count,
        inverse_operation_count: inverse.inverse_operation_count,
        preview_change_count: preview.change_count,
        would_create_files: preview.would_create_files,
        would_delete_files: preview.would_delete_files,
        would_replace_files: preview.would_replace_files,
        operations: inverse.operations,
        preview_changes: preview.changes,
    })
}

fn read_target_tip(layout: &RepositoryLayout, ref_name: &str) -> Result<RollbackTargetTip> {
    let ref_store = RefStore::new(layout.clone());
    let ref_state_id = ref_store
        .read_current_ref_state_id(ref_name)?
        .ok_or_else(|| PrikkError::InvalidName(format!("ref {ref_name} is not published")))?;
    let object_store = FileObjectStore::new(layout.clone());
    let envelope = object_store
        .read_typed(ref_state_id, ObjectType::RefState)?
        .ok_or_else(|| {
            PrikkError::Integrity(format!(
                "published ref {ref_name} points to missing RefState"
            ))
        })?;
    let payload =
        RefStatePayload::decode_canonical(&envelope.canonical_payload, envelope.schema_version)?;
    if payload.ref_name != ref_name {
        return Err(PrikkError::Integrity(format!(
            "published RefState name mismatch: expected {ref_name}, got {}",
            payload.ref_name
        )));
    }
    Ok(RollbackTargetTip {
        ref_state_id,
        target_block_id: payload.target_object_id,
    })
}

#[cfg(test)]
mod tests;
