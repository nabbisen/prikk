//! Worktree-to-patch draft generation.
//!
//! PR-025 turns snapshot-baseline worktree changes into a minimal signed Patch envelope and appends
//! it to the active WAL. It emits coarse file-level replace operations for modified files.
//! Create, delete, and opt-in text-edit authoring are deferred to the node model (node_id minting/
//! tracking, increment 4.4/4.4a; §9.3 EditText additionally needs FDD-01 §7.2.1 span anchoring).
//! Rename detection, arbitrary-span text diffs, and full algebra remain later increments.

use std::collections::BTreeMap;

use prikk_error::{PrikkError, Result};
use prikk_hash::sha256;
use prikk_object::{
    BlobKind, BlobPayload, CanonicalEncode, ObjectEnvelope, ObjectId, ObjectType, Signature,
    SignatureAlgorithm, SignerRole,
};

use crate::checkout::prepare_snapshot_checkout_plan;
use crate::layout::RepositoryLayout;
use crate::object_store::{FileObjectStore, ObjectReader, ObjectWriter};
use crate::snapshot::SnapshotManifest;
use crate::worktree_status::{WorktreeChangeKind, worktree_status};

/// Result of creating and appending a patch from snapshot-baseline worktree changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreePatchCommitReport {
    /// Baseline ref used to classify changes.
    pub ref_name: String,
    /// Patch object ID appended to the active WAL.
    pub patch_id: ObjectId,
    /// WAL sequence assigned to the patch envelope.
    pub wal_sequence: u64,
    /// Number of patch operations emitted.
    pub operation_count: usize,
    /// Number of Blob object references written or reused for operation payloads.
    pub referenced_blob_count: usize,
    /// Number of `EditText` operations emitted. Always 0 while text-edit authoring is
    /// deferred to the node model (increment 4.4 + FDD-01 §7.2.1).
    pub text_edit_count: usize,
    /// Operation summaries in emitted order.
    pub changes: Vec<WorktreePatchOperationSummary>,
}

/// Summary of one generated patch operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreePatchOperationSummary {
    /// Repository-relative path.
    pub path: String,
    /// Generated operation kind.
    pub operation: WorktreePatchOperationKind,
}

/// Generated operation kind for CLI/reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreePatchOperationKind {
    /// A new file will be represented as `CreateFile`.
    CreateFile,
    /// A missing tracked file will be represented as `DeleteFile`.
    DeleteFile,
    /// A modified tracked file will be represented as `ReplaceBinary`.
    ReplaceBinary,
    /// A modified UTF-8 tracked file would be represented as `EditText` (authoring deferred).
    EditText,
}

impl WorktreePatchOperationKind {
    /// Stable CLI label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CreateFile => "create-file",
            Self::DeleteFile => "delete-file",
            Self::ReplaceBinary => "replace-binary",
            Self::EditText => "edit-text",
        }
    }
}

/// Options for generating a patch from snapshot-baseline worktree changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorktreePatchCommitOptions {
    /// Prefer `EditText` for UTF-8 modified tracked files. NOTE: text-edit authoring is
    /// currently deferred to the node model, so enabling this makes modified-file commits
    /// fail closed rather than emit a node-bearing EditText with a fabricated node_id.
    pub prefer_text_edits: bool,
}

impl WorktreePatchCommitOptions {
    /// Return the default coarse file-level patch generation mode.
    #[must_use]
    pub const fn file_level() -> Self {
        Self {
            prefer_text_edits: false,
        }
    }

    /// Return the opt-in text-edit generation mode (authoring deferred to the node model).
    #[must_use]
    pub const fn prefer_text_edits() -> Self {
        Self {
            prefer_text_edits: true,
        }
    }
}

impl Default for WorktreePatchCommitOptions {
    fn default() -> Self {
        Self::file_level()
    }
}

/// Generate a minimal patch from snapshot-baseline worktree changes and append it to the active WAL.
pub fn commit_worktree_changes(
    layout: &RepositoryLayout,
    ref_name: &str,
    message: &str,
) -> Result<WorktreePatchCommitReport> {
    commit_worktree_changes_with_options(
        layout,
        ref_name,
        message,
        WorktreePatchCommitOptions::file_level(),
    )
}

