//! Simple file locks for active-session and ref writers.

use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};

use crate::fsutil::{MutationRoot, create_new_file_required, remove_file_cleanup_best_effort};
use crate::layout::RepositoryLayout;

/// Active session lock acquired before mutating an active WAL tail.
#[derive(Debug)]
pub struct ActiveLock {
    path: PathBuf,
    relative: PathBuf,
    mutation_root: MutationRoot,
}

impl ActiveLock {
    /// Acquire a lock through exclusive file creation.
    pub fn acquire(layout: &RepositoryLayout) -> Result<Self> {
        let path = layout.default_active_lock_path();
        let relative = layout.repository_relative(&path)?;
        let mutation_root = layout.repository_mutation_root().clone();
        acquire_lock_file(&mutation_root, &relative, &path, "active")?;
        Ok(Self {
            path,
            relative,
            mutation_root,
        })
    }

    /// Return lock file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn require_layout(&self, layout: &RepositoryLayout) -> Result<()> {
        if self
            .mutation_root
            .same_authority(layout.repository_mutation_root())
        {
            return Ok(());
        }
        Err(PrikkError::LockConflict(
            "active lock belongs to a different repository authority".to_string(),
        ))
    }
}

impl Drop for ActiveLock {
    fn drop(&mut self) {
        remove_file_cleanup_best_effort(&self.mutation_root, &self.relative);
    }
}

/// Ref-specific lock acquired before publishing one ref pointer.
#[derive(Debug)]
pub struct RefLock {
    path: PathBuf,
    relative: PathBuf,
    mutation_root: MutationRoot,
}

impl RefLock {
    /// Acquire a ref lock through exclusive file creation.
    pub fn acquire(layout: &RepositoryLayout, ref_name: &str) -> Result<Self> {
        let path = layout.ref_lock_path(ref_name);
        let relative = layout.repository_relative(&path)?;
        let mutation_root = layout.repository_mutation_root().clone();
        acquire_lock_file(&mutation_root, &relative, &path, "ref")?;
        Ok(Self {
            path,
            relative,
            mutation_root,
        })
    }

    /// Return lock file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for RefLock {
    fn drop(&mut self) {
        remove_file_cleanup_best_effort(&self.mutation_root, &self.relative);
    }
}

fn acquire_lock_file(
    mutation_root: &MutationRoot,
    relative: &Path,
    path: &Path,
    kind: &str,
) -> Result<()> {
    let body = lock_body(kind);
    match create_new_file_required(mutation_root, relative, body.as_bytes()) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Err(
            PrikkError::LockConflict(format!("{kind} lock already exists: {}", path.display())),
        ),
        Err(err) => Err(err.into()),
    }
}

fn lock_body(kind: &str) -> String {
    format!(
        "pid={}\nkind={kind}\nnote=PR-007 lock has no stale-lock stealing yet\n",
        std::process::id()
    )
}

#[cfg(test)]
mod tests;
