//! Filesystem utility helpers for storage operations.

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};

mod anchored;
mod contract;

// DC-71/DC-81: every test in both modules (including caller_tests' matrix submodules) sets up its
// scenario via real repository mutation, which is Linux/macOS-only; neither module ever compiles a
// test meaningful on any other platform.
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod caller_tests;
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests;

pub(crate) use anchored::{
    EntryKind, MutationRoot, RootFileStat, append_file_required, create_new_file_required,
    ensure_directory_required, inspect_entry, list_directory, promote_file_required,
    read_file_if_exists, read_file_required, remove_file_cleanup_best_effort,
    remove_file_if_present_required, remove_worktree_file_required, set_regular_file_mode_required,
    stat_file_state_if_exists, sync_directory_required, truncate_existing_file_required,
    truncate_file_empty_required, write_file_atomically, write_worktree_file_atomically,
};

// RFC 102 Stage 3, design-v1.md §12.3: G5 (`publish_immutable`) has no production caller left, but
// stays reachable for its own conformance tests (`object_store/tests/immutable.rs`, `races.rs`,
// `fsutil/tests.rs`) -- ruled "keep, record, decide separately" rather than retired as a stage side
// effect.
#[cfg(test)]
pub(crate) use anchored::publish_immutable_file;

#[cfg(all(test, target_os = "linux"))]
pub(crate) use anchored::LinuxDurability;
#[cfg(all(test, target_os = "macos"))]
pub(crate) use anchored::MacosDurability;
// DC-82: visible in test builds regardless of platform (`none`'s own gate), but only re-exported
// here where `fsutil::tests` — the only consumer — actually compiles.
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
pub(crate) use anchored::NoDurability;
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
pub(crate) use anchored::remove_file_required;
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
pub(crate) use contract::DurabilityContract;

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
pub(crate) use anchored::{TestFailPoint, fail_after_for_test, fail_once_for_test};

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
pub(crate) use anchored::{
    set_directory_create_barrier_for_test, set_immutable_install_barrier_for_test,
};

/// Return a process-unique temporary path next to the destination.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn temporary_path(path: &Path) -> Result<PathBuf> {
    let Some(file_name) = path.file_name() else {
        return Err(PrikkError::Io(
            "temporary path destination has no file name".to_string(),
        ));
    };
    let mut random = [0_u8; 16];
    getrandom::fill(&mut random)
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
