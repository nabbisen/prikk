//! Repository-relative lexical path validation.
//!
//! Moved here from `prikk-replay` (DC-54): this is pure lexical grammar with no dependency on
//! `RepoPath` or lifecycle state, and object-envelope encoders need to call it without creating a
//! `prikk-object -> prikk-replay` dependency cycle (`prikk-replay` already depends on
//! `prikk-object`). `prikk-replay::RepoPath::parse` calls this function and re-exports it, so
//! every existing caller of `prikk_replay::validate_repo_path` / `prikk_store::validate_repo_path`
//! keeps compiling unchanged.

use prikk_error::{PrikkError, Result};

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

/// Whether a path component's basename (before the first `.`) is a Windows-reserved device name
/// (`CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`), checked case-insensitively and
/// regardless of host OS. Exposed for other storage surfaces that build a literal filesystem path
/// component from user-supplied text outside the `RepoPath` grammar (DC-72).
#[must_use]
pub fn is_windows_reserved_name(component: &str) -> bool {
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
