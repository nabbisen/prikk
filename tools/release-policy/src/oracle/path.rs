use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

pub(crate) fn lexical(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && value.split('/').all(|part| {
            !matches!(part, "" | "." | "..")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
        })
}

pub(crate) fn repository_file(root: &Path, value: &str) -> Result<PathBuf> {
    if !lexical(value) {
        return Err(Error::new(format!("manifest-contract:path:{value}")));
    }
    let candidate = root.join(value);
    let resolved = candidate
        .canonicalize()
        .map_err(|_| Error::new(format!("input-identity:missing:{value}")))?;
    let resolved_root = root.canonicalize()?;
    if !resolved.starts_with(&resolved_root) {
        return Err(Error::new(format!("input-identity:outside-root:{value}")));
    }
    if !fs::metadata(&resolved)?.is_file() {
        return Err(Error::new(format!("input-identity:not-regular:{value}")));
    }
    Ok(candidate)
}

#[cfg(test)]
#[path = "path/tests.rs"]
mod tests;
