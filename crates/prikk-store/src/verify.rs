//! Repository verification routines.
//!
//! Verification remains read-only in PR-014. It checks object identity, object-type
//! placement, envelope decoding, sealed block references, ref pointer/log consistency, and active
//! WAL replay checksums. Repair/truncation belongs to a later `doctor` increment.

use std::fs;
use std::path::{Path, PathBuf};

mod objects;

use prikk_error::{PrikkError, Result};
use prikk_object::{BlockPayload, ObjectEnvelope, ObjectId, ObjectType};

use crate::active::{ActiveRefMetadata, read_active_ref_metadata};
use crate::layout::RepositoryLayout;
use crate::object_store::FileObjectStore;
use crate::refs::{decode_log_file_bytes, verify_refs};
use crate::rollback_verify::{verify_rollback_draft_wal_records, verify_rollback_patch_envelope};
use crate::trust::{
    MaintainerTrustPolicy, PublicationTrustIssue, load_maintainer_trust_policy,
    verify_trusted_publication_envelope,
};
use crate::wal::Wal;

use objects::verify_objects;

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
    verify_ref_update_publication_trust(layout, &mut trust_verifier)?;
    let wal = Wal::new(layout.default_queue_wal_path());
    let replay = wal.replay()?;
    let persisted_wal_patches = verify_wal_persistence(&object_store, &replay.records)?;
    let checked_rollback_draft_records = verify_rollback_draft_wal_records(&replay.records)?;
    let active_wal_metadata_status =
        classify_active_wal_metadata(layout, replay.records.is_empty())?;
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

struct PublicationTrustVerifier<'a> {
    layout: &'a RepositoryLayout,
    policy: Option<MaintainerTrustPolicy>,
    policy_issue_added: bool,
    checked_records: usize,
    issues: Vec<PublicationTrustIssue>,
}

impl<'a> PublicationTrustVerifier<'a> {
    const fn new(layout: &'a RepositoryLayout) -> Self {
        Self {
            layout,
            policy: None,
            policy_issue_added: false,
            checked_records: 0,
            issues: Vec::new(),
        }
    }

    fn verify(&mut self, envelope: &ObjectEnvelope) -> Result<()> {
        self.checked_records = self
            .checked_records
            .checked_add(1)
            .ok_or_else(|| PrikkError::Integrity("publication trust count overflow".to_string()))?;
        if self.policy.is_none() && !self.policy_issue_added {
            match load_maintainer_trust_policy(self.layout) {
                Ok(policy) => self.policy = Some(policy),
                Err(err) => {
                    self.policy_issue_added = true;
                    self.issues.push(PublicationTrustIssue::new(
                        "PRIKK-TRUST-POLICY-INVALID",
                        format!("publication trust policy is invalid: {err}"),
                    ));
                    return Ok(());
                }
            }
        }
        if let Some(policy) = &self.policy {
            if let Err(issue) = verify_trusted_publication_envelope(policy, envelope) {
                self.issues.push(issue);
            }
        }
        Ok(())
    }
}

fn verify_ref_update_publication_trust(
    layout: &RepositoryLayout,
    trust_verifier: &mut PublicationTrustVerifier<'_>,
) -> Result<()> {
    let dir = layout.refs_dir().join("logs");
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() || is_temporary_path(&path) {
            continue;
        }
        let bytes = fs::read(&path)?;
        let replay = decode_log_file_bytes(&bytes)?;
        if replay.trailing_partial_bytes != 0 {
            continue;
        }
        for record in &replay.records {
            trust_verifier.verify(&record.envelope)?;
        }
    }
    Ok(())
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

fn is_temporary_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.contains(".tmp."))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests;
