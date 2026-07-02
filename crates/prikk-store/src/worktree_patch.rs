//! Worktree-to-patch authoring.
//!
//! Turns worktree changes into a node-addressed, role-bound Ed25519 AUTHOR-signed Patch envelope
//! against a replay-derived baseline and appends it to the active WAL. The change-detection,
//! operation-mapping, minting, mode-normalization, and canonical-ordering logic lives in
//! [`node_authoring`]; signing goes through the injected [`crate::AuthorSigner`] boundary (no
//! placeholder signer). Existing paths resolve to their persisted `node_id`; fresh nodes are minted
//! in canonical create order; text edits go through the shared `text_span` module. Rename inference
//! and symlink authoring remain out of scope.

use prikk_error::{PrikkError, Result};
use prikk_object::ObjectId;

use crate::author_signing::AuthorSigner;
use crate::layout::RepositoryLayout;
use crate::node_id_gen::NodeIdGenerator;

mod node_authoring;

/// Result of authoring and appending a node-addressed patch from worktree changes.
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
    /// Number of whole-file `EditText` operations emitted (text-file modifications).
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
    /// A modified UTF-8 text file is represented as a whole-file `EditText`.
    EditText,
    /// A regular file whose normalized mode changed is represented as `ChangePerm`.
    ChangePerm,
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
            Self::ChangePerm => "change-perm",
        }
    }
}

/// Options for authoring a node-addressed patch from worktree changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorktreePatchCommitOptions {
    /// Retained for API compatibility. Existing-node `NodeKind` is now authoritative for the
    /// modified-file mapping (text files author `EditText`, binary files author `ReplaceBinary`),
    /// so this flag no longer drives kind selection and is a no-op.
    pub prefer_text_edits: bool,
}

impl WorktreePatchCommitOptions {
    /// Default options (kind-driven mapping; `prefer_text_edits` is a no-op, see the field).
    #[must_use]
    pub const fn file_level() -> Self {
        Self {
            prefer_text_edits: false,
        }
    }

    /// Options with `prefer_text_edits` set; retained for API compatibility (no-op, see the field).
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

/// Author a node-addressed patch from worktree changes against the replay-derived baseline, sign it
/// with a real role-bound Ed25519 AUTHOR signature from the injected `signer`, and append it to the
/// active WAL (DC-09 Phase 4.4a, R1). Existing paths resolve to their persisted `node_id`; fresh
/// nodes are minted through the production [`NodeIdGenerator`]; text edits go through the shared
/// `text_span` module. There is no placeholder signing path.
pub fn commit_worktree_changes_signed(
    layout: &RepositoryLayout,
    ref_name: &str,
    message: &str,
    options: WorktreePatchCommitOptions,
    signer: &impl AuthorSigner,
) -> Result<WorktreePatchCommitReport> {
    let mut generator = NodeIdGenerator::production();
    node_authoring::author_worktree_patch(
        layout,
        ref_name,
        message,
        options,
        &mut generator,
        signer,
    )
}

/// Test-only entry that injects a deterministic node-id generator and an explicit signer so authoring
/// is reproducible.
#[cfg(test)]
pub(crate) fn commit_worktree_changes_with_generator<S, A>(
    layout: &RepositoryLayout,
    ref_name: &str,
    message: &str,
    options: WorktreePatchCommitOptions,
    generator: &mut NodeIdGenerator<S>,
    signer: &A,
) -> Result<WorktreePatchCommitReport>
where
    S: crate::node_id_gen::NodeIdEntropySource,
    A: AuthorSigner,
{
    node_authoring::author_worktree_patch(layout, ref_name, message, options, generator, signer)
}

/// Assign a contiguous `op_seq` (1-based) for the operation at `index`. Used by node-addressed
/// worktree authoring.
pub(crate) fn next_op_seq(index: usize) -> Result<u32> {
    let next = index
        .checked_add(1)
        .ok_or_else(|| PrikkError::CanonicalEncoding("operation count overflow".to_string()))?;
    u32::try_from(next)
        .map_err(|_| PrikkError::CanonicalEncoding("operation count exceeds u32".to_string()))
}

#[cfg(test)]
mod tests;
