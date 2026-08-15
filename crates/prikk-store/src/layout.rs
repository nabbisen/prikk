//! Repository layout paths and initialization.

use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};
use prikk_hash::{sha256, to_hex};
use prikk_object::{ObjectId, ObjectType, is_windows_reserved_name};

use crate::fsutil::{
    MutationRoot, create_new_file_required, ensure_directory_required, read_file_if_exists,
    read_file_required, write_file_atomically,
};

const REPO_DIR: &str = ".prikk";
const LEGACY_FORMAT_VERSION: &[u8] = b"1\n";
const LEGACY_FORMAT_2_VERSION: &[u8] = b"2\n";
const LEGACY_FORMAT_3_VERSION: &[u8] = b"3\n";
const LEGACY_FORMAT_4_VERSION: &[u8] = b"4\n";
const CURRENT_FORMAT_VERSION: &[u8] = b"5\n";

/// Repository format selected by the authoritative `.prikk/FORMAT` marker.
///
/// RFC 103 retired format 1; RFC 102 Stage 3 retired format 2 the same way, RFC 102 Stage 4 did the
/// same again for format 3, and RFC 102 Stage 5 does it once more for format 4 -- rejected at open
/// (`read_repository_format`), not merely unsupported for mutation, no variant naming it here.
/// **This bump was sought and decided by the owner explicitly** (design-v1.md §14.7, 2026-08-15),
/// unlike the 3→4 bump, which reused Stage 3's own already-ruled general policy without a fresh
/// decision: Stage 5 adds container names to `init` that the 3→4-bump's own `durable_append`
/// strictness change made unsafe to leave undetected in an older repository (design-v1.md §14.7's own
/// account of why no middle path -- create-on-open or a side-check without a bump -- was viable). The
/// single remaining variant is kept as an enum rather than collapsed away, per design-v1.md §12.1's
/// own note: `require_current_format`'s disk re-read is a real runtime check (RFC 103 Increment B was
/// abandoned specifically because of it), so the enum's *shape* still carries meaning and is not free
/// to simplify away.
///
/// **"Format 2"/"format 3"/"format 4"/"format 5" here name the on-disk repository layout** (loose
/// objects/refs vs. RFC 102's containers) **-- a different axis from DC-40's "format-2" wire schema**
/// (`block_state.rs`, `state_root.rs`, `format.rs`'s Block/Patch shape and Merkle rules), which no RFC
/// 102 stage touches and which keeps its own "format-2" name regardless of what this enum's current
/// variant is called (design §8: "format-2's rejection of the ahead-log state" is explicitly
/// unchanged).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryFormat {
    /// Current format 5: RFC 102 Stage 5's container-based trust/received-ref/active-session storage,
    /// layered on Stage 3's and Stage 4's container-based object and ref storage, still writable under
    /// the unchanged DC-40 schema and state-root rules.
    CurrentV5,
}

/// One object-type container's pre-allocated alternate slot (RFC's §3.2 compaction requirement: a
/// fixed A/B pair of names, never a rotated/new name). Stage 3 only ever writes `A`; `B` exists solely
/// so its name is already allocated at `init` for Stage 6 compaction to use later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerSlot {
    /// The slot every Stage-3-era write and read targets.
    A,
    /// Reserved for compaction (Stage 6, not authorized). Never written by this stage.
    B,
}

