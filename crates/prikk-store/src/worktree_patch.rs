//! Worktree-to-patch draft generation.
//!
//! PR-025 turns snapshot-baseline worktree changes into a minimal signed Patch envelope and appends
//! it to the active WAL. By default it emits coarse file-level create/delete/replace operations.
//! With opt-in text mode it emits conservative full-file `EditText` operations for UTF-8
//! modifications. Rename detection, arbitrary-span text diffs, and full algebra remain later
//! increments.

use std::collections::BTreeMap;
use std::fs;

use prikk_error::{PrikkError, Result};
use prikk_hash::sha256;
use prikk_object::{
    BlobPayload, CanonicalEncode, CreateFile, DeleteFile, ObjectEnvelope, ObjectId, ObjectType,
    text_span_hash, EditText, Operation, OperationKind, PatchPayload, ReplaceBinary, Signature,
    SignatureAlgorithm, SignerRole,
};

use crate::active::ActiveSession;
use crate::checkout::prepare_snapshot_checkout_plan;
use crate::layout::RepositoryLayout;
use crate::object_store::{FileObjectStore, ObjectReader, ObjectWriter};
use crate::path::RepoPath;
use crate::snapshot::SnapshotManifest;
use crate::worktree_status::{worktree_status, WorktreeChangeKind};

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
    /// Number of full-file `EditText` operations emitted.
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
    /// A modified UTF-8 tracked file will be represented as full-file `EditText`.
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
    /// Prefer conservative full-file `EditText` for UTF-8 modified tracked files.
    pub prefer_text_edits: bool,
}

impl WorktreePatchCommitOptions {
    /// Return the default coarse file-level patch generation mode.
    #[must_use]
    pub const fn file_level() -> Self {
        Self { prefer_text_edits: false }
    }

