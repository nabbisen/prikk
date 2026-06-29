//! Conservative patch-replay worktree materialization.
//!
//! PR-022 extends patch materialization with an explicit, opt-in deletion path. Deletion remains
//! narrowly scoped: only files removed by replayed `DeleteFile` operations are eligible, and the
//! current worktree bytes must still match the delete precondition bytes before removal.

use std::fs;
use std::path::Path;

use prikk_error::{PrikkError, Result};

use crate::fsutil::sync_directory_best_effort;
use crate::layout::RepositoryLayout;
use crate::patch_replay::{PatchReplayDeletedFile, replay_supported_patch_chain};
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
    /// Number of explicit patch-deleted files removed by this invocation.
    pub deleted_files: usize,
    /// Number of explicit patch-deleted files that were already absent.
    pub already_absent_deleted_files: usize,
    /// Number of explicit patch deletions refused by preflight checks.
    pub deletion_conflicts: usize,
    /// Total content bytes represented by the replayed manifest.
    pub total_content_bytes: u64,
    /// Repository-relative paths in materialization order.
    pub paths: Vec<String>,
}

/// Read-only plan for explicit patch checkout deletions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchDeletionPlan {
    /// Human-readable ref name.
    pub ref_name: String,
    /// Number of files explicitly deleted by the replayed patch chain.
    pub planned_deletions: usize,
    /// Number of deletion candidates that can be removed safely.
    pub deletable_files: usize,
    /// Number of deletion candidates that are already absent.
    pub already_absent_files: usize,
    /// Refused deletion candidates.
    pub conflicts: Vec<PatchDeletionConflict>,
    /// Deletable repository-relative paths.
    pub deletable_paths: Vec<String>,
}

impl PatchDeletionPlan {
    /// Return true when all explicit deletions are either safe or already complete.
    #[must_use]
    pub fn is_safe_to_apply(&self) -> bool {
        self.conflicts.is_empty()
    }
}

/// A deletion candidate that cannot be safely removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchDeletionConflict {
    /// Repository-relative path.
    pub path: String,
    /// Human-readable reason for refusing deletion.
    pub reason: String,
}

/// Materialize the supported patch replay result into the repository worktree.
///
/// This is deliberately conservative:
///
/// - it supports only the `CreateFile`, `DeleteFile`, and `ReplaceBinary` subset already handled by
///   patch replay planning;
/// - it refuses conflicting existing files via the shared safe materializer;
/// - it does not remove files absent from the replay result;
/// - it remains separate from full patch algebra and conflict handling.
pub fn materialize_patch_checkout(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<PatchMaterializationReport> {
    materialize_patch_checkout_inner(layout, ref_name, false)
}

/// Materialize the supported patch replay result and remove explicit patch-deleted files.
///
/// This opt-in path removes only files for which the replayed patch chain contains a `DeleteFile`
/// operation and the current worktree bytes still match the operation's `old_blob_id` bytes. It
/// never removes arbitrary untracked files.
pub fn materialize_patch_checkout_with_deletions(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<PatchMaterializationReport> {
    materialize_patch_checkout_inner(layout, ref_name, true)
}

/// Prepare a read-only deletion plan for explicit patch-deleted files.
pub fn plan_patch_checkout_deletions(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<PatchDeletionPlan> {
    let snapshot = replay_supported_patch_chain(layout, ref_name)?;
    let analysis = analyze_deletions(layout.root(), &snapshot.deleted_files)?;
    Ok(PatchDeletionPlan {
        ref_name: snapshot.ref_name,
        planned_deletions: snapshot.deleted_files.len(),
        deletable_files: analysis.deletable.len(),
        already_absent_files: analysis.already_absent,
        conflicts: analysis.conflicts,
        deletable_paths: analysis
            .deletable
            .iter()
            .map(|entry| entry.path.path.as_str().to_string())
            .collect(),
    })
}

fn materialize_patch_checkout_inner(
    layout: &RepositoryLayout,
    ref_name: &str,
    delete_removed: bool,
) -> Result<PatchMaterializationReport> {
    let snapshot = replay_supported_patch_chain(layout, ref_name)?;
    let deletion_analysis = analyze_deletions(layout.root(), &snapshot.deleted_files)?;
    if delete_removed && !deletion_analysis.conflicts.is_empty() {
        return Err(PrikkError::Integrity(format!(
            "refusing checkout deletion because {} candidate(s) are unsafe",
            deletion_analysis.conflicts.len()
        )));
    }

    let write_report = materialize_manifest_entries(layout.root(), &snapshot.manifest)?;
    let deleted_files = if delete_removed {
        apply_deletions(&deletion_analysis.deletable)?
    } else {
        0
    };
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
        deleted_files,
        already_absent_deleted_files: deletion_analysis.already_absent,
        deletion_conflicts: deletion_analysis.conflicts.len(),
        total_content_bytes: snapshot.manifest.total_content_bytes(),
        paths,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeletionAnalysis {
    deletable: Vec<DeletableFile>,
    already_absent: usize,
    conflicts: Vec<PatchDeletionConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeletableFile {
    path: PatchReplayDeletedFile,
    target: std::path::PathBuf,
}

fn analyze_deletions(root: &Path, deleted: &[PatchReplayDeletedFile]) -> Result<DeletionAnalysis> {
    let mut deletable = Vec::new();
    let mut already_absent = 0_usize;
    let mut conflicts = Vec::new();
    for deleted_file in deleted {
        let target = deleted_file.path.join_to_root(root);
        if !target.starts_with(root) {
            conflicts.push(PatchDeletionConflict {
                path: deleted_file.path.as_str().to_string(),
                reason: "target escapes repository root".to_string(),
            });
            continue;
        }
        let metadata = match fs::symlink_metadata(&target) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                already_absent += 1;
                continue;
            }
            Err(err) => return Err(err.into()),
        };
        if metadata.file_type().is_symlink() {
            conflicts.push(PatchDeletionConflict {
                path: deleted_file.path.as_str().to_string(),
                reason: "target is a symlink".to_string(),
            });
            continue;
        }
        if !metadata.is_file() {
            conflicts.push(PatchDeletionConflict {
                path: deleted_file.path.as_str().to_string(),
                reason: "target is not a regular file".to_string(),
            });
            continue;
        }
        let current = fs::read(&target)?;
        if current != deleted_file.old_bytes {
            conflicts.push(PatchDeletionConflict {
                path: deleted_file.path.as_str().to_string(),
                reason: format!(
                    "current file bytes do not match delete precondition blob {}",
                    deleted_file.old_blob_id
                ),
            });
            continue;
        }
        deletable.push(DeletableFile {
            path: deleted_file.clone(),
            target,
        });
    }
    Ok(DeletionAnalysis {
        deletable,
        already_absent,
        conflicts,
    })
}

fn apply_deletions(deletable: &[DeletableFile]) -> Result<usize> {
    let mut removed = 0_usize;
    for item in deletable {
        fs::remove_file(&item.target)?;
        sync_directory_best_effort(item.target.parent().unwrap_or_else(|| Path::new(".")))?;
        removed += 1;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests;
