//! Verification helpers for rollback draft patches.
//!
//! This module keeps rollback publication non-mutating, but makes rollback drafts easier to audit before
//! seal. The active WAL verifier can now classify rollback draft records by their dedicated
//! development signature marker and validate that their Patch payload remains in the supported
//! replay subset. The stronger `verify_active_rollback_draft` API additionally compares the WAL
//! payload with the inverse Patch that would be derived from the currently published ref.

use prikk_error::{PrikkError, Result};
use prikk_object::{CanonicalEncode, ObjectEnvelope, ObjectId, ObjectType};

use crate::layout::RepositoryLayout;
use crate::patch_inverse::prepare_patch_inverse_plan;
use crate::patch_replay::decode::{decode_patch_operations, ensure_apply_supported};
use crate::rollback_draft::is_rollback_draft_envelope;
use crate::wal::{Wal, WalRecord};

/// Verification result for one active rollback draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackDraftVerification {
    /// Ref used to derive the expected inverse Patch.
    pub ref_name: String,
    /// WAL sequence containing the rollback draft Patch envelope.
    pub wal_sequence: u64,
    /// Signed rollback draft Patch ID currently present in the active WAL.
    pub draft_patch_id: ObjectId,
    /// Published block that was used as the rollback target.
    pub target_block_id: ObjectId,
    /// Number of blocks inspected while deriving the expected inverse.
    pub block_count: usize,
    /// Number of patch objects inspected while deriving the expected inverse.
    pub patch_count: usize,
    /// Number of supported inverse operations expected for this rollback draft.
    pub inverse_operation_count: usize,
    /// Number of supported operations decoded from the active WAL payload.
    pub decoded_operation_count: usize,
}

/// Verify that the active WAL contains exactly one rollback draft matching the current ref.
///
/// This is intentionally a pre-seal validation helper. It does not write objects, publish refs,
/// or mutate the worktree. It refuses trailing partial WAL bytes, non-rollback WAL records, and
/// rollback payloads that no longer match the inverse Patch derived from the selected ref.
pub fn verify_active_rollback_draft(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<RollbackDraftVerification> {
    let wal = Wal::new(layout.default_queue_wal_path());
    let replay = wal.replay()?;
    if replay.trailing_partial_bytes != 0 {
        return Err(PrikkError::Integrity(format!(
            "active WAL has {} trailing partial bytes; run doctor before rollback-draft-verify",
            replay.trailing_partial_bytes
        )));
    }
    let Some(record) = single_wal_record(&replay.records)? else {
        return Err(PrikkError::Integrity(
            "rollback-draft-verify requires exactly one active WAL record".to_string(),
        ));
    };
    verify_rollback_marker(record)?;

    let inverse = prepare_patch_inverse_plan(layout, ref_name)?;
    let expected_payload = inverse.inverse_payload.to_canonical_bytes()?;
    if record.envelope.canonical_payload != expected_payload {
        return Err(PrikkError::Integrity(
            "active rollback draft payload does not match the current inverse plan".to_string(),
        ));
    }
    let decoded = decode_patch_operations(&record.envelope.canonical_payload)?;
    // Erratum P1: decoding all §9.3 kinds does not prove the draft is replayable. A
    // rollback draft must consist only of apply-supported operations; gate explicitly
    // rather than relying on decode success.
    for operation in &decoded {
        ensure_apply_supported(operation)?;
    }
    if decoded.len() != inverse.inverse_operation_count {
        return Err(PrikkError::Integrity(format!(
            "rollback draft decoded {} operations but inverse plan has {}",
            decoded.len(),
            inverse.inverse_operation_count
        )));
    }

    Ok(RollbackDraftVerification {
        ref_name: ref_name.to_string(),
        wal_sequence: record.seq,
        draft_patch_id: record.envelope.object_id(),
        target_block_id: inverse.target_block_id,
        block_count: inverse.block_count,
        patch_count: inverse.patch_count,
        inverse_operation_count: inverse.inverse_operation_count,
        decoded_operation_count: decoded.len(),
    })
}

pub(crate) fn verify_rollback_draft_wal_records(records: &[WalRecord]) -> Result<usize> {
    let mut rollback_drafts = 0_usize;
    for record in records {
        let context = format!("rollback draft WAL record {}", record.seq);
        if verify_rollback_patch_envelope(&record.envelope, &context)? {
            rollback_drafts = rollback_drafts.checked_add(1).ok_or_else(|| {
                PrikkError::Integrity("rollback draft WAL count overflow".to_string())
            })?;
        }
    }
    Ok(rollback_drafts)
}

/// Verify a rollback-marked Patch envelope and return whether it is a rollback patch.
///
/// Non-rollback Patch envelopes return `Ok(false)`. Rollback-marked envelopes must decode under
/// the currently supported replay subset and must contain at least one inverse operation. This
/// helper is shared by active-WAL verification and sealed Block/history classification.
pub(crate) fn verify_rollback_patch_envelope(
    envelope: &ObjectEnvelope,
    context: &str,
) -> Result<bool> {
    if !is_rollback_draft_envelope(envelope) {
        return Ok(false);
    }
    if envelope.object_type != ObjectType::Patch {
        return Err(PrikkError::Integrity(format!(
            "{context} is {}, expected patch",
            envelope.object_type
        )));
    }
    let decoded = decode_patch_operations(&envelope.canonical_payload)?;
    // Erratum P1: require apply-support, not merely decodability.
    for operation in &decoded {
        ensure_apply_supported(operation)?;
    }
    if decoded.is_empty() {
        return Err(PrikkError::Integrity(format!(
            "{context} has no supported inverse operations"
        )));
    }
    Ok(true)
}

fn single_wal_record(records: &[WalRecord]) -> Result<Option<&WalRecord>> {
    match records {
        [] => Ok(None),
        [record] => Ok(Some(record)),
        _ => Err(PrikkError::LockConflict(
            "rollback-draft-verify requires an active WAL containing only the rollback draft"
                .to_string(),
        )),
    }
}

fn verify_rollback_marker(record: &WalRecord) -> Result<()> {
    if record.envelope.object_type != ObjectType::Patch {
        return Err(PrikkError::Integrity(format!(
            "rollback draft WAL record {} contains {}, expected patch",
            record.seq, record.envelope.object_type
        )));
    }
    if is_rollback_draft_envelope(&record.envelope) {
        return Ok(());
    }
    Err(PrikkError::InvalidSignature(format!(
        "active WAL record {} is not signed as a rollback draft",
        record.seq
    )))
}

#[cfg(test)]
mod tests;
