//! Repository verification routines.
//!
//! Verification is read-only. It checks object identity, object-type placement, envelope decoding,
//! sealed block references, joint ref publication state, active WAL replay checksums, and retained
//! active-publication cleanup state. Mutation belongs to narrow doctor or signer-backed seal paths.

use std::path::PathBuf;

mod objects;
mod ref_publication;
mod trust;

use prikk_error::{PrikkError, Result};
use prikk_object::{BlockPayload, ObjectId, ObjectType};

use crate::active::{ActiveRefMetadata, read_active_ref_metadata};
use crate::layout::{RepositoryFormat, RepositoryLayout};
use crate::object_store::FileObjectStore;
use crate::refs::verify_refs;
use crate::rollback_verify::{verify_rollback_draft_wal_records, verify_rollback_patch_envelope};
use crate::signature_diagnostics::{
    SignatureEnvelopeIssue, SignatureEnvelopeSource, classify_signature_envelope,
};
use crate::trust::PublicationTrustIssue;
use crate::wal::Wal;

use objects::verify_objects;
use trust::PublicationTrustVerifier;

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
    /// True when format-1 scaffold roots cannot be verified as clean-state commitments.
    pub legacy_state_roots_unverifiable: bool,
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
    /// Interrupted ref-publication and candidate-debris conditions found by joint verification.
    pub ref_publication_issues: Vec<crate::refs::RefPublicationIssue>,
    /// Warning-level format-1 signature-envelope compatibility findings in deterministic order.
    pub signature_envelope_issues: Vec<SignatureEnvelopeIssue>,
    /// Number of active WAL records classified and decoded as rollback drafts.
    pub checked_rollback_draft_records: usize,
    /// Number of publication envelopes checked against repository-local trust.
    pub checked_publication_trust_records: usize,
    /// Publication-trust issues found while structural verification succeeded.
    pub publication_trust_issues: Vec<PublicationTrustIssue>,
    /// Recognized non-authoritative object publication temps left for explicit maintenance.
    pub object_temp_paths: Vec<PathBuf>,
    /// Number of trailing bytes in the active WAL that look like an incomplete final record.
    pub trailing_partial_wal_bytes: usize,
    /// Active-WAL ref metadata status relative to the replayed WAL.
    pub active_wal_metadata_status: ActiveWalMetadataStatus,
}

impl RepositoryVerification {
    /// Return true when legacy scaffold roots prevent state-commitment verification.
    #[must_use]
    pub const fn has_unverifiable_state_roots(&self) -> bool {
        self.legacy_state_roots_unverifiable
    }

    /// Return true if the active WAL contained an incomplete trailing record.
    #[must_use]
    pub const fn has_trailing_partial_wal(&self) -> bool {
        self.trailing_partial_wal_bytes != 0
    }

    /// Return true when all structurally verified publication objects also passed trust checks.
    #[must_use]
    pub fn has_publication_trust_issues(&self) -> bool {
        !self.publication_trust_issues.is_empty()
    }

    /// Return true when pointer/log state requires signer-backed recovery or manual intervention.
    #[must_use]
    pub fn has_blocking_ref_publication_issues(&self) -> bool {
        self.ref_publication_issues
            .iter()
            .any(|issue| issue.blocking)
    }

    /// Return true when a non-empty active WAL lacks valid ownership metadata.
    #[must_use]
    pub const fn has_active_wal_metadata_integrity_issue(&self) -> bool {
        self.active_wal_metadata_status.has_integrity_issue()
    }

    /// Return true when an empty active WAL has stale local metadata debris.
    #[must_use]
    pub const fn has_active_wal_metadata_warning(&self) -> bool {
        self.active_wal_metadata_status.has_local_debris_warning()
    }
}

/// Active-WAL ref metadata status derived during repository verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveWalMetadataStatus {
    /// Empty active WAL and no metadata.
    MissingForEmptyWal,
    /// Empty active WAL with stale but valid local metadata.
    ValidForEmptyWal {
        /// Ref recorded in the stale metadata.
        ref_name: String,
    },
    /// Empty active WAL with malformed local metadata.
    InvalidForEmptyWal {
        /// Parse or validation failure.
        reason: String,
    },
    /// Non-empty active WAL with valid ownership metadata.
    ValidForNonEmptyWal {
        /// Ref recorded in the active metadata.
        ref_name: String,
    },
    /// Non-empty active WAL missing required ownership metadata.
    MissingForNonEmptyWal,
    /// Non-empty active WAL with malformed ownership metadata.
    InvalidForNonEmptyWal {
        /// Parse or validation failure.
        reason: String,
    },
}

