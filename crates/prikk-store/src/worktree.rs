//! Safe worktree materialization helpers.
//!
//! PR-017 adds an opt-in snapshot materializer for snapshot-backed blocks. It writes only
//! repository-validated snapshot entries, refuses conflicting existing files, refuses symlinked
//! parents/targets, and never removes files.

use std::path::Path;

use prikk_error::{PrikkError, Result};
use prikk_object::ObjectType;

use crate::checkout::prepare_snapshot_checkout_plan;
use crate::fsutil::{
    ensure_directory_required, read_file_if_exists, sync_directory_required,
    write_worktree_file_atomically,
};
use crate::layout::RepositoryLayout;
use crate::object_store::{FileObjectStore, ObjectReader};
use crate::path::join_repo_path_to_root;
use crate::snapshot::{SnapshotEntry, SnapshotManifest};

/// Result of an opt-in snapshot worktree materialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotMaterializationReport {
    /// Human-readable ref name.
    pub ref_name: String,
    /// Number of files described by the snapshot manifest.
    pub planned_files: usize,
    /// Number of files written by this invocation.
    pub written_files: usize,
    /// Number of files already present with identical bytes.
    pub unchanged_files: usize,
    /// Total content bytes represented by the snapshot manifest.
    pub total_content_bytes: u64,
    /// Repository-relative paths in materialization order.
    pub paths: Vec<String>,
}

/// Materialize a snapshot-backed checkout into the repository worktree.
///
/// This operation is intentionally conservative:
///
/// - it accepts only blocks with a validated snapshot manifest;
/// - it refuses to overwrite an existing file with different bytes;
/// - it refuses symlinked parents and symlinked target files;
/// - it never removes extra worktree files;
/// - it relies on `RepoPath` validation to keep writes inside the worktree.
pub fn materialize_snapshot_checkout(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<SnapshotMaterializationReport> {
    let plan = prepare_snapshot_checkout_plan(layout, ref_name)?;
    let manifest = load_snapshot_manifest(layout, plan.snapshot_blob_id)?;
    let write_report = materialize_manifest_entries(layout, &manifest)?;
    Ok(SnapshotMaterializationReport {
        ref_name: ref_name.to_string(),
        planned_files: manifest.files.len(),
        written_files: write_report.written_files,
        unchanged_files: write_report.unchanged_files,
        total_content_bytes: manifest.total_content_bytes(),
        paths: manifest
            .files
            .iter()
            .map(|entry| entry.path.as_str().to_string())
            .collect(),
    })
}

/// Result of materializing a validated manifest into a worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManifestMaterializationReport {
    /// Number of files written by this invocation.
    pub(crate) written_files: usize,
    /// Number of files already present with identical bytes.
    pub(crate) unchanged_files: usize,
}

/// Materialize a validated manifest without deleting extra files.
pub(crate) fn materialize_manifest_entries(
    layout: &RepositoryLayout,
    manifest: &SnapshotManifest,
) -> Result<ManifestMaterializationReport> {
    let mut written_files = 0_usize;
    let mut unchanged_files = 0_usize;
    for entry in &manifest.files {
        match materialize_entry(layout, entry)? {
            EntryWriteOutcome::Written => written_files += 1,
            EntryWriteOutcome::Unchanged => unchanged_files += 1,
        }
    }
    Ok(ManifestMaterializationReport {
        written_files,
        unchanged_files,
    })
}

fn load_snapshot_manifest(
    layout: &RepositoryLayout,
    snapshot_blob_id: prikk_object::ObjectId,
) -> Result<SnapshotManifest> {
    let object_store = FileObjectStore::new(layout.clone());
    let Some(envelope) = object_store.read_object(snapshot_blob_id)? else {
        return Err(PrikkError::Integrity(format!(
            "snapshot Blob {snapshot_blob_id} is missing"
        )));
    };
    if envelope.object_type != ObjectType::Blob {
        return Err(PrikkError::ObjectTypeMismatch {
            expected: ObjectType::Blob.to_string(),
            actual: envelope.object_type.to_string(),
        });
    }
    let snapshot_content = crate::blob_access::decode_snapshot_blob(&envelope.canonical_payload)?;
    SnapshotManifest::decode(&snapshot_content)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryWriteOutcome {
    Written,
    Unchanged,
}

fn materialize_entry(
    layout: &RepositoryLayout,
    entry: &SnapshotEntry,
) -> Result<EntryWriteOutcome> {
    let root = layout.root();
    let target = join_repo_path_to_root(&entry.path, root);
    ensure_target_is_inside_root(root, &target)?;
    ensure_parent_directory(layout, entry.path.as_str())?;
    let relative = Path::new(entry.path.as_str());
    if let Some(current) = read_file_if_exists(layout.worktree_mutation_root(), relative)? {
        if current == entry.bytes {
            let parent = relative.parent().unwrap_or_else(|| Path::new(""));
            sync_directory_required(layout.worktree_mutation_root(), parent)?;
            return Ok(EntryWriteOutcome::Unchanged);
        }
        return Err(PrikkError::Integrity(format!(
            "refusing to overwrite existing file with different content: {}",
            target.display()
        )));
    }
    write_worktree_file_atomically(layout.worktree_mutation_root(), relative, &entry.bytes)?;
    Ok(EntryWriteOutcome::Written)
}

fn ensure_parent_directory(layout: &RepositoryLayout, repo_path: &str) -> Result<()> {
    let relative = Path::new(repo_path);
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    ensure_directory_required(layout.worktree_mutation_root(), parent)
}

fn ensure_target_is_inside_root(root: &Path, target: &Path) -> Result<()> {
    if !target.starts_with(root) {
        return Err(PrikkError::Integrity(format!(
            "materialization target escaped repository root: {}",
            target.display()
        )));
    }
    Ok(())
}
