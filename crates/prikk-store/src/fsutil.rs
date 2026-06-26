//! Filesystem utility helpers for storage operations.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};

/// Write a file through a temporary path, fsync the file, rename it, and fsync the parent.
pub(crate) fn write_file_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Err(PrikkError::Io("atomic write path has no parent directory".to_string()));
    };
    fs::create_dir_all(parent)?;
    let tmp_path = temporary_path(path);
    {
        let mut file = File::create(&tmp_path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    sync_directory_best_effort(parent)?;
    Ok(())
}

/// Return a process-local temporary path next to the destination.
pub(crate) fn temporary_path(path: &Path) -> PathBuf {
    let mut file_name = path.file_name().map(|name| name.to_os_string()).unwrap_or_default();
    file_name.push(format!(".tmp.{}", std::process::id()));
    path.with_file_name(file_name)
}

/// Best-effort directory sync used after durable file creation or rename.
pub(crate) fn sync_directory_best_effort(path: &Path) -> Result<()> {
    match File::open(path) {
        Ok(file) => {
            let _ = file.sync_all();
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => Ok(()),
        Err(err) => Err(err.into()),
    }
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