impl ContainerSlot {
    /// Return a stable lower-case label used in the container's file name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::B => "b",
        }
    }
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
                // RFC 102 Stage 3, design-v1.md §12.1: this refusal inherited the format-2 -> 3 bump;
                // Stage 4 carried it forward for format-3 -> 4, and Stage 5 does the same again for
                // format-4 -> 5 (design-v1.md §14.7). Audited, not just the constant swapped: unlike
                // `read_repository_format`'s own rejection (reached via `open`), this fires only on a
                // redundant `init` against an already-initialized repository of some other format, so
                // it stays terse and points at `open` (any other command) for the detailed migration
                // message rather than duplicating it here.
                return Err(PrikkError::Integrity(
                    "refusing to initialize an existing non-format-5 Prikk repository (open it \
                     with any other command for a detailed unsupported-format message)"
                        .to_string(),
                ));
            }
        }
        let layout = Self {
            root,
            prikk_dir,
            worktree_mutation,
            repository_mutation,
            format: RepositoryFormat::CurrentV5,
        };
        for dir in layout.required_repository_directories()? {
            ensure_directory_required(layout.repository_mutation_root(), &dir)?;
        }
        // RFC 102 Stage 1: created at `init`, never later -- a missing file and an idempotent
        // re-`init` on an already-initialized repository must not clobber either.
        create_empty_file_once(&layout, &layout.worktree_unclean_shutdown_marker_path())?;
        create_empty_file_once(&layout, &layout.default_queue_wal_path())?;
        // RFC 102 Stage 5, design-v1.md §14.6: the active-WAL ref-ownership metadata, on the same
        // marker pattern as the worktree marker above -- created empty at `init`, set by truncate-then
        // -append, cleared by truncate-to-empty, never removed. Previously created lazily on the
        // empty-to-non-empty WAL transition (`active.rs::prepare_empty_active_ref_for_append`).
        create_empty_file_once(&layout, &layout.default_active_ref_name_path())?;
        // RFC 102 Stage 3, design-v1.md §2: every container name, both slots, plus the index and the
        // (currently unused) compaction generation log -- all allocated here, at `init`, and nowhere
        // else, for the life of the repository. This is the acceptance test itself (handoff §5
        // criterion 1): enumerate every one of these paths and confirm none of them is ever created
        // by any other code path.
        for object_type in persisted_object_types() {
            create_empty_file_once(
                &layout,
                &layout.container_slot_path(object_type, ContainerSlot::A),
            )?;
            create_empty_file_once(
                &layout,
                &layout.container_slot_path(object_type, ContainerSlot::B),
            )?;
        }
        create_empty_file_once(&layout, &layout.container_index_path())?;
        create_empty_file_once(&layout, &layout.container_generation_log_path())?;
        // RFC 102 Stage 4, Step 0 §13.2/§13.4: the shared ref-log container's both slots, plus the
        // separate ref-pointer-index container -- allocated here, at `init`, and nowhere else, the
        // same acceptance-criterion-1 discipline Stage 3 established.
        create_empty_file_once(
            &layout,
            &layout.ref_log_container_slot_path(ContainerSlot::A),
        )?;
        create_empty_file_once(
            &layout,
            &layout.ref_log_container_slot_path(ContainerSlot::B),
        )?;
        create_empty_file_once(&layout, &layout.ref_pointer_index_path())?;
        create_empty_file_once(&layout, &layout.received_index_path())?;
        create_empty_file_once(&layout, &layout.trust_key_container_path())?;
        create_empty_file_once(&layout, &layout.trust_policy_container_path())?;
        // RFC 102 Stage 5, design-v1.md §14.2: written last, once every container/marker/WAL name
        // above is confirmed present. `FORMAT`'s presence is what certifies `init` completed --
        // written first (the old order), a crash between it and the containers left a repository
        // that read as a valid, empty format-4 repository with every container absent, and nothing
        // detected it (`status`/`verify`/`doctor` all exited 0 against a probe repository with all 16
        // container files deleted). Written last, an interrupted `init` leaves `FORMAT` absent, so a
        // re-`init` skips the mismatched-format guard above (it only fires when `FORMAT` already
        // exists) and re-enters this same body -- every `create_empty_file_once` call above is
        // idempotent, so the re-run completes whichever names are still missing and finishes by
        // writing `FORMAT`, exactly the "detectable and completable" property the reordering exists
        // to provide.
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
        if self.format == RepositoryFormat::CurrentV5 {
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

    /// Return the unclean-shutdown worktree marker path (RFC 102 Stage 1). Created empty at `init`;
    /// non-empty means worktree materialization was interrupted and commit-authoring must refuse to
    /// infer deletion from absence until the worktree is re-verified against its baseline. Always
    /// updated by append/truncate (`fsutil::append_file_required`/`truncate_file_empty_required`),
    /// never by `atomic_replace` -- RFC 102 §3's correction: `atomic_replace` renames over the
    /// destination unconditionally, which is a new-name event whose Windows durability is DC-87
    /// §3.4's still-open question, exactly the gap this marker exists to close.
    #[must_use]
    pub fn worktree_unclean_shutdown_marker_path(&self) -> PathBuf {
        self.prikk_dir.join("worktree.marker")
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

    /// Return the container root directory (RFC 102 Stage 3, design-v1.md §2).
    #[must_use]
    pub fn containers_dir(&self) -> PathBuf {
        self.prikk_dir.join("containers")
    }

    /// Return the container directory for a persisted object type.
    #[must_use]
    pub fn container_type_dir(&self, object_type: ObjectType) -> PathBuf {
        self.containers_dir()
            .join(object_type_directory_name(object_type))
    }

    /// Return one object type's container file for a given slot. **Every slot's name is allocated
    /// at `init`, including B, even though Stage 3 only ever writes A** -- compaction (Stage 6, not
    /// authorized) is what would ever target B; the RFC's §3.2 fixed-name-set requirement applies to
    /// the whole RFC, not per stage, so the name exists now regardless of when it is first used.
    #[must_use]
    pub fn container_slot_path(&self, object_type: ObjectType, slot: ContainerSlot) -> PathBuf {
        self.container_type_dir(object_type)
            .join(format!("{}.container", slot.as_str()))
    }

    /// Return the object index's container path. Single file, no A/B slot -- design-v1.md §4 /
    /// RFC 102's own §6.7 answer #2: the index's publication shape is plain append-only ("A/B" for an
    /// index reduces to "append-only wearing an A/B costume, not a second option" once forced through
    /// this codebase's real primitives), so unlike the six object-type containers it needs only one
    /// name.
    #[must_use]
    pub fn container_index_path(&self) -> PathBuf {
        self.containers_dir().join("index.container")
    }

    /// Return the small, fixed-name compaction generation log (design-v1.md §4: "compaction publishes
    /// by appending a generation record to a small fixed-name log; readers take the last complete
    /// generation record"). **Reserved, not used, by Stage 3** -- its name must still be allocated at
    /// `init` because compaction (Stage 6) is not authorized to create any name later. Absent any
    /// generation record (the only state Stage 3 ever produces), every container type's slot A is
    /// live by construction -- there is nothing for an empty log to disambiguate yet.
    #[must_use]
    pub fn container_generation_log_path(&self) -> PathBuf {
        self.containers_dir().join("generations.log")
    }

    /// Return the ref-container root directory (RFC 102 Stage 4). Kept under `refs/`, sibling to the
    /// now-vestigial `by-id/`/`logs/`/`tmp/`/`locks/` directories, rather than under the object
    /// `containers/` tree -- ref containers are not object containers and the two are never confused
    /// for the same purpose.
    #[must_use]
    pub fn refs_containers_dir(&self) -> PathBuf {
        self.refs_dir().join("containers")
    }

    /// Return the shared ref-log container file for a given slot (Step 0 §13.2: one container holds
    /// every ref's log records, forced by acceptance criterion 1 -- ref names do not exist at `init`,
    /// so a per-ref container is architecturally impossible). **Both slots allocated at `init`**,
    /// matching Stage 3's own A/B convention exactly (`container_slot_path`'s own doc comment) --
    /// Stage 4 only ever writes `A`.
    #[must_use]
    pub fn ref_log_container_slot_path(&self, slot: ContainerSlot) -> PathBuf {
        self.refs_containers_dir()
            .join(format!("log-{}.container", slot.as_str()))
    }

    /// Return the ref-pointer-index container path (Step 0 §13.4, ruled in design-v1.md §13.4: a
    /// *separate* ref index, its own type and its own container -- `index.rs`'s already-shipped object
    /// index schema is never widened). Single file, no A/B slot, same reasoning as
    /// `container_index_path`: an index's publication shape is plain append-only.
    #[must_use]
    pub fn ref_pointer_index_path(&self) -> PathBuf {
        self.refs_containers_dir().join("pointer-index.container")
    }

    /// Return the received-ref-index container path (RFC 102 Stage 5, design-v1.md §14.1/Step 0 item
    /// 2: `received.rs` on the same unbounded-per-name-minted-after-`init` shape that forced the ref
    /// pointer index, Stage 4's §13.2 argument applied to a second subsystem). Single file, no A/B
    /// slot, no separate log -- a received ref's own semantics are already last-entry-wins/replace-
    /// outright (`received.rs`'s own doc: "no CAS and no merge... this is what I have now"), the same
    /// shape the ref pointer index already proved sound, so this container plays both roles at once.
    #[must_use]
    pub fn received_index_path(&self) -> PathBuf {
        self.refs_containers_dir().join("received-index.container")
    }

    /// Return all required directories for layout creation.
    #[must_use]
    pub fn required_directories(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        dirs.push(self.objects_dir());
        for object_type in persisted_object_types() {
            dirs.push(self.object_type_dir(object_type));
        }
        dirs.push(self.containers_dir());
        for object_type in persisted_object_types() {
            dirs.push(self.container_type_dir(object_type));
        }
        dirs.push(self.active_dir());
        dirs.push(self.default_active_dir());
        dirs.push(self.refs_dir());
        dirs.push(self.refs_dir().join("by-id"));
        dirs.push(self.refs_dir().join("logs"));
        dirs.push(self.refs_dir().join("locks"));
        dirs.push(self.refs_dir().join("tmp"));
        dirs.push(self.refs_containers_dir());
        dirs.push(self.trust_dir());
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

    /// Return the trust key-material container path (RFC 102 Stage 5, design-v1.md §14/§14.9).
    /// Replaces the one-file-per-key-id `trust/keys/maintainer/*.pub` directory entirely -- format 5
    /// rejects every repository old enough to have one, so no repository this code can open ever
    /// contains that directory's contents (§14.9 §3's own reasoning, applied here as it was to
    /// `refs/received/`, not Stage 4's "keep, dead" precedent).
    #[must_use]
    pub fn trust_key_container_path(&self) -> PathBuf {
        self.trust_dir().join("keys.container")
    }

    /// Return the trust policy container path (RFC 102 Stage 5, design-v1.md §14/§14.9). Replaces
    /// `trust/policy.toml`. Each append is a **complete snapshot** of the adopted key id list, not an
    /// incremental log entry -- see `trust_index.rs`'s own module doc for why that is what makes
    /// revocation representable without a tombstone record.
    #[must_use]
    pub fn trust_policy_container_path(&self) -> PathBuf {
        self.trust_dir().join("policy.container")
    }
}

/// Validate a maintainer key id's storage safety: ASCII alphanumeric/`-`/`_` only, and not a
/// Windows-reserved device stem. Split out from the retired `maintainer_trust_key_path` (which paired
/// this check with building a per-key-id file path that no longer exists under the container model) --
/// the validation itself is unchanged and still required before a key id is accepted.
pub(crate) fn validate_maintainer_key_id_storage_safety(key_id: &str) -> Result<()> {
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
    Ok(())
}

/// Create `path` empty if it does not already exist. Idempotent, so a retried or re-run `init`
/// against an already-initialized repository never clobbers it -- the same rule RFC 102 Stage 1
/// established for the worktree marker and the active WAL, now shared by every `init`-time file this
/// layout creates.
fn create_empty_file_once(layout: &RepositoryLayout, path: &Path) -> Result<()> {
    let relative = layout.repository_relative(path)?;
    if read_file_if_exists(layout.repository_mutation_root(), &relative)?.is_none() {
        create_new_file_required(layout.repository_mutation_root(), &relative, &[])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;

fn read_repository_format(root: &MutationRoot) -> Result<RepositoryFormat> {
    let version = read_file_required(root, Path::new("FORMAT"))?;
    match version.as_slice() {
        LEGACY_FORMAT_VERSION => Err(PrikkError::Integrity(
            "this repository uses format 1, which prikk no longer supports (this version \
             requires format 5). format-1 support was removed after 0.19.0. to migrate: use \
             prikk 0.19.0 or earlier to `prikk bundle export`, then `prikk bundle import` here"
                .to_string(),
        )),
        // RFC 102 Stage 3, design-v1.md §12.1 (owner decision): bump to format 3, reject format-2 at
        // open, no dual-layout bridge. Format 2 was current through the 0.19.0 release (the same
        // release format-1's own rejection message above already points at), so it is the last
        // release able to open a format-2 repository -- established from the release record
        // (`CHANGELOG.md`, `git tag`), not guessed.
        LEGACY_FORMAT_2_VERSION => Err(PrikkError::Integrity(
            "this repository uses format 2, which prikk no longer supports (this version \
             requires format 5). format-2 support was removed after 0.19.0. to migrate: use \
             prikk 0.19.0 or earlier to `prikk bundle export`, then `prikk bundle import` here"
                .to_string(),
        )),
        // RFC 102 Stage 4: bump to format 4, reject format-3 at open, no dual-layout bridge --
        // applying Stage 3's own already-ruled policy (design-v1.md §12.1), not a fresh decision (see
        // `RepositoryFormat`'s own doc comment). Unlike format 2, format 3 was never itself the
        // subject of a tagged release (Stage 3 was still pending three-platform CI when Stage 4
        // began) -- no specific "removed after X.Y.Z" version is named here, since none can be
        // verified from the release record yet; naming one would be guessing, which this project's
        // own discipline for these messages does not do.
        LEGACY_FORMAT_3_VERSION => Err(PrikkError::Integrity(
            "this repository uses format 3, which prikk no longer supports (this version \
             requires format 5). to migrate: use a prikk version that supports format 3 to \
             `prikk bundle export`, then `prikk bundle import` here"
                .to_string(),
        )),
        // RFC 102 Stage 5, design-v1.md §14.7 (owner decision): bump to format 5, reject format-4 at
        // open, no dual-layout bridge -- format 3's precedent, not format 2's. No release was ever
        // tagged at format 4 either (Stage 4 merged and Stage 5 began before any tag), verified against
        // the release record (`CHANGELOG.md`, `git tag`) rather than assumed from format 3's own
        // no-version precedent -- naming one anyway would be guessing, which this project's own
        // discipline for these messages does not do.
        LEGACY_FORMAT_4_VERSION => Err(PrikkError::Integrity(
            "this repository uses format 4, which prikk no longer supports (this version \
             requires format 5). to migrate: use a prikk version that supports format 4 to \
             `prikk bundle export`, then `prikk bundle import` here"
                .to_string(),
        )),
        CURRENT_FORMAT_VERSION => Ok(RepositoryFormat::CurrentV5),
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
    to_hex(&ref_name_key_bytes(ref_name))
}

/// The raw 32-byte form of [`ref_name_storage_key`] (RFC 102 Stage 4, Step 0 §13.4 / design-v1.md
/// §13.4's ruling): a fixed-width key already used to name every ref pointer/log file today, reused
/// as the ref-pointer-index's own key rather than inventing a second one -- the "new key shape"
/// objection Step 0 raised dissolved specifically because this already existed.
#[must_use]
pub(crate) fn ref_name_key_bytes(ref_name: &str) -> [u8; 32] {
    sha256(ref_name.as_bytes())
}
