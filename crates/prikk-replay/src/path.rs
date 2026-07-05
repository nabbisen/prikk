//! Repository-relative lexical path validation.
//!
//! This module owns the canonical repository-relative path value used by replay/lifecycle semantic
//! state. Filesystem layout, materialization policy, and worktree ownership remain in `prikk-store`.
//! The path subset is intentionally ASCII-only until Unicode NFC normalization is designed and
//! tested.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};

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

    /// Return a lexical filesystem path below `root`.
    #[must_use]
    pub fn join_to_root(&self, root: &Path) -> PathBuf {
        let mut out = root.to_path_buf();
        for component in self.0.split('/') {
            out.push(component);
        }
        out
    }
}

/// Validate that a path is safe as a repository-relative path.
pub fn validate_repo_path(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(PrikkError::InvalidName(
            "repository path must not be empty".to_string(),
        ));
    }
    if value.starts_with('/') {
        return Err(PrikkError::InvalidName(
            "absolute paths are not allowed".to_string(),
        ));
    }
    if value.contains('\\') {
        return Err(PrikkError::InvalidName(
            "backslashes are not allowed in repository paths".to_string(),
        ));
    }
    if value.contains(':') {
        return Err(PrikkError::InvalidName(
            "colon characters are not allowed in repository paths".to_string(),
        ));
    }
    if !value.is_ascii() {
        return Err(PrikkError::InvalidName(
            "non-ASCII paths are deferred until Unicode NFC normalization is implemented"
                .to_string(),
        ));
    }
    if value.bytes().any(|byte| byte < 0x20 || byte == 0x7f) {
        return Err(PrikkError::InvalidName(
            "control characters are not allowed in repository paths".to_string(),
        ));
    }
    for (index, component) in value.split('/').enumerate() {
        if index == 0 && component.eq_ignore_ascii_case(".prikk") {
            return Err(PrikkError::InvalidName(
                "repository paths must not target the .prikk metadata directory".to_string(),
            ));
        }
        validate_component(component)?;
    }
    Ok(())
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
        let folded_value = exact_value.to_ascii_lowercase();
        if !folded.insert(folded_value) {
            return Err(PrikkError::InvalidName(format!(
                "case-insensitive path collision involving: {exact_value}"
            )));
        }
    }
    Ok(())
}

fn validate_component(component: &str) -> Result<()> {
    if component.is_empty() {
        return Err(PrikkError::InvalidName(
            "empty path components are not allowed".to_string(),
        ));
    }
    if component == "." || component == ".." {
        return Err(PrikkError::InvalidName(
            "dot path components are not allowed".to_string(),
        ));
    }
    if component.ends_with(' ') || component.ends_with('.') {
        return Err(PrikkError::InvalidName(
            "path components must not end with space or dot".to_string(),
        ));
    }
    if is_windows_reserved_name(component) {
        return Err(PrikkError::InvalidName(format!(
            "Windows reserved path component is not allowed: {component}"
        )));
    }
    Ok(())
}

fn is_windows_reserved_name(component: &str) -> bool {
    let base = component
        .split('.')
        .next()
        .unwrap_or(component)
        .to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || matches!(
            base.as_str(),
            "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9"
        )
        || matches!(
            base.as_str(),
            "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9"
        )
}
