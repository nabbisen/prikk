//! Repository-relative lexical path validation.
//!
//! This module owns the canonical repository-relative path value used by replay/lifecycle semantic
//! state. Filesystem layout, materialization policy, and worktree ownership remain in `prikk-store`.
//! The path subset is intentionally ASCII-only until Unicode NFC normalization is designed and
//! tested.
//!
//! The lexical grammar itself (`validate_repo_path`) moved to `prikk-object` (DC-54): object
//! envelope encoders need to call it without creating a `prikk-object -> prikk-replay` dependency
//! cycle. Re-exported here so every existing `prikk_replay::validate_repo_path` caller keeps
//! compiling unchanged.

use std::collections::BTreeSet;

use prikk_error::{PrikkError, Result};

use prikk_object::ascii_fold;
pub use prikk_object::validate_repo_path;

/// A validated repository-relative path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepoPath(String);

impl RepoPath {
    /// Validate and construct a repository-relative path.
    pub fn parse(value: &str) -> Result<Self> {
        validate_repo_path(value)?;
        Ok(Self(value.to_string()))
    }

    /// Return the canonical slash-separated representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Reject duplicate paths and case-insensitive collisions.
pub fn validate_no_path_collisions(paths: &[RepoPath]) -> Result<()> {
    let mut exact = BTreeSet::<&str>::new();
    let mut folded = BTreeSet::<String>::new();
    for path in paths {
        let exact_value = path.as_str();
        if !exact.insert(exact_value) {
            return Err(PrikkError::InvalidName(format!(
                "duplicate repository path: {exact_value}"
            )));
        }
        let folded_value = ascii_fold(exact_value);
        if !folded.insert(folded_value) {
            return Err(PrikkError::InvalidName(format!(
                "case-insensitive path collision involving: {exact_value}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
