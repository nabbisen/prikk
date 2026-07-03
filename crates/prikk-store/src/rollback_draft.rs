//! Mutating rollback draft append for the supported patch subset.
//!
//! This module deliberately keeps rollback publication and worktree mutation out of scope. It
//! validates the same supported inverse plan used by rollback preview, requires an empty active
//! WAL, marks the inverse Patch payload with `PatchPurpose::RollbackDraft`, signs it with a real
//! role-bound Ed25519 AUTHOR signer, and appends that Patch envelope to the active WAL under the
//! active-session lock. The existing seal path is still responsible for publishing refs later.

use prikk_error::{PrikkError, Result};
use prikk_object::{CanonicalEncode, ObjectEnvelope, ObjectId, ObjectType, PatchPurpose};

use crate::author_signing::{AuthorSigner, author_signature};
use crate::layout::RepositoryLayout;
use crate::lock::ActiveLock;
use crate::patch_inverse::{PatchInverseOperationSummary, prepare_patch_inverse_plan};
use crate::rollback_preview::{RollbackPreviewChange, prepare_rollback_preview};
use crate::wal::Wal;

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
    if message.trim().is_empty() {
        return Err(PrikkError::InvalidName(
            "rollback draft message must not be empty".to_string(),
        ));
    }

    let mut inverse = prepare_patch_inverse_plan(layout, ref_name)?;
    if inverse.inverse_operation_count == 0 {
        return Err(PrikkError::InvalidName(
            "rollback draft has no supported inverse operations to append".to_string(),
        ));
    }
    let preview = prepare_rollback_preview(layout, ref_name)?;
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

    let wal = Wal::new(layout.default_queue_wal_path());
    let _lock = ActiveLock::acquire(layout.default_active_lock_path())?;
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
    let wal_sequence = wal.append_patch(&envelope)?;

    Ok(RollbackDraftReport {
        ref_name: ref_name.to_string(),
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

#[cfg(test)]
mod tests;