    /// Return the opt-in full-file text-edit generation mode.
    #[must_use]
    pub const fn prefer_text_edits() -> Self {
        Self { prefer_text_edits: true }
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
pub fn commit_worktree_changes_with_options(
    layout: &RepositoryLayout,
    ref_name: &str,
    message: &str,
    options: WorktreePatchCommitOptions,
) -> Result<WorktreePatchCommitReport> {
    if message.trim().is_empty() {
        return Err(PrikkError::InvalidName("commit message must not be empty".to_string()));
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

    let baseline = load_snapshot_baseline(layout, ref_name)?;
    let mut object_store = FileObjectStore::new(layout.clone());
    let mut operations = Vec::new();
    let mut summaries = Vec::new();
    let mut referenced_blob_count = 0_usize;
    let mut text_edit_count = 0_usize;

    for change in &status.changes {
        let path = RepoPath::parse(&change.path)?;
        let op_seq = next_op_seq(operations.len())?;
        match change.kind {
            WorktreeChangeKind::Missing => {
                let old_bytes = baseline.get(path.as_str()).ok_or_else(|| {
                    PrikkError::Integrity(format!(
                        "missing tracked path was not found in baseline: {}",
                        path.as_str()
                    ))
                })?;
                let old_blob_id = write_blob(&mut object_store, old_bytes)?;
                referenced_blob_count += 1;
                operations.push(Operation {
                    op_seq,
                    op_id: Some(format!("delete-{}", path.as_str())),
                    preconditions: Vec::new(),
                    kind: OperationKind::DeleteFile(DeleteFile {
                        path: path.as_str().to_string(),
                        old_blob_id,
                    }),
                });
                summaries.push(WorktreePatchOperationSummary {
                    path: path.as_str().to_string(),
                    operation: WorktreePatchOperationKind::DeleteFile,
                });
            }
            WorktreeChangeKind::Modified => {
                let old_bytes = baseline.get(path.as_str()).ok_or_else(|| {
                    PrikkError::Integrity(format!(
                        "modified tracked path was not found in baseline: {}",
                        path.as_str()
                    ))
                })?;
                let new_bytes = read_regular_worktree_file(layout, &path)?;
                if options.prefer_text_edits {
                    if let Some(edit) = full_file_text_edit(&path, old_bytes, &new_bytes) {
                        operations.push(Operation {
                            op_seq,
                            op_id: Some(format!("edit-text-{}", path.as_str())),
                            preconditions: Vec::new(),
                            kind: OperationKind::EditText(edit),
                        });
                        summaries.push(WorktreePatchOperationSummary {
                            path: path.as_str().to_string(),
                            operation: WorktreePatchOperationKind::EditText,
                        });
                        text_edit_count += 1;
                        continue;
                    }
                }
                let old_blob_id = write_blob(&mut object_store, old_bytes)?;
                let new_blob_id = write_blob(&mut object_store, &new_bytes)?;
                referenced_blob_count += 2;
                operations.push(Operation {
                    op_seq,
                    op_id: Some(format!("replace-{}", path.as_str())),
                    preconditions: Vec::new(),
                    kind: OperationKind::ReplaceBinary(ReplaceBinary {
                        path: path.as_str().to_string(),
                        old_blob_id,
                        new_blob_id,
                    }),
                });
                summaries.push(WorktreePatchOperationSummary {
                    path: path.as_str().to_string(),
                    operation: WorktreePatchOperationKind::ReplaceBinary,
                });
            }
            WorktreeChangeKind::Untracked => {
                let new_bytes = read_regular_worktree_file(layout, &path)?;
                let blob_id = write_blob(&mut object_store, &new_bytes)?;
                referenced_blob_count += 1;
                operations.push(Operation {
                    op_seq,
                    op_id: Some(format!("create-{}", path.as_str())),
                    preconditions: Vec::new(),
                    kind: OperationKind::CreateFile(CreateFile {
                        path: path.as_str().to_string(),
                        blob_id,
                        mode: 0o100644,
                    }),
                });
                summaries.push(WorktreePatchOperationSummary {
                    path: path.as_str().to_string(),
                    operation: WorktreePatchOperationKind::CreateFile,
                });
            }
            WorktreeChangeKind::UnsupportedPath => {
                return Err(PrikkError::InvalidName(format!(
                    "unsupported worktree path cannot become a patch operation: {}",
                    change.path
                )));
            }
        }
    }

    let payload = PatchPayload {
        operations,
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
    };
    let payload_bytes = payload.to_canonical_bytes()?;
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, payload_bytes);
    envelope.add_signature(dev_author_signature(message))?;
    let patch_id = envelope.object_id();
    let wal_sequence = ActiveSession::new(layout.clone()).append_patch(&envelope)?.wal_sequence;

    Ok(WorktreePatchCommitReport {
        ref_name: ref_name.to_string(),
        patch_id,
        wal_sequence,
        operation_count: payload.operations.len(),
        referenced_blob_count,
        text_edit_count,
        changes: summaries,
    })
}

fn full_file_text_edit(path: &RepoPath, old_bytes: &[u8], new_bytes: &[u8]) -> Option<EditText> {
    let new_text = std::str::from_utf8(new_bytes).ok()?;
    std::str::from_utf8(old_bytes).ok()?;
    Some(EditText {
        path: path.as_str().to_string(),
        anchor_id: "full-file".to_string(),
        old_span_hash: text_span_hash(old_bytes),
        replacement: new_text.to_string(),
    })
}

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
    let blob = BlobPayload::decode_canonical(&envelope.canonical_payload)?;
    let manifest = SnapshotManifest::decode(&blob.bytes)?;
    let mut out = BTreeMap::new();
    for entry in manifest.files {
        out.insert(entry.path.as_str().to_string(), entry.bytes);
    }
    Ok(out)
}

fn write_blob(object_store: &mut FileObjectStore, bytes: &[u8]) -> Result<ObjectId> {
    let payload = BlobPayload { bytes: bytes.to_vec() };
    let canonical_payload = payload.to_canonical_bytes()?;
    let envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, canonical_payload);
    object_store.write_object(&envelope)
}

fn read_regular_worktree_file(layout: &RepositoryLayout, path: &RepoPath) -> Result<Vec<u8>> {
    let target = path.join_to_root(layout.root());
    let metadata = fs::symlink_metadata(&target)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(PrikkError::Integrity(format!(
            "worktree path is not a regular file: {}",
            target.display()
        )));
    }
    fs::read(&target).map_err(PrikkError::from)
}

fn next_op_seq(index: usize) -> Result<u32> {
    let next = index
        .checked_add(1)
        .ok_or_else(|| PrikkError::CanonicalEncoding("operation count overflow".to_string()))?;
    u32::try_from(next)
        .map_err(|_| PrikkError::CanonicalEncoding("operation count exceeds u32".to_string()))
}

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
