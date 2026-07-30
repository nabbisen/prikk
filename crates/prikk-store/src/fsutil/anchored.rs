//! Root-scoped filesystem mutation primitives.

use std::fs::File;
use std::io::Write;
use std::path::Path;

use prikk_error::{PrikkError, Result};

use super::temporary_path;

mod directory;
mod failpoints;
mod immutable;
mod read;
mod regular;

pub(crate) use directory::{MutationRoot, ensure_directory_required, sync_directory_required};
#[cfg(target_os = "linux")]
use directory::{open_existing_directory_required, prepare_directory_required};
pub(crate) use immutable::publish_immutable_file;
pub(crate) use read::{
    EntryKind, RootFileStat, inspect_entry, list_directory, read_file_if_exists,
    read_file_required, stat_file_state_if_exists,
};
#[cfg(target_os = "linux")]
use regular::{
    open_append_regular, open_existing_or_create_regular, open_existing_regular, open_new_regular,
};
use regular::{required_file_name, required_parent};

#[cfg(test)]
pub(crate) use failpoints::{
    Point as TestFailPoint, fail_after as fail_after_for_test, fail_once as fail_once_for_test,
    set_directory_create_barrier as set_directory_create_barrier_for_test,
    set_immutable_install_barrier as set_immutable_install_barrier_for_test,
};

#[cfg(target_os = "linux")]
use rustix::fs::{self, OFlags};

/// Write mutable metadata through a unique same-directory temporary file.
pub(crate) fn write_file_atomically(
    root: &MutationRoot,
    relative: &Path,
    bytes: &[u8],
) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let parent = required_parent(relative)?;
        let destination = required_file_name(relative)?;
        let directory = prepare_directory_required(root, parent)?;
        let temp_path = temporary_path(relative)?;
        let temp_name = required_file_name(&temp_path)?;
        let fd = open_new_regular(&directory.fd, temp_name).map_err(io_error)?;
        let mut file = File::from(fd);
        file.write_all(bytes)?;
        failpoints::mutable_file_sync()?;
        file.sync_all()?;
        drop(file);
        failpoints::mutable_rename()?;
        fs::renameat(&directory.fd, temp_name, &directory.fd, destination).map_err(io_error)?;
        failpoints::mutable_parent_sync()?;
        directory.sync()
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
        let directory = open_existing_directory_required(root, required_parent(relative)?)?;
        let name = required_file_name(relative)?;
        let fd = open_append_regular(&directory.fd, name)?;
        let mut file = File::from(fd);
        failpoints::append_write()?;
        file.write_all(bytes)?;
        failpoints::required_file_sync()?;
        file.sync_all()?;
        failpoints::required_directory_sync()?;
        directory.sync()
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
        let directory = open_existing_directory_required(root, required_parent(relative)?)?;
        let fd =
            open_existing_regular(&directory.fd, required_file_name(relative)?, OFlags::WRONLY)?;
        let file = File::from(fd);
        failpoints::truncate()?;
        file.set_len(len)?;
        failpoints::required_file_sync()?;
        file.sync_all()?;
        failpoints::required_directory_sync()?;
        directory.sync()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative, len);
        unsupported_mutation()
    }
}

/// Create or truncate a regular file, then sync it and its retained parent.
pub(crate) fn truncate_file_empty_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let directory = prepare_directory_required(root, required_parent(relative)?)?;
        let fd = open_existing_or_create_regular(
            &directory.fd,
            required_file_name(relative)?,
            OFlags::WRONLY,
        )?;
        let file = File::from(fd);
        failpoints::truncate()?;
        file.set_len(0)?;
        failpoints::required_file_sync()?;
        file.sync_all()?;
        failpoints::required_directory_sync()?;
        directory.sync()
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
        let parent = required_parent(relative).map_err(prikk_to_io)?;
        let directory = prepare_directory_required(root, parent).map_err(prikk_to_io)?;
        let fd = open_new_regular(
            &directory.fd,
            required_file_name(relative).map_err(prikk_to_io)?,
        )
        .map_err(std::io::Error::from)?;
        let mut file = File::from(fd);
        file.write_all(bytes)?;
        failpoints::required_file_sync().map_err(prikk_to_io)?;
        file.sync_all()?;
        failpoints::required_directory_sync().map_err(prikk_to_io)?;
        directory.sync().map_err(prikk_to_io)
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
        let directory = open_existing_directory_required(root, required_parent(relative)?)?;
        failpoints::unlink()?;
        let removed = match fs::unlinkat(
            &directory.fd,
            required_file_name(relative)?,
            fs::AtFlags::empty(),
        ) {
            Ok(()) => true,
            Err(rustix::io::Errno::NOENT) => false,
            Err(error) => return Err(io_error(error)),
        };
        failpoints::cleanup_directory_sync()?;
        directory.sync()?;
        Ok(removed)
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
        let source_dir = open_existing_directory_required(root, required_parent(source)?)?;
        let destination_dir =
            open_existing_directory_required(root, required_parent(destination)?)?;
        failpoints::promotion_rename()?;
        fs::renameat(
            &source_dir.fd,
            required_file_name(source)?,
            &destination_dir.fd,
            required_file_name(destination)?,
        )
        .map_err(io_error)?;
        failpoints::promotion_destination_sync()?;
        destination_dir.sync()?;
        failpoints::promotion_source_sync()?;
        source_dir.sync()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, source, destination);
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
