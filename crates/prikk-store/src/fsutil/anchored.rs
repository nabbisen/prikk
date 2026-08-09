//! Root-scoped filesystem mutation primitives. Every function here is a thin call through the
//! durability contract (DC-76, `super::contract::DurabilityContract`) — the guarantee each one
//! provides is stated on the trait method it calls, not repeated here. `Linux` is the sole
//! implementor (`linux::LinuxDurability`); no `target_os` gate is relaxed by this indirection.

use std::path::Path;

use prikk_error::{PrikkError, Result};

mod directory;
#[cfg(target_os = "linux")]
mod failpoints;
mod immutable;
#[cfg(target_os = "linux")]
mod linux;
mod read;
mod regular;

pub(crate) use directory::MutationRoot;
pub(crate) use read::{
    EntryKind, RootFileStat, inspect_entry, list_directory, read_file_if_exists,
    read_file_required, stat_file_state_if_exists,
};

#[cfg(target_os = "linux")]
use crate::fsutil::contract::DurabilityContract;
#[cfg(target_os = "linux")]
pub(crate) use linux::LinuxDurability;

#[cfg(all(test, target_os = "linux"))]
pub(crate) use failpoints::{
    Point as TestFailPoint, fail_after as fail_after_for_test, fail_once as fail_once_for_test,
    set_directory_create_barrier as set_directory_create_barrier_for_test,
    set_immutable_install_barrier as set_immutable_install_barrier_for_test,
};

/// Write mutable metadata through a unique same-directory temporary file.
pub(crate) fn write_file_atomically(
    root: &MutationRoot,
    relative: &Path,
    bytes: &[u8],
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        LinuxDurability.atomic_replace(root, relative, bytes)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative, bytes);
        unsupported_mutation()
    }
}

/// Write a worktree file through its retained worktree-root authority.
pub(crate) fn write_worktree_file_atomically(
    root: &MutationRoot,
    relative: &Path,
    bytes: &[u8],
) -> Result<()> {
    write_file_atomically(root, relative, bytes)
}

/// Append bytes, sync the file, and always re-establish retained-parent durability.
pub(crate) fn append_file_required(
    root: &MutationRoot,
    relative: &Path,
    bytes: &[u8],
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        LinuxDurability.durable_append(root, relative, bytes)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative, bytes);
        unsupported_mutation()
    }
}

/// Truncate an existing regular file to a retained length and sync its parent.
pub(crate) fn truncate_existing_file_required(
    root: &MutationRoot,
    relative: &Path,
    len: u64,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        LinuxDurability.durable_truncate(root, relative, len)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative, len);
        unsupported_mutation()
    }
}

/// Set an existing regular file's mode bits (DC-73: worktree materialization needs to write the
/// mode a `CreateFile`/`ChangePerm` operation recorded, not whatever the anchored create primitive
/// defaults new files to). No-follow, matching every other anchored open — a symlink at the final
/// component is refused rather than chmod'd through.
pub(crate) fn set_regular_file_mode_required(
    root: &MutationRoot,
    relative: &Path,
    mode: u32,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        LinuxDurability.set_permission_bits(root, relative, mode)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative, mode);
        unsupported_mutation()
    }
}

/// Create or truncate a regular file, then sync it and its retained parent.
pub(crate) fn truncate_file_empty_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        LinuxDurability.durable_truncate_to_empty(root, relative)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative);
        unsupported_mutation()
    }
}

/// Create, write, and durably publish an exclusive regular file.
pub(crate) fn create_new_file_required(
    root: &MutationRoot,
    relative: &Path,
    bytes: &[u8],
) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        LinuxDurability.create_exclusive(root, relative, bytes)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative, bytes);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "repository mutation requires Linux anchored filesystem primitives",
        ))
    }
}

/// Remove a file and sync the exact parent handle that owned the unlink.
pub(crate) fn remove_file_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    remove_file_if_present_required(root, relative).map(|_| ())
}

/// Remove a file if present and sync its exact parent even for observed absence.
pub(crate) fn remove_file_if_present_required(
    root: &MutationRoot,
    relative: &Path,
) -> Result<bool> {
    #[cfg(target_os = "linux")]
    {
        LinuxDurability.remove_if_present(root, relative)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative);
        unsupported_mutation()
    }
}

/// Attempt fallible removal where a destructor cannot report the result.
pub(crate) fn remove_file_cleanup_best_effort(root: &MutationRoot, relative: &Path) {
    let _ = remove_file_required(root, relative);
}

/// Remove a worktree file through its retained worktree-root authority.
pub(crate) fn remove_worktree_file_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    remove_file_required(root, relative)
}

/// Rename within one root, syncing destination before source.
pub(crate) fn promote_file_required(
    root: &MutationRoot,
    source: &Path,
    destination: &Path,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        LinuxDurability.promote(root, source, destination)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, source, destination);
        unsupported_mutation()
    }
}

/// Publish immutable, content-addressed bytes at `relative` without ever replacing existing
/// content — see `DurabilityContract::publish_immutable` for the guarantee.
pub(crate) fn publish_immutable_file(
    root: &MutationRoot,
    relative: &Path,
    candidate: &[u8],
    validate_existing: impl Fn(&[u8]) -> Result<()>,
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        LinuxDurability.publish_immutable(root, relative, candidate, validate_existing)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative, candidate, validate_existing);
        unsupported_mutation()
    }
}

/// Ensure a relative directory tree exists, durably, tolerating a concurrent creator (G8).
pub(crate) fn ensure_directory_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        LinuxDurability.ensure_directory(root, relative)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative);
        unsupported_mutation()
    }
}

/// Open and required-sync an existing root-relative directory.
pub(crate) fn sync_directory_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        LinuxDurability.durable_directory_entry(root, relative)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative);
        unsupported_mutation()
    }
}

#[cfg(target_os = "linux")]
fn io_error(error: rustix::io::Errno) -> PrikkError {
    PrikkError::from(std::io::Error::from(error))
}

#[cfg(target_os = "linux")]
fn prikk_to_io(error: PrikkError) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

#[cfg(not(target_os = "linux"))]
fn unsupported_mutation<T>() -> Result<T> {
    Err(PrikkError::Io(
        "repository mutation requires Linux root-scoped filesystem capabilities".to_string(),
    ))
}
