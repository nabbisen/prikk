//! The Windows implementation of the durability contract (DC-87 Stage 2, design-v1.md §2-§4).
//!
//! **G1 (root-anchored, no-follow resolution), what it holds and what it does not.** Every path
//! this module resolves is walked one component at a time (`directory.rs`'s
//! `WindowsAuthority`/`prepare_windows_directory_required`/`open_existing_windows_directory_required`),
//! each component opened with `FILE_FLAG_OPEN_REPARSE_POINT` and checked for the
//! `FILE_ATTRIBUTE_REPARSE_POINT` bit before being treated as a plain directory or file. A reparse
//! point already in place when a component is opened is detected and refused, unconditionally.
//! **What this does not close**: the walk is not handle-anchored between steps the way `openat` is
//! on Linux/macOS -- there is no Win32 primitive that resolves a child by name against an
//! already-open directory handle, and the identity check that would substitute for one
//! (`file_index`/`volume_serial_number`) is behind the unstable `windows_by_handle` feature. A
//! concurrent local process that replaces a component *after* this module's check and *before* the
//! next component's open is not detected -- one precise window, requiring a concurrent local
//! attacker, documented in `docs/src/reference/platform-support.md` and accepted there on the
//! condition that it be stated rather than elided (`prerequisite-ruling-v1.md` §4.1).
//!
//! **`FILE_SHARE_DELETE` is a whole-backend rule, enforced in one place.** [`open_no_follow`] is
//! the only function in this module that calls `CreateFile` (via `std::fs::OpenOptions`), and it
//! always requests `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE` -- matching `std`'s own
//! documented default, stated explicitly here rather than relied on implicitly, so a future call
//! site cannot narrow it by omission. Without it, [`remove_if_present`](DurabilityContract::remove_if_present)'s
//! guarantee is false on Windows: `DeleteFileW` fails against a file another handle holds open
//! without that flag, unlike POSIX `unlink`, which never fails for that reason.
//!
//! **Two documented no-ops.** [`set_permission_bits`](DurabilityContract::set_permission_bits) and
//! [`durable_directory_entry`](DurabilityContract::durable_directory_entry) are each `Ok(())` with
//! no filesystem effect -- see their own doc comments below for exactly why each is safe rather
//! than merely silent.
//!
//! **Three weaker-guarantee methods, documented rather than approximated.**
//! [`atomic_replace`](DurabilityContract::atomic_replace),
//! [`promote`](DurabilityContract::promote), and
//! [`publish_immutable`](DurabilityContract::publish_immutable) all rest on rename/link semantics
//! Windows does not provide identically to POSIX -- see each method's own doc comment for the
//! specific gap and why it is acceptable given today's callers.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use prikk_error::{PrikkError, Result};

use super::directory::{
    MutationRoot, open_existing_windows_directory_required, prepare_windows_directory_required,
};
use super::prikk_to_io;
use super::regular::{required_file_name, required_parent};
use crate::fsutil::contract::DurabilityContract;
use crate::fsutil::temporary_path;

/// `CreateFile`'s `dwFlagsAndAttributes` bit required to obtain a handle to a directory at all
/// (Microsoft Learn, `CreateFileA`, `dwFlagsAndAttributes`: *"You must set this flag to obtain a
/// handle to a directory."*), value `0x02000000`.
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
/// Same reference: land on a reparse point itself rather than transparently following it, value
/// `0x00200000`.
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
/// Microsoft Learn, "File Attribute Constants (WinNT.h)": *"A file or directory that has an
/// associated reparse point, or a file that is a symbolic link."*, value `0x00000400`.
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
/// Same `CreateFileA` reference, `dwShareMode` table: `FILE_SHARE_READ` (`0x00000001`) |
/// `FILE_SHARE_WRITE` (`0x00000002`) | `FILE_SHARE_DELETE` (`0x00000004`). This matches `std`'s own
/// documented default share mode -- stated explicitly so it cannot be narrowed by omission at any
/// call site, since [`open_no_follow`] is the only place a share mode is chosen.
const SHARE_READ_WRITE_DELETE: u32 = 0x0000_0001 | 0x0000_0002 | 0x0000_0004;

