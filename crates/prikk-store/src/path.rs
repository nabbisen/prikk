//! Compatibility imports for repository-relative path validation.

use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};
pub use prikk_replay::{RepoPath, validate_no_path_collisions, validate_repo_path};

/// Join a validated repository-relative path to a worktree/repository root.
pub(crate) fn join_repo_path_to_root(path: &RepoPath, root: &Path) -> PathBuf {
    let mut out = root.to_path_buf();
    for component in path.as_str().split('/') {
        out.push(component);
    }
    out
}

/// Convert a filesystem path into its canonical slash-separated repository-relative string form,
/// component by component. **Never use `Path::to_str()`/`Path::display()` directly for a path a
/// worktree walk builds with `Path::join`** — `join` inserts the platform separator, so on Windows
/// that string is backslash-joined, and every repository-path consumer downstream (`RepoPath::parse`,
/// an ignore rule, a tracked-path set built from `/`-joined baseline paths) either rejects it outright
/// or, worse, silently fails to match against something it should. This is the one converter both of
/// the crate's live-worktree walks (`worktree_status.rs`, `worktree_patch/node_authoring/worktree_files.rs`)
/// must go through — a third differently-shaped conversion is exactly the defect this function exists
/// to close off (RFC 124's re-land, after the mechanism's own first landing broke `commit` on Windows
/// by skipping this and calling `Path::to_str()` instead).
pub(crate) fn pathbuf_to_slash_string(path: &Path) -> Result<String> {
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
