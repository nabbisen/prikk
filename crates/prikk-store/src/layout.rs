//! Repository layout paths and initialization.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectId, ObjectType};

use crate::fsutil::{sync_directory_best_effort, write_file_atomically};

const REPO_DIR: &str = ".prikk";
const FORMAT_VERSION: &str = "1\n";

/// Repository layout paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryLayout {
    root: PathBuf,
    prikk_dir: PathBuf,
}

impl RepositoryLayout {
    /// Create a layout for a working tree root.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let prikk_dir = root.join(REPO_DIR);
        Self { root, prikk_dir }
    }

    /// Initialize a repository layout on disk.
    pub fn init(root: impl Into<PathBuf>) -> Result<Self> {
        let layout = Self::new(root);
        fs::create_dir_all(&layout.prikk_dir)?;
        for dir in layout.required_directories() {
            fs::create_dir_all(dir)?;
        }
        write_file_atomically(&layout.format_path(), FORMAT_VERSION.as_bytes())?;
        sync_directory_best_effort(&layout.prikk_dir)?;
        Ok(layout)
    }

    /// Open an existing repository layout.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        let layout = Self::new(root);
        let mut version = String::new();
        File::open(layout.format_path())?.read_to_string(&mut version)?;
        if version != FORMAT_VERSION {
            return Err(PrikkError::UnsupportedFormatVersion(0));
        }
        Ok(layout)
    }

    /// Return the working tree root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the `.prikk` directory.
    #[must_use]
    pub fn prikk_dir(&self) -> &Path {
        &self.prikk_dir
    }

    /// Return the repository format marker path.
    #[must_use]
    pub fn format_path(&self) -> PathBuf {
        self.prikk_dir.join("FORMAT")
    }

    /// Return the object root directory.
    #[must_use]
    pub fn objects_dir(&self) -> PathBuf {
        self.prikk_dir.join("objects")
    }

    /// Return the active-session root directory.
    #[must_use]
    pub fn active_dir(&self) -> PathBuf {
        self.prikk_dir.join("active")
    }

    /// Return the default active-session directory.
    #[must_use]
    pub fn default_active_dir(&self) -> PathBuf {
        self.active_dir().join("default")
    }

    /// Return the default active WAL path.
    #[must_use]
    pub fn default_queue_wal_path(&self) -> PathBuf {
        self.default_active_dir().join("queue.wal")
    }

    /// Return the default active lock path.
    #[must_use]
    pub fn default_active_lock_path(&self) -> PathBuf {
        self.default_active_dir().join("active.lock")
    }

    /// Return the ref root directory.
    #[must_use]
    pub fn refs_dir(&self) -> PathBuf {
        self.prikk_dir.join("refs")
    }

    /// Return the cache directory.
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.prikk_dir.join("cache")
    }

    /// Return the quarantine directory.
    #[must_use]
    pub fn quarantine_dir(&self) -> PathBuf {
        self.prikk_dir.join("quarantine")
    }

    /// Return all required directories for layout creation.
    #[must_use]
    pub fn required_directories(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        dirs.push(self.objects_dir());
        for object_type in persisted_object_types() {
            dirs.push(self.object_type_dir(object_type));
        }
        dirs.push(self.active_dir());
        dirs.push(self.default_active_dir());
        dirs.push(self.refs_dir());
        dirs.push(self.refs_dir().join("by-id"));
        dirs.push(self.refs_dir().join("logs"));
        dirs.push(self.refs_dir().join("locks"));
        dirs.push(self.refs_dir().join("tmp"));
        dirs.push(self.cache_dir());
        dirs.push(self.quarantine_dir());
        dirs
    }

    /// Return the object directory for a persisted object type.
    #[must_use]
    pub fn object_type_dir(&self, object_type: ObjectType) -> PathBuf {
        self.objects_dir().join(object_type_directory_name(object_type))
    }

    /// Return the storage path for a persisted object ID and type.
    #[must_use]
    pub fn object_path(&self, object_type: ObjectType, id: ObjectId) -> PathBuf {
        let hex = id.to_hex();
        let prefix = hex_prefix(&hex);
        self.object_type_dir(object_type).join(prefix).join(format!("{hex}.pobj"))
    }
}

/// Return persisted object types. RefUpdate is log-inline in v1 and is intentionally absent.
#[must_use]
pub fn persisted_object_types() -> [ObjectType; 6] {
    [
        ObjectType::Patch,
        ObjectType::Block,
        ObjectType::RefState,
        ObjectType::Tag,
        ObjectType::Attestation,
        ObjectType::Blob,
    ]
}

/// Return a stable directory name for an object type.
#[must_use]
pub fn object_type_directory_name(object_type: ObjectType) -> &'static str {
    match object_type {
        ObjectType::Patch => "patch",
        ObjectType::Block => "block",
        ObjectType::RefState => "ref-state",
        ObjectType::Tag => "tag",
        ObjectType::Attestation => "attestation",
        ObjectType::Blob => "blob",
        ObjectType::RefUpdate => "ref-update-inline-only",
    }
}

fn hex_prefix(hex: &str) -> String {
    hex.chars().take(2).collect()
}
