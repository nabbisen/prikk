//! Nonblocking final-entry opens and same-handle validation.

use std::path::Path;

use prikk_error::{PrikkError, Result};

use super::{failpoints, io_error};

#[cfg(target_os = "linux")]
use rustix::fd::{AsFd, OwnedFd};
#[cfg(target_os = "linux")]
use rustix::fs::{self, FileType, Mode, OFlags};

pub(super) fn required_parent(path: &Path) -> Result<&Path> {
    path.parent()
        .ok_or_else(|| PrikkError::Io("relative mutation path has no parent".to_string()))
}

pub(super) fn required_file_name(path: &Path) -> Result<&std::ffi::OsStr> {
    path.file_name()
        .ok_or_else(|| PrikkError::Io("relative mutation path has no file name".to_string()))
}

#[cfg(target_os = "linux")]
pub(super) fn open_new_regular(
    directory: impl AsFd,
    name: &std::ffi::OsStr,
) -> rustix::io::Result<OwnedFd> {
    failpoints::required_open().map_err(|_| rustix::io::Errno::IO)?;
    fs::openat(
        directory,
        name,
        OFlags::WRONLY
            | OFlags::CREATE
            | OFlags::EXCL
            | OFlags::NONBLOCK
            | OFlags::NOFOLLOW
            | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
}

#[cfg(target_os = "linux")]
pub(super) fn open_existing_regular(
    directory: impl AsFd,
    name: &std::ffi::OsStr,
    flags: OFlags,
) -> Result<OwnedFd> {
    failpoints::required_open()?;
    let fd = fs::openat(
        directory,
        name,
        flags | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(io_error)?;
    validate_regular(&fd)?;
    Ok(fd)
}

#[cfg(target_os = "linux")]
pub(super) fn open_existing_or_create_regular(
    directory: impl AsFd,
    name: &std::ffi::OsStr,
    flags: OFlags,
) -> Result<OwnedFd> {
    failpoints::required_open()?;
    match fs::openat(
        directory.as_fd(),
        name,
        flags | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => {
            validate_regular(&fd)?;
            Ok(fd)
        }
        Err(rustix::io::Errno::NOENT) => open_new_regular(directory, name).map_err(io_error),
        Err(error) => Err(io_error(error)),
    }
}

#[cfg(target_os = "linux")]
pub(super) fn open_append_regular(directory: impl AsFd, name: &std::ffi::OsStr) -> Result<OwnedFd> {
    match open_new_regular(directory.as_fd(), name) {
        Ok(fd) => Ok(fd),
        Err(rustix::io::Errno::EXIST) => {
            open_existing_regular(directory, name, OFlags::WRONLY | OFlags::APPEND)
        }
        Err(error) => Err(io_error(error)),
    }
}

#[cfg(target_os = "linux")]
fn validate_regular(fd: impl AsFd) -> Result<()> {
    let stat = fs::fstat(fd).map_err(io_error)?;
    if FileType::from_raw_mode(stat.st_mode).is_file() {
        return Ok(());
    }
    Err(PrikkError::Integrity(
        "mutation target is not a regular file".to_string(),
    ))
}
