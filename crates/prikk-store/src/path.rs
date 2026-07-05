//! Compatibility imports for repository-relative path validation.

use std::path::{Path, PathBuf};

pub use prikk_replay::{RepoPath, validate_no_path_collisions, validate_repo_path};

/// Join a validated repository-relative path to a worktree/repository root.
pub(crate) fn join_repo_path_to_root(path: &RepoPath, root: &Path) -> PathBuf {
    let mut out = root.to_path_buf();
    for component in path.as_str().split('/') {
        out.push(component);
    }
    out
}

#[cfg(test)]
mod tests;