impl ActiveWalMetadataStatus {
    /// Return true when the status represents a repository-integrity issue.
    #[must_use]
    pub const fn has_integrity_issue(&self) -> bool {
        matches!(
            self,
            Self::MissingForNonEmptyWal | Self::InvalidForNonEmptyWal { .. }
        )
    }

    /// Return true when the status represents local debris on an otherwise empty active WAL.
    #[must_use]
    pub const fn has_local_debris_warning(&self) -> bool {
        matches!(
            self,
            Self::ValidForEmptyWal { .. } | Self::InvalidForEmptyWal { .. }
        )
    }
}

/// Verify a repository layout without modifying it.
pub fn verify_repository(layout: &RepositoryLayout) -> Result<RepositoryVerification> {
    let object_store = FileObjectStore::new(layout.clone());
    let mut trust_verifier = PublicationTrustVerifier::new(layout);
    let object_summary = verify_objects(layout, &object_store, &mut trust_verifier)?;
    let ref_verification = verify_refs(layout)?;
    for envelope in &ref_verification.ref_update_envelopes {
        crate::format::validate_read_schema(layout.format(), envelope)?;
        trust_verifier.verify(envelope)?;
    }
    let wal = Wal::for_layout(layout);
    let replay = wal.replay()?;
    let persisted_wal_patches = verify_wal_persistence(&object_store, &replay.records)?;
    let checked_rollback_draft_records = verify_rollback_draft_wal_records(&replay.records)?;
    let mut signature_envelope_issues = object_summary.signature_issues;
    for record in &replay.records {
        crate::format::validate_read_schema(layout.format(), &record.envelope)?;
        signature_envelope_issues.extend(classify_signature_envelope(
            &record.envelope,
            SignatureEnvelopeSource::ActiveWal {
                sequence: record.seq,
                object_id: record.envelope.object_id(),
            },
        )?);
    }
    signature_envelope_issues.extend(ref_verification.signature_envelope_issues);
    let active_wal_metadata_status =
        classify_active_wal_metadata(layout, replay.records.is_empty())?;
    let mut ref_publication_issues = ref_verification.publication_issues;
    ref_publication::require_retained_evidence(
        layout,
        &replay.records,
        &active_wal_metadata_status,
        trust_verifier.issues.is_empty(),
        &mut ref_publication_issues,
    )?;
    Ok(RepositoryVerification {
        legacy_state_roots_unverifiable: layout.format() == RepositoryFormat::LegacyV1,
        checked_objects: object_summary.object_count,
        checked_wal_records: replay.records.len(),
        checked_blocks: object_summary.block_count,
        checked_rollback_blocks: object_summary.rollback_block_count,
        checked_sealed_rollback_patches: object_summary.rollback_patch_count,
        persisted_wal_patches,
        checked_refs: ref_verification.pointer_count,
        checked_ref_log_records: ref_verification.log_record_count,
        ref_publication_issues,
        signature_envelope_issues,
        checked_rollback_draft_records,
        checked_publication_trust_records: trust_verifier.checked_records,
        publication_trust_issues: trust_verifier.issues,
        object_temp_paths: object_summary.temp_paths,
        trailing_partial_wal_bytes: replay.trailing_partial_bytes,
        active_wal_metadata_status,
    })
}

fn classify_active_wal_metadata(
    layout: &RepositoryLayout,
    wal_is_empty: bool,
) -> Result<ActiveWalMetadataStatus> {
    match (wal_is_empty, read_active_ref_metadata(layout)?) {
        (true, ActiveRefMetadata::Missing) => Ok(ActiveWalMetadataStatus::MissingForEmptyWal),
        (true, ActiveRefMetadata::Valid(ref_name)) => {
            Ok(ActiveWalMetadataStatus::ValidForEmptyWal { ref_name })
        }
        (true, ActiveRefMetadata::Invalid(reason)) => {
            Ok(ActiveWalMetadataStatus::InvalidForEmptyWal { reason })
        }
        (false, ActiveRefMetadata::Missing) => Ok(ActiveWalMetadataStatus::MissingForNonEmptyWal),
        (false, ActiveRefMetadata::Valid(ref_name)) => {
            Ok(ActiveWalMetadataStatus::ValidForNonEmptyWal { ref_name })
        }
        (false, ActiveRefMetadata::Invalid(reason)) => {
            Ok(ActiveWalMetadataStatus::InvalidForNonEmptyWal { reason })
        }
    }
}

fn verify_block_payload(
    object_store: &FileObjectStore,
    block_id: ObjectId,
    format: RepositoryFormat,
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
    if format == RepositoryFormat::CurrentV2 {
        crate::block_state::verify_block_v2_state(object_store, block_id, &payload)?;
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

#[cfg(test)]
mod tests;
