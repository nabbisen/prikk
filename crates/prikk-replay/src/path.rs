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
    ///
    /// # Examples
    ///
    /// A validated `RepoPath` carries its own string back out unchanged -- construction does not
    /// normalize case or separators, only refuse what is unsafe. Two paths differing only by case
    /// both construct successfully as distinct values; [`validate_no_path_collisions`] is the
    /// separate check that catches them being used together.
    ///
    /// ```
    /// use prikk_replay::RepoPath;
    ///
    /// let path = RepoPath::parse("src/lib.rs").unwrap();
    /// assert_eq!(path.as_str(), "src/lib.rs");
    ///
    /// assert!(RepoPath::parse("../outside").is_err());
    /// ```
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
///
/// # Examples
///
/// `README.md` and `readme.md` are two different, individually valid paths -- each passes
/// [`RepoPath::parse`] on its own. Committing both side by side is silently fine on a
/// case-sensitive filesystem (Linux) and silently *not the same two files* on a case-insensitive
/// one (macOS's and Windows' defaults) -- exactly the hazard this check exists to refuse before it
/// becomes a materialization surprise on a different platform than the one that authored it.
///
/// ```
/// use prikk_replay::{RepoPath, validate_no_path_collisions};
///
/// let distinct = [RepoPath::parse("a.txt").unwrap(), RepoPath::parse("b.txt").unwrap()];
/// assert!(validate_no_path_collisions(&distinct).is_ok());
///
/// let colliding = [RepoPath::parse("README.md").unwrap(), RepoPath::parse("readme.md").unwrap()];
/// assert!(
///     validate_no_path_collisions(&colliding).is_err(),
///     "paths differing only by case must be refused together"
/// );
/// ```
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
