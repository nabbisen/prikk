//! Simple file locks for active-session and ref writers.

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
        acquire_lock_file(&path, "active")?;
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
        release_lock_file(&self.path);
    }
}

/// Ref-specific lock acquired before publishing one ref pointer.
#[derive(Debug)]
pub struct RefLock {
    path: PathBuf,
}

impl RefLock {
    /// Acquire a ref lock through exclusive file creation.
    pub fn acquire(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        acquire_lock_file(&path, "ref")?;
        Ok(Self { path })
    }

    /// Return lock file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for RefLock {
    fn drop(&mut self) {
        release_lock_file(&self.path);
    }
}

fn acquire_lock_file(path: &Path, kind: &str) -> Result<()> {
    let Some(parent) = path.parent() else {
        return Err(PrikkError::Io(format!(
            "{kind} lock path has no parent directory"
        )));
    };
    fs::create_dir_all(parent)?;
    let mut file = match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(PrikkError::LockConflict(format!(
                "{kind} lock already exists: {}",
                path.display()
            )));
        }
        Err(err) => return Err(err.into()),
    };
    write_lock_body(&mut file, kind)?;
    file.sync_all()?;
    sync_directory_best_effort(parent)?;
    Ok(())
}

fn release_lock_file(path: &Path) {
    let _ = fs::remove_file(path);
    if let Some(parent) = path.parent() {
        let _ = sync_directory_best_effort(parent);
    }
}

fn write_lock_body(file: &mut File, kind: &str) -> Result<()> {
    writeln!(file, "pid={}", std::process::id())?;
    writeln!(file, "kind={kind}")?;
    writeln!(file, "note=PR-007 lock has no stale-lock stealing yet")?;
    Ok(())
}
