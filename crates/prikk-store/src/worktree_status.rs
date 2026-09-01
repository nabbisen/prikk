//! Read-only worktree status against the replay baseline.
//!
//! RFC 122 (`replay-baseline-handoff-v1.md`) rewired this off the pre-node snapshot-blob baseline
//! `commit` left behind at the patch-replay migration (`patch_replay.rs`): every real repository
//! this CLI can create is authored against node-addressed replay state, never a stored snapshot
//! Blob, so requiring one here (as this module did before) refused on every such repository. This
//! module now shares `commit`'s own baseline derivation
//! (`patch_replay::resolve_folded_worktree_baseline`) rather than reconstructing it a second way —
//! see that function's own doc comment for why a second implementation that merely agrees today is
//! itself the defect class being fixed.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use prikk_error::{PrikkError, Result};

use crate::blob_access::ensure_blob_matches_node_kind;
use crate::layout::{DEFAULT_ACTIVE_NAME, RepositoryLayout};
use crate::lifecycle_cache::replay::TextCache;
use crate::node_lifecycle::NodeContent;
use crate::object_store::ObjectReadSnapshot;
use crate::patch_replay::resolve_folded_worktree_baseline;
use crate::path::{RepoPath, join_repo_path_to_root};
use crate::wal::Wal;

/// Read-only worktree status report against the replay baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeStatusReport {
    /// Human-readable ref name used as the baseline.
    pub ref_name: String,
    /// Number of tracked files in the baseline.
    pub tracked_files: usize,
    /// Number of tracked files that match the baseline bytes.
    pub unchanged_files: usize,
    /// Worktree changes detected against the baseline.
    pub changes: Vec<WorktreeChange>,
    /// `Some(other_ref)` when the active WAL is non-empty but owned by a ref other than
    /// `ref_name` — real, committed-but-unsealed work, not part of this ref's baseline (RFC 122
    /// `replay-baseline-handoff-v2-amendment.md` §4). Any `Untracked` changes above are reported
    /// relative to `ref_name`'s own baseline regardless — this field adds context, it does not
    /// reclassify them: a queued file for a *different* ref is not part of that other ref's
    /// baseline either, so it correctly still shows as untracked here.
    pub queued_elsewhere: Option<String>,
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
        self.changes
            .iter()
            .filter(|change| change.kind == kind)
            .count()
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
    /// A tracked file is missing from the worktree.
    Missing,
    /// A tracked file exists but differs from the baseline bytes.
    Modified,
    /// A worktree file is not present in the baseline.
    Untracked,
    /// A worktree path could not be safely represented as a Prikk repo path.
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

/// Compute read-only worktree status against the replay baseline for `ref_name` — the same
/// baseline `commit` would author against, already-queued (unsealed) patches folded on top
/// (`resolve_folded_worktree_baseline`'s own doc comment explains why folding is the right choice
/// here: it answers "what would the next commit author?", which is the question this command's own
/// caller is actually asking, and it agrees with `commit` by construction rather than by
/// coincidence).
pub fn worktree_status(layout: &RepositoryLayout, ref_name: &str) -> Result<WorktreeStatusReport> {
    let object_store = ObjectReadSnapshot::open(layout)?;
    let wal = Wal::for_layout(layout, DEFAULT_ACTIVE_NAME);
    let active_replay = wal.replay()?;
    if active_replay.trailing_partial_bytes != 0 {
        return Err(PrikkError::Integrity(format!(
            "active WAL has {} trailing partial bytes; run `prikk doctor --repair-wal-tail` \
             before checking worktree status",
            active_replay.trailing_partial_bytes
        )));
    }
    if active_replay.has_item_failure() {
        return Err(PrikkError::Integrity(
            "active WAL has a damaged record; run doctor before checking worktree status"
                .to_string(),
        ));
    }
    let mut text_cache = TextCache::new();
    let resolved = resolve_folded_worktree_baseline(
        layout,
        &object_store,
        ref_name,
        &active_replay,
        &mut text_cache,
    )?;

    let mut baseline_paths = BTreeSet::new();
    let mut seen_paths = BTreeSet::new();
    let mut changes = Vec::new();
    let mut unchanged_files = 0_usize;
    let mut tracked_files = 0_usize;

    for (_, node) in resolved.state.live_nodes() {
        // Symlink nodes carry no file-content blob to compare (`ensure_blob_matches_node_kind`
        // refuses one outright) — no current authoring path creates one anyway (module doc: "symlink
        // authoring fails closed"), and the snapshot-manifest baseline this replaced never carried
        // symlinks either, so this is the same, pre-existing blind spot, not a new one.
        let NodeContent::File { blob_id, .. } = &node.content else {
            continue;
        };
        tracked_files += 1;
        let path_text = node.path.as_str().to_string();
        baseline_paths.insert(path_text.clone());
        seen_paths.insert(path_text.clone());
        let target = join_repo_path_to_root(&node.path, layout.root());
        if !target.exists() {
            changes.push(WorktreeChange {
                path: path_text,
                kind: WorktreeChangeKind::Missing,
                detail: "tracked file is absent from the worktree".to_string(),
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
        if ensure_blob_matches_node_kind(&bytes, *blob_id, node.kind).is_ok() {
            unchanged_files += 1;
        } else {
            changes.push(WorktreeChange {
                path: path_text,
                kind: WorktreeChangeKind::Modified,
                detail: "tracked file bytes differ from the baseline".to_string(),
            });
        }
    }

    scan_untracked(
        layout.root(),
        layout.root(),
        &baseline_paths,
        &seen_paths,
        &mut changes,
    )?;
    changes.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.kind.as_str().cmp(right.kind.as_str()))
    });

    Ok(WorktreeStatusReport {
        ref_name: ref_name.to_string(),
        tracked_files,
        unchanged_files,
        changes,
        queued_elsewhere: resolved.queued_on_other_ref,
    })
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
                        detail: "worktree file is not in the baseline".to_string(),
                    });
                }
            }
            Err(err) => {
                changes.push(WorktreeChange {
                    path: path.display().to_string(),
                    kind: WorktreeChangeKind::UnsupportedPath,
                    detail: format!(
                        "worktree path is not representable as a safe Prikk path: {err}"
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

#[cfg(test)]
mod tests;
