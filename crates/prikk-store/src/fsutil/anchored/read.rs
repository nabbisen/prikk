//! Root-relative reads used by mutation-authorizing workflows.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::io::Read;

use prikk_error::{PrikkError, Result};

use super::MutationRoot;
#[cfg(target_os = "linux")]
use super::directory::open_existing_directory_for_read;
#[cfg(target_os = "linux")]
use super::{failpoints, io_error, required_file_name, required_parent};

#[cfg(target_os = "linux")]
use rustix::fs::{self, AtFlags, FileType, Mode, OFlags};

/// No-follow classification of a root-relative final entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryKind {
    Regular,
    Directory,
    Symlink,
    Other,
}

/// Size, modification time, and raw mode of one validated regular-file entry, gathered without
/// opening or reading its content (DC-56: the commit-index cache consults this to decide whether a
/// content read can be skipped).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RootFileStat {
    pub(crate) size: u64,
    pub(crate) mtime_secs: i64,
    pub(crate) mtime_nanos: u32,
    pub(crate) mode: u32,
}

/// One no-follow root-relative directory entry.
pub(crate) struct RootDirEntry {
    pub(crate) name: OsString,
    pub(crate) kind: EntryKind,
}

/// Read a regular file's bytes, returning `None` only when a path component is absent.
pub(crate) fn read_file_if_exists(root: &MutationRoot, relative: &Path) -> Result<Option<Vec<u8>>> {
    #[cfg(target_os = "linux")]
    {
        let Some(directory) = open_existing_directory_for_read(root, required_parent(relative)?)?
        else {
            return Ok(None);
        };
        failpoints::required_open()?;
        let fd = match fs::openat(
            &directory.fd,
            required_file_name(relative)?,
            OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(error) => return Err(io_error(error)),
        };
        let stat = fs::fstat(&fd).map_err(io_error)?;
        if !FileType::from_raw_mode(stat.st_mode).is_file() {
            return Err(PrikkError::Integrity(
                "read target is not a regular file".to_string(),
            ));
        }
        let mut bytes = Vec::new();
        File::from(fd).read_to_end(&mut bytes)?;
        Ok(Some(bytes))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let path = root.fallback_path(relative)?;
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

/// Stat a regular file's size, mtime, and mode without opening or reading its content.
pub(crate) fn stat_file_state_if_exists(
    root: &MutationRoot,
    relative: &Path,
) -> Result<Option<RootFileStat>> {
    #[cfg(target_os = "linux")]
    {
        let Some(directory) = open_existing_directory_for_read(root, required_parent(relative)?)?
        else {
            return Ok(None);
        };
        let stat = match fs::statat(
            &directory.fd,
            required_file_name(relative)?,
            AtFlags::SYMLINK_NOFOLLOW,
        ) {
            Ok(stat) => stat,
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(error) => return Err(io_error(error)),
        };
        if !FileType::from_raw_mode(stat.st_mode).is_file() {
            return Err(PrikkError::Integrity(
                "stat target is not a regular file".to_string(),
            ));
        }
        Ok(Some(RootFileStat {
            size: u64::try_from(stat.st_size).unwrap_or_default(),
            mtime_secs: stat.st_mtime,
            mtime_nanos: u32::try_from(stat.st_mtime_nsec).unwrap_or_default(),
            mode: stat.st_mode,
        }))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let path = root.fallback_path(relative)?;
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.is_file() {
                    return Err(PrikkError::Integrity(
                        "stat target is not a regular file".to_string(),
                    ));
                }
                #[cfg(unix)]
                let (mode, mtime_secs, mtime_nanos) = {
                    use std::os::unix::fs::MetadataExt;
                    (
                        metadata.mode(),
                        metadata.mtime(),
                        u32::try_from(metadata.mtime_nsec()).unwrap_or_default(),
                    )
                };
                #[cfg(not(unix))]
                let (mode, mtime_secs, mtime_nanos) = (0_u32, 0_i64, 0_u32);
                Ok(Some(RootFileStat {
                    size: metadata.len(),
                    mtime_secs,
                    mtime_nanos,
                    mode,
                }))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

/// List a root-relative directory and classify entries without following symlinks.
pub(crate) fn list_directory(root: &MutationRoot, relative: &Path) -> Result<Vec<RootDirEntry>> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::ffi::OsStrExt;

        let directory = open_existing_directory_for_read(root, relative)?.ok_or_else(|| {
            PrikkError::Io(format!("directory is absent: {}", relative.display()))
        })?;
        let mut stream = fs::Dir::read_from(&directory.fd).map_err(io_error)?;
        let mut entries = Vec::new();
        for entry in &mut stream {
            let entry = entry.map_err(io_error)?;
            let name = std::ffi::OsStr::from_bytes(entry.file_name().to_bytes()).to_os_string();
            if name == "." || name == ".." {
                continue;
            }
            let child = join_relative(relative, &name);
            let kind = inspect_entry(root, &child)?.ok_or_else(|| {
                PrikkError::Io(format!("directory entry disappeared: {}", child.display()))
            })?;
            entries.push(RootDirEntry { name, kind });
        }
        Ok(entries)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let path = root.fallback_path(relative)?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let name = entry.file_name();
            let child = join_relative(relative, &name);
            let kind = inspect_entry(root, &child)?.ok_or_else(|| {
                PrikkError::Io(format!("directory entry disappeared: {}", child.display()))
            })?;
            entries.push(RootDirEntry { name, kind });
        }
        Ok(entries)
    }
}

/// Read and require a root-relative regular file.
pub(crate) fn read_file_required(root: &MutationRoot, relative: &Path) -> Result<Vec<u8>> {
    read_file_if_exists(root, relative)?.ok_or_else(|| {
        PrikkError::Io(format!(
            "required root-relative file is absent: {}",
            relative.display()
        ))
    })
}

/// Inspect a root-relative entry without following its final component.
pub(crate) fn inspect_entry(root: &MutationRoot, relative: &Path) -> Result<Option<EntryKind>> {
    #[cfg(target_os = "linux")]
    {
        let Some(directory) = open_existing_directory_for_read(root, required_parent(relative)?)?
        else {
            return Ok(None);
        };
        let stat = match fs::statat(
            &directory.fd,
            required_file_name(relative)?,
            AtFlags::SYMLINK_NOFOLLOW,
        ) {
            Ok(stat) => stat,
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(error) => return Err(io_error(error)),
        };
        Ok(Some(classify(FileType::from_raw_mode(stat.st_mode))))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let path = root.fallback_path(relative)?;
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let kind = if metadata.file_type().is_symlink() {
            EntryKind::Symlink
        } else if metadata.is_file() {
            EntryKind::Regular
        } else if metadata.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::Other
        };
        Ok(Some(kind))
    }
}

#[cfg(target_os = "linux")]
fn classify(file_type: FileType) -> EntryKind {
    match file_type {
        FileType::RegularFile => EntryKind::Regular,
        FileType::Directory => EntryKind::Directory,
        FileType::Symlink => EntryKind::Symlink,
        FileType::Fifo
        | FileType::Socket
        | FileType::CharacterDevice
        | FileType::BlockDevice
        | FileType::Unknown => EntryKind::Other,
    }
}

fn join_relative(parent: &Path, name: &std::ffi::OsStr) -> PathBuf {
    if parent.as_os_str().is_empty() {
        PathBuf::from(name)
    } else {
        parent.join(name)
    }
}
