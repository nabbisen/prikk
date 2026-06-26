//! Simple file locks for active-session writers.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};

use crate::fsutil::sync_directory_best_effort;

/// Active session lock acquired before mutating an active WAL tail.
#[derive(Debug)]
pub struct ActiveLock {
    path: PathBuf,
}

impl ActiveLock {
    /// Acquire a lock through exclusive file creation.
    pub fn acquire(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let Some(parent) = path.parent() else {
            return Err(PrikkError::Io("active lock path has no parent directory".to_string()));
        };
        fs::create_dir_all(parent)?;
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(PrikkError::LockConflict(format!(
                    "active lock already exists: {}",
                    path.display()
                )));
            }
            Err(err) => return Err(err.into()),
        };
        write_lock_body(&mut file)?;
        file.sync_all()?;
        sync_directory_best_effort(parent)?;
        Ok(Self { path })
    }

    /// Return lock file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ActiveLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        if let Some(parent) = self.path.parent() {
            let _ = sync_directory_best_effort(parent);
        }
    }
}

fn write_lock_body(file: &mut File) -> Result<()> {
    writeln!(file, "pid={}", std::process::id())?;
    writeln!(file, "note=PR-004 lock has no stale-lock stealing yet")?;
    Ok(())
}