fn io_error(path: &Path, error: io::Error) -> PrikkError {
    PrikkError::Io(format!("{}: {error}", path.display()))
}

/// The single open primitive every operation in this module funnels through -- the enforcement
/// point for `FILE_SHARE_DELETE` (module doc) and for `FILE_FLAG_OPEN_REPARSE_POINT` (no
/// transparent reparse-point following at the component being opened). `for_directory` adds
/// `FILE_FLAG_BACKUP_SEMANTICS`, required to obtain a directory handle via `CreateFile` at all.
fn open_no_follow(path: &Path, options: &mut OpenOptions, for_directory: bool) -> io::Result<File> {
    let mut flags = FILE_FLAG_OPEN_REPARSE_POINT;
    if for_directory {
        flags |= FILE_FLAG_BACKUP_SEMANTICS;
    }
    options
        .share_mode(SHARE_READ_WRITE_DELETE)
        .custom_flags(flags)
        .open(path)
}

fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

fn open_directory_handle(path: &Path) -> io::Result<File> {
    open_no_follow(path, OpenOptions::new().read(true), true)
}

fn validate_directory_not_reparse_point(file: &File, path: &Path) -> Result<()> {
    let metadata = file.metadata().map_err(|error| io_error(path, error))?;
    if is_reparse_point(&metadata) {
        return Err(PrikkError::Io(format!(
            "refusing to resolve through a reparse point: {}",
            path.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(PrikkError::Io(format!(
            "expected a directory: {}",
            path.display()
        )));
    }
    Ok(())
}

/// Open `path` as a directory handle, refusing a reparse point or non-directory. Component-level
/// building block for `windows_authority.rs`'s `WindowsAuthority`. Returns the validated handle
/// (rather than discarding it) so a caller that needs this component's identity -- only the final
/// component of a walk, per DC-96 -- can read it from the same open, not a second one.
pub(super) fn open_directory_no_follow(path: &Path) -> Result<File> {
    let file = open_directory_handle(path).map_err(|error| io_error(path, error))?;
    validate_directory_not_reparse_point(&file, path)?;
    Ok(file)
}

/// Read `file`'s identity (DC-96) -- the `windows.rs`-local wrapper around `prikk_ffi::identity_of`
/// that also attaches `path` to any I/O error, matching every other function in this module.
pub(super) fn identity_no_follow(file: &File, path: &Path) -> Result<prikk_ffi::FileIdentity> {
    prikk_ffi::identity_of(file).map_err(|error| io_error(path, error))
}

/// `open_directory_no_follow`, but `None` (not an error) when `path` does not exist.
pub(super) fn stat_directory_no_follow(path: &Path) -> Result<Option<()>> {
    match open_directory_handle(path) {
        Ok(file) => {
            validate_directory_not_reparse_point(&file, path)?;
            Ok(Some(()))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(path, error)),
    }
}

/// Open or create `path` as a directory, tolerating a concurrent creator (G8) the same way
/// `AnchoredDirectory::ensure_child` does on Unix: try the open first, create only on `NotFound`,
/// and treat a create-time `AlreadyExists` as the concurrent winner rather than an error. Returns
/// the validated handle for the same reason `open_directory_no_follow` does.
pub(super) fn ensure_directory_component_no_follow(path: &Path) -> Result<File> {
    match open_directory_handle(path) {
        Ok(file) => {
            validate_directory_not_reparse_point(&file, path)?;
            Ok(file)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => match fs::create_dir(path) {
            Ok(()) => open_directory_no_follow(path),
            Err(create_error) if create_error.kind() == io::ErrorKind::AlreadyExists => {
                open_directory_no_follow(path)
            }
            Err(create_error) => Err(io_error(path, create_error)),
        },
        Err(error) => Err(io_error(path, error)),
    }
}

/// Coarse classification of a resolved entry, for `read.rs`'s `WindowsReader::inspect_entry`/
/// `list_directory`. Any reparse point classifies as `ReparsePoint` regardless of its specific tag
/// (junction, mount point, or symbolic link) -- distinguishing them needs the reparse tag itself
/// (`FSCTL_GET_REPARSE_POINT`), a raw `DeviceIoControl` call this design does not need and does not
/// take: every caller of this classification treats "some kind of indirection" as the thing to not
/// silently follow, the same coarseness `EntryKind::Symlink` already has on the caller side.
pub(super) enum RawKind {
    File,
    Directory,
    ReparsePoint,
    Other,
}

/// Classify `path`'s final component without following it, `None` (not an error) when absent.
pub(super) fn classify_no_follow(path: &Path) -> Result<Option<RawKind>> {
    match open_no_follow(path, OpenOptions::new().read(true), true) {
        Ok(file) => {
            let metadata = file.metadata().map_err(|error| io_error(path, error))?;
            let kind = if is_reparse_point(&metadata) {
                RawKind::ReparsePoint
            } else if metadata.is_dir() {
                RawKind::Directory
            } else if metadata.is_file() {
                RawKind::File
            } else {
                RawKind::Other
            };
            Ok(Some(kind))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(path, error)),
    }
}

/// Stat `path`'s final component without opening or reading its content, `None` when absent.
/// Refuses a reparse point or non-file the same way `open_existing_file_no_follow` does, since a
/// stat that silently accepted either would be a weaker guarantee than the read path gives.
pub(super) fn stat_file_no_follow(path: &Path) -> Result<Option<std::fs::Metadata>> {
    match open_existing_file_no_follow(path, OpenOptions::new().read(true))? {
        Some(file) => Ok(Some(
            file.metadata().map_err(|error| io_error(path, error))?,
        )),
        None => Ok(None),
    }
}

/// Open an existing regular file, no-follow, refusing a reparse point or non-file. `None` (not an
/// error) when `path` does not exist -- the shape every caller here needs, since "does the entry
/// exist" and "is it the right shape" are different questions with different callers.
pub(super) fn open_existing_file_no_follow(
    path: &Path,
    options: &mut OpenOptions,
) -> Result<Option<File>> {
    match open_no_follow(path, options, false) {
        Ok(file) => {
            let metadata = file.metadata().map_err(|error| io_error(path, error))?;
            if is_reparse_point(&metadata) {
                return Err(PrikkError::Io(format!(
                    "refusing to open a reparse point: {}",
                    path.display()
                )));
            }
            if !metadata.is_file() {
                return Err(PrikkError::Io(format!(
                    "expected a regular file: {}",
                    path.display()
                )));
            }
            Ok(Some(file))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(path, error)),
    }
}

fn required_existing_file_no_follow(path: &Path, options: &mut OpenOptions) -> Result<File> {
    open_existing_file_no_follow(path, options)?
        .ok_or_else(|| PrikkError::Io(format!("required file is absent: {}", path.display())))
}

fn resolved_existing_path(root: &MutationRoot, relative: &Path) -> Result<PathBuf> {
    let parent = required_parent(relative)?;
    let name = required_file_name(relative)?;
    let resolved_parent = open_existing_windows_directory_required(root, parent)?;
    Ok(resolved_parent.join(name))
}

fn resolved_prepared_path(root: &MutationRoot, relative: &Path) -> Result<PathBuf> {
    let parent = required_parent(relative)?;
    let name = required_file_name(relative)?;
    let resolved_parent = prepare_windows_directory_required(root, parent)?;
    Ok(resolved_parent.join(name))
}

/// The Windows durability contract implementation. Zero-sized, matching
/// `LinuxDurability`/`MacosDurability`/`NoDurability`'s shape exactly.
pub(crate) struct WindowsDurability;

impl DurabilityContract for WindowsDurability {
    fn atomic_replace(&self, root: &MutationRoot, relative: &Path, bytes: &[u8]) -> Result<()> {
        // Weaker guarantee, documented rather than approximated (design-v1.md §3.2): `ReplaceFileW`
        // has three documented partial-completion error codes and `REPLACEFILE_WRITE_THROUGH` is
        // documented "not supported"; `MOVEFILE_WRITE_THROUGH`'s same-volume durability guarantee
        // was investigated to three independent primary sources and found genuinely undeterminable
        // (`narrow-round-ruling-v1.md` §1). `std::fs::rename` is used with no durability lever
        // asserted -- content is written and flushed before the rename, but the rename itself is
        // not claimed durable on return. **Acceptable only because this method's remaining
        // callers are the two rebuildable caches** (`commit_index.rs`, `lifecycle_cache.rs`), whose
        // absence or corruption after an interrupted replace changes no result. If a future caller
        // puts durability-bearing state behind this method, that premise no longer holds and this
        // comment -- not the reader's expectations -- is what has to change.
        let parent = required_parent(relative)?;
        let name = required_file_name(relative)?;
        let resolved_parent = prepare_windows_directory_required(root, parent)?;
        let destination = resolved_parent.join(name);
        let temp_path = temporary_path(relative)?;
        let temp_name = required_file_name(&temp_path)?;
        let temp_full = resolved_parent.join(temp_name);
        let mut file = open_no_follow(
            &temp_full,
            OpenOptions::new().write(true).create(true).truncate(true),
            false,
        )
        .map_err(|error| io_error(&temp_full, error))?;
        file.write_all(bytes)
            .map_err(|error| io_error(&temp_full, error))?;
        file.sync_all()
            .map_err(|error| io_error(&temp_full, error))?;
        drop(file);
        fs::rename(&temp_full, &destination).map_err(|error| io_error(&destination, error))
    }

    fn durable_append(&self, root: &MutationRoot, relative: &Path, bytes: &[u8]) -> Result<()> {
        let path = resolved_existing_path(root, relative)?;
        let mut file = required_existing_file_no_follow(&path, OpenOptions::new().append(true))?;
        file.write_all(bytes)
            .map_err(|error| io_error(&path, error))?;
        file.sync_all().map_err(|error| io_error(&path, error))
    }

    fn durable_truncate(&self, root: &MutationRoot, relative: &Path, len: u64) -> Result<()> {
        let path = resolved_existing_path(root, relative)?;
        let file = required_existing_file_no_follow(&path, OpenOptions::new().write(true))?;
        file.set_len(len).map_err(|error| io_error(&path, error))?;
        file.sync_all().map_err(|error| io_error(&path, error))
    }

    fn durable_truncate_to_empty(&self, root: &MutationRoot, relative: &Path) -> Result<()> {
        self.durable_truncate(root, relative, 0)
    }

    fn create_exclusive(
        &self,
        root: &MutationRoot,
        relative: &Path,
        bytes: &[u8],
    ) -> std::io::Result<()> {
        let path = resolved_prepared_path(root, relative).map_err(prikk_to_io)?;
        let mut file = open_no_follow(
            &path,
            OpenOptions::new().write(true).create_new(true),
            false,
        )?;
        file.write_all(bytes)?;
        file.sync_all()
    }

    fn set_permission_bits(&self, root: &MutationRoot, relative: &Path, mode: u32) -> Result<()> {
        // Documented no-op (design-v1.md §3.3, ruled 2026-08-16). NTFS has no POSIX execute bit --
        // executability is determined by file extension and association, not filesystem metadata
        // `chmod` could set. Prikk records `mode` internally and never derives it from the
        // filesystem (the DC-87 mode-carrying fix, merged before this stage), so a round-trip --
        // authored here, checked out again on Linux -- restores the node's own recorded mode
        // faithfully regardless of what this method does or does not touch on disk. NTFS ACLs
        // could carry an execute permission, but `SetNamedSecurityInfo`-class APIs are a materially
        // larger, security-sensitive surface for a property nothing reads back -- ruled against.
        let _ = (root, relative, mode);
        Ok(())
    }

    fn remove_if_present(&self, root: &MutationRoot, relative: &Path) -> Result<bool> {
        let parent = required_parent(relative)?;
        let name = required_file_name(relative)?;
        let resolved_parent = open_existing_windows_directory_required(root, parent)?;
        let path = resolved_parent.join(name);
        match fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(io_error(&path, error)),
        }
    }

    fn promote(&self, root: &MutationRoot, source: &Path, destination: &Path) -> Result<()> {
        // Weaker guarantee, documented rather than approximated, and unreachable in production
        // (zero callers, DC-87 §0). Same rename-durability caveat `atomic_replace` states above --
        // `std::fs::rename` gives type-matching, overwrite-on-supported-Windows-versions semantics
        // with no asserted durability lever.
        let source_parent = required_parent(source)?;
        let source_name = required_file_name(source)?;
        let resolved_source_parent = open_existing_windows_directory_required(root, source_parent)?;
        let destination_parent = required_parent(destination)?;
        let destination_name = required_file_name(destination)?;
        let resolved_destination_parent =
            open_existing_windows_directory_required(root, destination_parent)?;
        let source_path = resolved_source_parent.join(source_name);
        let destination_path = resolved_destination_parent.join(destination_name);
        fs::rename(&source_path, &destination_path)
            .map_err(|error| io_error(&destination_path, error))
    }

    fn publish_immutable(
        &self,
        root: &MutationRoot,
        relative: &Path,
        candidate: &[u8],
        validate_existing: impl Fn(&[u8]) -> Result<()>,
    ) -> Result<()> {
        // Weaker guarantee, documented rather than approximated, and unreachable in production
        // (zero callers, DC-87 §0's standing G5 orphan finding). `std::fs::hard_link` maps to
        // `CreateHardLinkW`, which -- like POSIX `linkat` -- fails if the destination name already
        // exists, giving the same no-clobber install shape the Linux implementation uses.
        let parent = required_parent(relative)?;
        let name = required_file_name(relative)?;
        let resolved_parent = prepare_windows_directory_required(root, parent)?;
        let destination = resolved_parent.join(name);
        if let Some(existing) = read_existing_regular(&destination)? {
            validate_existing(&existing)?;
            if existing != candidate {
                return Err(PrikkError::Integrity(
                    "existing immutable object bytes differ from candidate".to_string(),
                ));
            }
            return Ok(());
        }

        let temp_path = temporary_path(relative)?;
        let temp_name = required_file_name(&temp_path)?;
        let temp_full = resolved_parent.join(temp_name);
        let mut file = open_no_follow(
            &temp_full,
            OpenOptions::new().write(true).create_new(true),
            false,
        )
        .map_err(|error| io_error(&temp_full, error))?;
        file.write_all(candidate)
            .map_err(|error| io_error(&temp_full, error))?;
        file.sync_all()
            .map_err(|error| io_error(&temp_full, error))?;
        drop(file);

        match fs::hard_link(&temp_full, &destination) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&temp_full);
                let Some(existing) = read_existing_regular(&destination)? else {
                    return Err(PrikkError::Integrity(
                        "no-clobber install reported an absent winner".to_string(),
                    ));
                };
                validate_existing(&existing)?;
                if existing != candidate {
                    return Err(PrikkError::Integrity(
                        "existing immutable object bytes differ from candidate".to_string(),
                    ));
                }
                return Ok(());
            }
            Err(error) => return Err(io_error(&destination, error)),
        }
        fs::remove_file(&temp_full).map_err(|error| io_error(&temp_full, error))
    }

    fn ensure_directory(&self, root: &MutationRoot, relative: &Path) -> Result<()> {
        prepare_windows_directory_required(root, relative)?;
        Ok(())
    }

    fn durable_directory_entry(&self, root: &MutationRoot, relative: &Path) -> Result<()> {
        // Documented no-op (design-v1.md §3.4). `FlushFileBuffers`'s own documentation covers file,
        // communications-device, named-pipe, and volume handles and says nothing about a directory
        // handle -- there is no contract to implement against (corroborated: a production key-value
        // store hit `ERROR_INVALID_FUNCTION` on SMB and unverifiable silent success on local NTFS
        // attempting exactly this). Safe as a no-op because this method's only two production
        // callers (`worktree.rs:151`, `:199`) sit inside the unclean-shutdown marker's bracket
        // (`worktree_marker.rs`): a crash between this call and the entry actually becoming durable
        // leaves the marker dirty, and commit-authoring refuses to infer deletion from worktree
        // absence until the worktree is re-verified. Worst case is a spurious refusal, never a
        // silent wrong inference.
        let _ = (root, relative);
        Ok(())
    }
}

fn read_existing_regular(path: &Path) -> Result<Option<Vec<u8>>> {
    let Some(mut file) = open_existing_file_no_follow(path, OpenOptions::new().read(true))? else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| io_error(path, error))?;
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests;
