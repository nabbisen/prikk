//! Root-scoped directory capabilities and synchronization.

use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use prikk_error::{PrikkError, Result};

#[cfg(target_os = "linux")]
use rustix::fd::OwnedFd;
#[cfg(target_os = "linux")]
use rustix::fs::{self, Mode, OFlags};

use super::failpoints;
#[cfg(target_os = "linux")]
use super::io_error;
#[cfg(not(target_os = "linux"))]
use super::unsupported_mutation;

/// A validated mutation authority rooted at one retained directory handle.
#[derive(Clone)]
pub(crate) struct MutationRoot {
    path: Arc<PathBuf>,
    #[cfg(target_os = "linux")]
    directory: Arc<AnchoredDirectory>,
}

impl fmt::Debug for MutationRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MutationRoot")
            .finish_non_exhaustive()
    }
}

#[cfg(target_os = "linux")]
pub(super) struct AnchoredDirectory {
    pub(super) fd: OwnedFd,
}

impl MutationRoot {
    pub(crate) fn same_authority(&self, other: &Self) -> bool {
        #[cfg(target_os = "linux")]
        {
            Arc::ptr_eq(&self.directory, &other.directory)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Arc::ptr_eq(&self.path, &other.path)
        }
    }

    /// Bind mutation authority to an existing no-follow directory handle.
    pub(crate) fn open(path: &Path) -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            Ok(Self {
                path: Arc::new(path.to_path_buf()),
                directory: Arc::new(AnchoredDirectory::open(path)?),
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(Self {
                path: Arc::new(path.to_path_buf()),
            })
        }
    }

    /// Create and bind a nested root relative to this authority.
    pub(crate) fn ensure_root(&self, relative: &Path) -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            Ok(Self {
                path: Arc::new(self.fallback_path(relative)?),
                directory: Arc::new(prepare_directory_required(self, relative)?),
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = relative;
            unsupported_mutation()
        }
    }

    /// Bind a nested existing root relative to this authority.
    pub(crate) fn open_root(&self, relative: &Path) -> Result<Self> {
        #[cfg(target_os = "linux")]
        {
            Ok(Self {
                path: Arc::new(self.fallback_path(relative)?),
                directory: Arc::new(open_existing_directory_required(self, relative)?),
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(Self {
                path: Arc::new(self.fallback_path(relative)?),
            })
        }
    }

    pub(super) fn fallback_path(&self, relative: &Path) -> Result<PathBuf> {
        validate_relative(relative)?;
        Ok(self.path.join(relative))
    }

    #[cfg(target_os = "linux")]
    pub(super) fn duplicate_directory(&self) -> Result<AnchoredDirectory> {
        let fd = rustix::io::dup(&self.directory.fd).map_err(io_error)?;
        Ok(AnchoredDirectory { fd })
    }
}

#[cfg(target_os = "linux")]
impl AnchoredDirectory {
    fn open(path: &Path) -> Result<Self> {
        failpoints::required_open()?;
        let fd = fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(io_error)?;
        Ok(Self { fd })
    }

    fn open_child(&self, name: &std::ffi::OsStr) -> Result<Self> {
        failpoints::required_open()?;
        let fd = fs::openat(
            &self.fd,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(io_error)?;
        Ok(Self { fd })
    }

    fn open_child_for_read(&self, name: &std::ffi::OsStr) -> Result<Option<Self>> {
        failpoints::required_open()?;
        match fs::openat(
            &self.fd,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => Ok(Some(Self { fd })),
            Err(rustix::io::Errno::NOENT) => Ok(None),
            Err(error) => Err(io_error(error)),
        }
    }

    fn open_validated_child(&self, name: &std::ffi::OsStr) -> Result<Self> {
        let child = self.open_child(name)?;
        failpoints::observed_directory_parent_sync()?;
        self.sync()?;
        Ok(child)
    }

    fn ensure_child(&self, name: &std::ffi::OsStr) -> Result<Self> {
        failpoints::required_open()?;
        match fs::openat(
            &self.fd,
            name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => {
                failpoints::observed_directory_parent_sync()?;
                self.sync()?;
                Ok(Self { fd })
            }
            Err(rustix::io::Errno::NOENT) => {
                failpoints::directory_create()?;
                failpoints::wait_at_directory_create();
                match fs::mkdirat(&self.fd, name, Mode::from_raw_mode(0o755)) {
                    Ok(()) => {
                        failpoints::created_directory_parent_sync()?;
                        self.sync()?;
                        self.open_child(name)
                    }
                    Err(rustix::io::Errno::EXIST) => self.open_validated_child(name),
                    Err(error) => Err(io_error(error)),
                }
            }
            Err(error) => Err(io_error(error)),
        }
    }

    pub(super) fn sync(&self) -> Result<()> {
        fs::fsync(&self.fd).map_err(io_error)
    }
}

/// Ensure a relative directory tree and sync every owning parent.
pub(crate) fn ensure_directory_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        prepare_directory_required(root, relative)?;
        Ok(())
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
        let directory = open_existing_directory_required(root, relative)?;
        failpoints::required_directory_sync()?;
        directory.sync()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (root, relative);
        unsupported_mutation()
    }
}

#[cfg(target_os = "linux")]
pub(super) fn prepare_directory_required(
    root: &MutationRoot,
    relative: &Path,
) -> Result<AnchoredDirectory> {
    let mut current = root.duplicate_directory()?;
    for component in relative_components(relative)? {
        current = current.ensure_child(component)?;
    }
    Ok(current)
}

#[cfg(target_os = "linux")]
pub(super) fn open_existing_directory_required(
    root: &MutationRoot,
    relative: &Path,
) -> Result<AnchoredDirectory> {
    let mut current = root.duplicate_directory()?;
    for component in relative_components(relative)? {
        current = current.open_validated_child(component)?;
    }
    Ok(current)
}

#[cfg(target_os = "linux")]
pub(super) fn open_existing_directory_for_read(
    root: &MutationRoot,
    relative: &Path,
) -> Result<Option<AnchoredDirectory>> {
    let mut current = root.duplicate_directory()?;
    for component in relative_components(relative)? {
        let Some(child) = current.open_child_for_read(component)? else {
            return Ok(None);
        };
        current = child;
    }
    Ok(Some(current))
}

#[cfg(target_os = "linux")]
fn relative_components(path: &Path) -> Result<Vec<&std::ffi::OsStr>> {
    validate_relative(path)?;
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(value) => components.push(value),
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(PrikkError::Io(
                    "path must be relative to its authority root".to_string(),
                ));
            }
        }
    }
    Ok(components)
}

fn validate_relative(path: &Path) -> Result<()> {
    for component in path.components() {
        if matches!(
            component,
            Component::RootDir | Component::ParentDir | Component::Prefix(_)
        ) {
            return Err(PrikkError::Io(
                "path must be relative to its authority root".to_string(),
            ));
        }
    }
    Ok(())
}
