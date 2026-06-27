//! Conservative patch-replay worktree materialization.
//!
//! PR-021 adds an explicit, opt-in materializer for the supported file-level patch replay result.
//! It writes the replayed final manifest using the same safe path checks as snapshot checkout. It
//! does not remove extra worktree files yet; destructive removal remains a later, separately
//! designed increment.

use prikk_error::Result;

use crate::layout::RepositoryLayout;
use crate::patch_replay::replay_supported_patch_chain;
use crate::worktree::materialize_manifest_entries;

/// Result of an opt-in patch replay materialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchMaterializationReport {
    /// Human-readable ref name.
    pub ref_name: String,
    /// Number of blocks replayed from oldest to newest.
    pub block_count: usize,
    /// Number of patch objects replayed.
    pub patch_count: usize,
    /// Number of supported operations applied.
    pub applied_operation_count: usize,
    /// Number of files in the final replay manifest.
    pub planned_files: usize,
    /// Number of files written by this invocation.
    pub written_files: usize,
    /// Number of files already present with identical bytes.
    pub unchanged_files: usize,
    /// Total content bytes represented by the replayed manifest.
    pub total_content_bytes: u64,
    /// Repository-relative paths in materialization order.
    pub paths: Vec<String>,
}

/// Materialize the supported patch replay result into the repository worktree.
///
/// This is deliberately conservative:
///
/// - it supports only the `CreateFile`, `DeleteFile`, and `ReplaceBinary` subset already handled by
///   patch replay planning;
/// - it refuses conflicting existing files via the shared safe materializer;
/// - it never deletes files that are absent from the final replay manifest;
/// - it remains separate from full patch algebra and conflict handling.
pub fn materialize_patch_checkout(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<PatchMaterializationReport> {
    let snapshot = replay_supported_patch_chain(layout, ref_name)?;
    let write_report = materialize_manifest_entries(layout.root(), &snapshot.manifest)?;
    let paths = snapshot
        .manifest
        .files
        .iter()
        .map(|entry| entry.path.as_str().to_string())
        .collect();
    Ok(PatchMaterializationReport {
        ref_name: snapshot.ref_name,
        block_count: snapshot.block_count,
        patch_count: snapshot.patch_count,
        applied_operation_count: snapshot.applied_operation_count,
        planned_files: snapshot.manifest.files.len(),
        written_files: write_report.written_files,
        unchanged_files: write_report.unchanged_files,
        total_content_bytes: snapshot.manifest.total_content_bytes(),
        paths,
    })
}
