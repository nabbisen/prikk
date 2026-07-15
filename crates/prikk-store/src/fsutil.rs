//! Filesystem utility helpers for storage operations.

use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};

mod anchored;

#[cfg(test)]
mod caller_tests;
#[cfg(test)]
mod tests;

pub(crate) use anchored::{
    EntryKind, MutationRoot, append_file_required, create_new_file_required,
    ensure_directory_required, inspect_entry, list_directory, promote_file_required,
    read_file_if_exists, read_file_required, read_file_state_if_exists,
    remove_file_cleanup_best_effort, remove_file_if_present_required,
    remove_worktree_file_required, sync_directory_required, truncate_existing_file_required,
    truncate_file_empty_required, write_file_atomically, write_worktree_file_atomically,
};

#[cfg(test)]
pub(crate) use anchored::remove_file_required;

#[cfg(test)]
pub(crate) use anchored::{TestFailPoint, fail_after_for_test, fail_once_for_test};

/// Return a process-unique temporary path next to the destination.
pub(crate) fn temporary_path(path: &Path) -> Result<PathBuf> {
    let Some(file_name) = path.file_name() else {
        return Err(PrikkError::Io(
            "temporary path destination has no file name".to_string(),
        ));
    };
    let mut random = [0_u8; 16];
    getrandom::getrandom(&mut random)
        .map_err(|error| PrikkError::Io(format!("temporary path randomness failed: {error}")))?;
    let mut name = file_name.to_os_string();
    name.push(format!(
        ".tmp.{}.{:032x}",
        std::process::id(),
        u128::from_le_bytes(random)
    ));
    Ok(path.with_file_name(name))
}

/// Convert a usize length to u16.
pub(crate) fn len_to_u16(len: usize) -> Result<u16> {
    u16::try_from(len).map_err(|_| PrikkError::MalformedData("length exceeds u16".to_string()))
}

/// Convert a usize length to u32.
pub(crate) fn len_to_u32(len: usize) -> Result<u32> {
    u32::try_from(len).map_err(|_| PrikkError::MalformedData("length exceeds u32".to_string()))
}

/// Convert a usize length to u64.
pub(crate) fn len_to_u64(len: usize) -> Result<u64> {
    u64::try_from(len).map_err(|_| PrikkError::MalformedData("length exceeds u64".to_string()))
}