/// Generate a minimal patch using explicit worktree patch generation options.
///
/// As of increment 4.2c, every §9.3 mutation operation (`CreateFile`, `DeleteNode`,
/// `EditText`, `ReplaceBinary`) is node-addressed, so worktree authoring of all
/// change kinds fails closed pending the node model (path->node_id tracking and
/// node_id minting, increments 4.4/4.4a; `EditText` also needs FDD-01 §7.2.1). The
/// blob-writing, op-sequencing, signing, and WAL-append helpers are retained as the
/// substrate the node model re-enables.
pub fn commit_worktree_changes_with_options(
    layout: &RepositoryLayout,
    ref_name: &str,
    message: &str,
    _options: WorktreePatchCommitOptions,
) -> Result<WorktreePatchCommitReport> {
    if message.trim().is_empty() {
        return Err(PrikkError::InvalidName(
            "commit message must not be empty".to_string(),
        ));
    }
    let status = worktree_status(layout, ref_name)?;
    if status.is_clean() {
        return Err(PrikkError::InvalidName(
            "worktree has no snapshot-baseline changes to commit".to_string(),
        ));
    }
    if status.count_kind(WorktreeChangeKind::UnsupportedPath) > 0 {
        return Err(PrikkError::InvalidName(
            "worktree contains paths that cannot be represented safely".to_string(),
        ));
    }

    // As of increment 4.2c, every FDD-03 §9.3 mutation operation (CreateFile,
    // DeleteNode, EditText, ReplaceBinary) is node-addressed, so authoring any
    // worktree change requires the node model: path->node_id tracking and node_id
    // minting (increments 4.4/4.4a), and for EditText span anchoring (FDD-01
    // §7.2.1). Until then, authoring fails closed on the first change rather than
    // emit a node-bearing operation with a fabricated node_id, which the node-model
    // plan forbids. `status.is_clean()` above guarantees at least one change.
    let change = status.changes.first().ok_or_else(|| {
        PrikkError::Integrity("worktree change set unexpectedly empty".to_string())
    })?;
    match change.kind {
        WorktreeChangeKind::Missing => Err(PrikkError::Integrity(format!(
            "worktree delete authoring is pending the node model \
             (increment 4.4 path->node_id tracking): {}",
            change.path
        ))),
        WorktreeChangeKind::Modified => Err(PrikkError::Integrity(format!(
            "worktree modified-file authoring is pending the node model \
             (increment 4.4 path->node_id tracking; ReplaceBinary binary-only blob \
             check; EditText needs FDD-01 §7.2.1): {}",
            change.path
        ))),
        WorktreeChangeKind::Untracked => Err(PrikkError::Integrity(format!(
            "worktree create authoring is pending the node model \
             (increment 4.4a node_id minting): {}",
            change.path
        ))),
        WorktreeChangeKind::UnsupportedPath => Err(PrikkError::InvalidName(format!(
            "unsupported worktree path cannot become a patch operation: {}",
            change.path
        ))),
    }
}

#[allow(dead_code)] // node-model authoring substrate; re-enters production at increment 4.4
fn load_snapshot_baseline(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let plan = prepare_snapshot_checkout_plan(layout, ref_name)?;
    let object_store = FileObjectStore::new(layout.clone());
    let Some(envelope) = object_store.read_object(plan.snapshot_blob_id)? else {
        return Err(PrikkError::Integrity(format!(
            "snapshot Blob {} is missing",
            plan.snapshot_blob_id
        )));
    };
    if envelope.object_type != ObjectType::Blob {
        return Err(PrikkError::ObjectTypeMismatch {
            expected: ObjectType::Blob.to_string(),
            actual: envelope.object_type.to_string(),
        });
    }
    let snapshot_content = crate::blob_access::decode_snapshot_blob(&envelope.canonical_payload)?;
    let manifest = SnapshotManifest::decode(&snapshot_content)?;
    let mut out = BTreeMap::new();
    for entry in manifest.files {
        out.insert(entry.path.as_str().to_string(), entry.bytes);
    }
    Ok(out)
}

#[allow(dead_code)] // shared test helper; re-enters production worktree authoring at increment 4.4
fn write_blob(object_store: &mut FileObjectStore, bytes: &[u8]) -> Result<ObjectId> {
    let payload = BlobPayload::new(BlobKind::Text, bytes.to_vec());
    let canonical_payload = payload.to_canonical_bytes()?;
    let envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, canonical_payload);
    object_store.write_object(&envelope)
}

#[allow(dead_code)] // node-model authoring substrate; re-enters production at increment 4.4
fn next_op_seq(index: usize) -> Result<u32> {
    let next = index
        .checked_add(1)
        .ok_or_else(|| PrikkError::CanonicalEncoding("operation count overflow".to_string()))?;
    u32::try_from(next)
        .map_err(|_| PrikkError::CanonicalEncoding("operation count exceeds u32".to_string()))
}

#[allow(dead_code)] // node-model authoring substrate; re-enters production at increment 4.4
fn dev_author_signature(message: &str) -> Signature {
    let mut signature_preimage = Vec::new();
    signature_preimage.extend_from_slice(b"prikk.dev.placeholder-signature.v1");
    signature_preimage.extend_from_slice(message.as_bytes());
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "dev-placeholder-author".to_string(),
        signature_bytes: sha256(&signature_preimage).to_vec(),
        created_at: 0,
        signer_role: SignerRole::Author,
    }
}

#[cfg(test)]
mod tests;
