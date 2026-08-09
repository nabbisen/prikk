//! Repository layout paths and initialization.

use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};
use prikk_hash::{sha256, to_hex};
use prikk_object::{ObjectId, ObjectType, is_windows_reserved_name};

use crate::fsutil::{
    MutationRoot, ensure_directory_required, read_file_if_exists, read_file_required,
    write_file_atomically,
};

const REPO_DIR: &str = ".prikk";
const LEGACY_FORMAT_VERSION: &[u8] = b"1\n";
const CURRENT_FORMAT_VERSION: &[u8] = b"2\n";

/// Repository format selected by the authoritative `.prikk/FORMAT` marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryFormat {
    /// Released format 1, opened for bounded legacy read-only use.
    LegacyV1,
    /// Current format 2, writable under the DC-40 schema and state-root rules.
    CurrentV2,
}

/// Repository layout paths.
#[derive(Debug, Clone)]
pub struct RepositoryLayout {
    root: PathBuf,
    prikk_dir: PathBuf,
    worktree_mutation: MutationRoot,
    repository_mutation: MutationRoot,
    format: RepositoryFormat,
}

impl PartialEq for RepositoryLayout {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root && self.prikk_dir == other.prikk_dir
    }
}

impl Eq for RepositoryLayout {}

impl RepositoryLayout {
    /// Create a layout for a working tree root.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let prikk_dir = root.join(REPO_DIR);
        let worktree_mutation = MutationRoot::open(&root)?;
        let repository_mutation = worktree_mutation.open_root(Path::new(REPO_DIR))?;
        let format = read_repository_format(&repository_mutation)?;
        Ok(Self {
            root,
            prikk_dir,
            worktree_mutation,
            repository_mutation,
            format,
        })
    }

    /// Initialize a repository layout on disk.
    pub fn init(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let prikk_dir = root.join(REPO_DIR);
        let worktree_mutation = MutationRoot::open(&root)?;
        let repository_mutation = worktree_mutation.ensure_root(Path::new(REPO_DIR))?;
        if let Some(version) = read_file_if_exists(&repository_mutation, Path::new("FORMAT"))? {
            if version != CURRENT_FORMAT_VERSION {
                return Err(PrikkError::Integrity(
                    "refusing to initialize an existing non-format-2 Prikk repository".to_string(),
                ));
            }
        }
        let layout = Self {
            root,
            prikk_dir,
            worktree_mutation,
            repository_mutation,
            format: RepositoryFormat::CurrentV2,
        };
        for dir in layout.required_repository_directories()? {
            ensure_directory_required(layout.repository_mutation_root(), &dir)?;
        }
        if read_file_if_exists(layout.repository_mutation_root(), Path::new("FORMAT"))?.is_none() {
            write_file_atomically(
                layout.repository_mutation_root(),
                Path::new("FORMAT"),
                CURRENT_FORMAT_VERSION,
            )?;
        }
        Ok(layout)
    }

    /// Open an existing repository layout.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self> {
        Self::new(root)
    }

    /// Return the repository format selected when this layout was opened.
    #[must_use]
    pub const fn format(&self) -> RepositoryFormat {
        self.format
    }

    pub(crate) fn validate_format(&self) -> Result<()> {
        let format = read_repository_format(self.repository_mutation_root())?;
        if format != self.format {
            return Err(PrikkError::UnsupportedFormatVersion(0));
        }
        Ok(())
    }

    /// Refuse ordinary repository/worktree mutation in legacy format 1.
    pub fn require_current_format(&self) -> Result<()> {
        self.validate_format()?;
        if self.format == RepositoryFormat::CurrentV2 {
            return Ok(());
        }
        Err(PrikkError::UnsupportedFormatVersion(1))
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

    /// Return the default active-session ref-name metadata path.
    #[must_use]
    pub fn default_active_ref_name_path(&self) -> PathBuf {
        self.default_active_dir().join("ref-name")
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
        dirs.push(self.trust_dir());
        dirs.push(self.trust_keys_dir());
        dirs.push(self.maintainer_trust_keys_dir());
        dirs.push(self.cache_dir());
        dirs.push(self.quarantine_dir());
        dirs
    }

    pub(crate) fn repository_mutation_root(&self) -> &MutationRoot {
        &self.repository_mutation
    }

    pub(crate) fn worktree_mutation_root(&self) -> &MutationRoot {
        &self.worktree_mutation
    }

    pub(crate) fn repository_relative(&self, path: &Path) -> Result<PathBuf> {
        path.strip_prefix(&self.prikk_dir)
            .map(Path::to_path_buf)
            .map_err(|_| {
                PrikkError::Io("path is outside repository mutation authority".to_string())
            })
    }

    fn required_repository_directories(&self) -> Result<Vec<PathBuf>> {
        self.required_directories()
            .into_iter()
            .map(|path| self.repository_relative(&path))
            .collect()
    }

    /// Return the object directory for a persisted object type.
    #[must_use]
    pub fn object_type_dir(&self, object_type: ObjectType) -> PathBuf {
        self.objects_dir()
            .join(object_type_directory_name(object_type))
    }

    /// Return the storage path for a persisted object ID and type.
    #[must_use]
    pub fn object_path(&self, object_type: ObjectType, id: ObjectId) -> PathBuf {
        let hex = id.to_hex();
        let prefix = hex_prefix(&hex);
        self.object_type_dir(object_type)
            .join(prefix)
            .join(format!("{hex}.pobj"))
    }

    /// Return the flat ref pointer path for a human-readable ref name.
    #[must_use]
    pub fn ref_pointer_path(&self, ref_name: &str) -> PathBuf {
        self.refs_dir()
            .join("by-id")
            .join(format!("{}.ref", ref_name_storage_key(ref_name)))
    }

    /// Return the ref log path for a human-readable ref name.
    #[must_use]
    pub fn ref_log_path(&self, ref_name: &str) -> PathBuf {
        self.refs_dir()
            .join("logs")
            .join(format!("{}.log", ref_name_storage_key(ref_name)))
    }

    /// Return the ref lock path for a human-readable ref name.
    #[must_use]
    pub fn ref_lock_path(&self, ref_name: &str) -> PathBuf {
        self.refs_dir()
            .join("locks")
            .join(format!("{}.lock", ref_name_storage_key(ref_name)))
    }

    /// Return the ref temporary candidate path for a human-readable ref name.
    #[must_use]
    pub fn ref_tmp_path(&self, ref_name: &str) -> PathBuf {
        self.refs_dir()
            .join("tmp")
            .join(format!("{}.tmp", ref_name_storage_key(ref_name)))
    }

    /// Return the publication trust-store directory.
    #[must_use]
    pub fn trust_dir(&self) -> PathBuf {
        self.prikk_dir.join("trust")
    }

    /// Return the trust-store key directory.
    #[must_use]
    pub fn trust_keys_dir(&self) -> PathBuf {
        self.trust_dir().join("keys")
    }

    /// Return the trusted MAINTAINER public-key directory.
    #[must_use]
    pub fn maintainer_trust_keys_dir(&self) -> PathBuf {
        self.trust_keys_dir().join("maintainer")
    }

    /// Return the trusted MAINTAINER public-key path for a storage-safe key id.
    pub fn maintainer_trust_key_path(&self, key_id: &str) -> Result<PathBuf> {
        if key_id.is_empty()
            || !key_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(PrikkError::InvalidName(
                "maintainer key id is not storage-safe".to_string(),
            ));
        }
        // DC-72: the allowlist above is character-shape only and does not exclude Windows-reserved
        // device stems (`CON`, `PRN`, ...) — `CON` is all ASCII-alphanumeric and would otherwise
        // pass. Checked regardless of host OS, matching `RepoPath`'s equivalent rule.
        if is_windows_reserved_name(key_id) {
            return Err(PrikkError::InvalidName(format!(
                "maintainer key id is a Windows reserved device name: {key_id}"
            )));
        }
        Ok(self
            .maintainer_trust_keys_dir()
            .join(format!("{key_id}.pub")))
    }

    /// Return the trust policy path.
    #[must_use]
    pub fn trust_policy_path(&self) -> PathBuf {
        self.trust_dir().join("policy.toml")
    }
}

fn read_repository_format(root: &MutationRoot) -> Result<RepositoryFormat> {
    let version = read_file_required(root, Path::new("FORMAT"))?;
    match version.as_slice() {
        LEGACY_FORMAT_VERSION => Ok(RepositoryFormat::LegacyV1),
        CURRENT_FORMAT_VERSION => Ok(RepositoryFormat::CurrentV2),
        _ => Err(PrikkError::UnsupportedFormatVersion(0)),
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
        // New FDD-03 §3 types. Full storage-layout placement (`objects/genesis/`,
        // `cache/block-summary/`, `refs/recovery/`) is reconciled in the FDD-02
        // layout phase; these names keep the mapper exhaustive without creating
        // directories yet.
        ObjectType::BlockSummaryCache => "block-summary-cache-rebuildable",
        ObjectType::RecoveryNote => "recovery-note-inline-only",
        ObjectType::ProjectGenesis => "genesis",
    }
}

fn hex_prefix(hex: &str) -> String {
    hex.chars().take(2).collect()
}

pub(crate) fn ref_name_storage_key(ref_name: &str) -> String {
    to_hex(&sha256(ref_name.as_bytes()))
}
