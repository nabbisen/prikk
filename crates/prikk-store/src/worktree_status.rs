//! Read-only worktree status against a snapshot baseline.
//!
//! PR-019 compares the current worktree with the snapshot manifest referenced by a published block.
//! It is intentionally read-only and does not create patch operations yet.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use prikk_error::{PrikkError, Result};
use prikk_object::{BlobPayload, ObjectType};

use crate::checkout::prepare_snapshot_checkout_plan;
use crate::layout::RepositoryLayout;
use crate::object_store::{FileObjectStore, ObjectReader};
use crate::path::RepoPath;
use crate::snapshot::SnapshotManifest;

/// Read-only worktree status report against a snapshot baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeStatusReport {
    /// Human-readable ref name used as the baseline.
    pub ref_name: String,
    /// Number of tracked files in the snapshot baseline.
    pub tracked_files: usize,
    /// Number of tracked files that match the baseline bytes.
    pub unchanged_files: usize,
    /// Worktree changes detected against the baseline.
    pub changes: Vec<WorktreeChange>,
}

impl WorktreeStatusReport {
    /// Return true when the worktree has no detected changes.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.changes.is_empty()
    }

    /// Count changes by kind.
    #[must_use]
    pub fn count_kind(&self, kind: WorktreeChangeKind) -> usize {
        self.changes.iter().filter(|change| change.kind == kind).count()
    }
}

/// A single worktree change detected by the read-only status scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeChange {
    /// Repository-relative path, when it could be represented safely.
    pub path: String,
    /// Change kind.
    pub kind: WorktreeChangeKind,
    /// Short explanation intended for CLI display.
    pub detail: String,
}

/// Worktree change kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeChangeKind {
    /// A tracked snapshot file is missing from the worktree.
    Missing,
    /// A tracked snapshot file exists but differs from the baseline bytes.
    Modified,
    /// A worktree file is not present in the snapshot baseline.
    Untracked,
    /// A worktree path could not be safely represented as a PRIKK repo path.
    UnsupportedPath,
}

impl WorktreeChangeKind {
    /// Stable CLI label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Modified => "modified",
            Self::Untracked => "untracked",
            Self::UnsupportedPath => "unsupported-path",
        }
    }
}

/// Compute read-only worktree status against the snapshot referenced by a ref.
pub fn worktree_status(layout: &RepositoryLayout, ref_name: &str) -> Result<WorktreeStatusReport> {
    let plan = prepare_snapshot_checkout_plan(layout, ref_name)?;
    let manifest = load_snapshot_manifest(layout, plan.snapshot_blob_id)?;
    let baseline_paths: BTreeSet<String> = manifest
        .files
        .iter()
        .map(|entry| entry.path.as_str().to_string())
        .collect();
    let mut seen_paths = BTreeSet::new();
    let mut changes = Vec::new();
    let mut unchanged_files = 0_usize;

    for entry in &manifest.files {
        let path_text = entry.path.as_str().to_string();
        let target = entry.path.join_to_root(layout.root());
        seen_paths.insert(path_text.clone());
        if !target.exists() {
            changes.push(WorktreeChange {
                path: path_text,
                kind: WorktreeChangeKind::Missing,
                detail: "tracked snapshot file is absent from the worktree".to_string(),
            });
            continue;
        }
        let metadata = fs::symlink_metadata(&target)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            changes.push(WorktreeChange {
                path: path_text,
                kind: WorktreeChangeKind::Modified,
                detail: "tracked path is not a regular file".to_string(),
            });
            continue;
        }
        let bytes = fs::read(&target)?;
        if bytes == entry.bytes {
            unchanged_files += 1;
        } else {
            changes.push(WorktreeChange {
                path: path_text,
                kind: WorktreeChangeKind::Modified,
                detail: "tracked file bytes differ from the snapshot baseline".to_string(),
            });
        }
    }

    scan_untracked(layout.root(), layout.root(), &baseline_paths, &seen_paths, &mut changes)?;
    changes.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.kind.as_str().cmp(right.kind.as_str()))
    });

    Ok(WorktreeStatusReport {
        ref_name: ref_name.to_string(),
        tracked_files: manifest.files.len(),
        unchanged_files,
        changes,
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
    let blob = BlobPayload::decode_canonical(&envelope.canonical_payload)?;
    SnapshotManifest::decode(&blob.bytes)
}

fn scan_untracked(
    root: &Path,
    current: &Path,
    baseline_paths: &BTreeSet<String>,
    seen_paths: &BTreeSet<String>,
    changes: &mut Vec<WorktreeChange>,
) -> Result<()> {
    let entries = match fs::read_dir(current) {
        Ok(entries) => entries,
        Err(err) => return Err(PrikkError::Io(err.to_string())),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if is_prikk_metadata_path(root, &path) {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            scan_untracked(root, &path, baseline_paths, seen_paths, changes)?;
            continue;
        }
        let repo_path = path_to_repo_string(root, &path);
        match repo_path.and_then(|text| RepoPath::parse(&text).map(|_| text)) {
            Ok(text) => {
                if !baseline_paths.contains(&text) && !seen_paths.contains(&text) {
                    changes.push(WorktreeChange {
                        path: text,
                        kind: WorktreeChangeKind::Untracked,
                        detail: "worktree file is not in the snapshot baseline".to_string(),
                    });
                }
            }
            Err(err) => {
                changes.push(WorktreeChange {
                    path: path.display().to_string(),
                    kind: WorktreeChangeKind::UnsupportedPath,
                    detail: format!(
                        "worktree path is not representable as a safe PRIKK path: {err}"
                    ),
                });
            }
        }
    }
    Ok(())
}

fn is_prikk_metadata_path(root: &Path, path: &Path) -> bool {
    match path.strip_prefix(root) {
        Ok(relative) => first_component_is_prikk(relative),
        Err(_) => false,
    }
}

fn first_component_is_prikk(relative: &Path) -> bool {
    let Some(first) = relative.components().next() else {
        return false;
    };
    first.as_os_str().to_str() == Some(".prikk")
}

fn path_to_repo_string(root: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(root).map_err(|_| {
        PrikkError::Integrity(format!(
            "worktree path escaped repository root: {}",
            path.display()
        ))
    })?;
    pathbuf_to_slash_string(relative)
}

fn pathbuf_to_slash_string(path: &Path) -> Result<String> {
    let mut components = Vec::new();
    for component in path.components() {
        let text = component.as_os_str().to_str().ok_or_else(|| {
            PrikkError::Integrity(format!("worktree path is not UTF-8: {}", path.display()))
        })?;
        components.push(text.to_string());
    }
    if components.is_empty() {
        return Err(PrikkError::Integrity("empty worktree path".to_string()));
    }
    Ok(components.join("/"))
}
