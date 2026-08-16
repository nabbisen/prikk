//! Root-scoped filesystem mutation primitives. Every function here is a thin, **unconditional** call
//! through the durability contract (DC-76, `super::contract::DurabilityContract`) — the guarantee
//! each one provides is stated on the trait method it calls, not repeated here. `Linux`
//! (`linux::LinuxDurability`), `Macos` (`macos::MacosDurability`, DC-81), `Windows`
//! (`windows::WindowsDurability`, DC-87 Stage 2), and `NoDurability` (`none::NoDurability`, every
//! method an "unsupported" error, for every remaining target) are the implementors.
//! `ACTIVE_DURABILITY` below is the single gated constant that picks among them; no `target_os` gate
//! appears at any call site in this file (DC-82's bar) — a further platform is one more `#[cfg]` arm
//! on `ACTIVE_DURABILITY`, not one more arm at every one of these eleven functions.

use std::path::Path;

use prikk_error::{PrikkError, Result};

mod directory;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod failpoints;
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod immutable;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(
    all(test, not(target_os = "windows")),
    not(any(target_os = "linux", target_os = "macos", target_os = "windows"))
))]
mod none;
mod read;
mod regular;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
mod windows_authority;

pub(crate) use directory::MutationRoot;
pub(crate) use read::{
    EntryKind, RootFileStat, inspect_entry, list_directory, read_file_if_exists,
    read_file_required, stat_file_state_if_exists,
};

use crate::fsutil::contract::DurabilityContract;
#[cfg(target_os = "linux")]
pub(crate) use linux::LinuxDurability;
#[cfg(target_os = "macos")]
pub(crate) use macos::MacosDurability;
#[cfg(any(
    all(test, not(target_os = "windows")),
    not(any(target_os = "linux", target_os = "macos", target_os = "windows"))
))]
pub(crate) use none::NoDurability;
#[cfg(target_os = "windows")]
pub(crate) use windows::WindowsDurability;

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
pub(crate) use failpoints::{
    Point as TestFailPoint, fail_after as fail_after_for_test, fail_once as fail_once_for_test,
    set_directory_create_barrier as set_directory_create_barrier_for_test,
    set_immutable_install_barrier as set_immutable_install_barrier_for_test,
};

/// The one gated symbol DC-82 exists to introduce: picks the active `DurabilityContract`
/// implementor for this build. Every function below calls through it unconditionally.
#[cfg(target_os = "linux")]
const ACTIVE_DURABILITY: LinuxDurability = LinuxDurability;
#[cfg(target_os = "macos")]
const ACTIVE_DURABILITY: MacosDurability = MacosDurability;
#[cfg(target_os = "windows")]
const ACTIVE_DURABILITY: WindowsDurability = WindowsDurability;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
const ACTIVE_DURABILITY: NoDurability = NoDurability;

/// Write mutable metadata through a unique same-directory temporary file.
pub(crate) fn write_file_atomically(
    root: &MutationRoot,
    relative: &Path,
    bytes: &[u8],
) -> Result<()> {
    ACTIVE_DURABILITY.atomic_replace(root, relative, bytes)
}

/// Write a worktree file through its retained worktree-root authority.
pub(crate) fn write_worktree_file_atomically(
    root: &MutationRoot,
    relative: &Path,
    bytes: &[u8],
) -> Result<()> {
    write_file_atomically(root, relative, bytes)
}

/// Append bytes, sync the file, and always re-establish retained-parent durability.
pub(crate) fn append_file_required(
    root: &MutationRoot,
    relative: &Path,
    bytes: &[u8],
) -> Result<()> {
    ACTIVE_DURABILITY.durable_append(root, relative, bytes)
}

/// Truncate an existing regular file to a retained length and sync its parent.
pub(crate) fn truncate_existing_file_required(
    root: &MutationRoot,
    relative: &Path,
    len: u64,
) -> Result<()> {
    ACTIVE_DURABILITY.durable_truncate(root, relative, len)
}

/// Set an existing regular file's mode bits (DC-73: worktree materialization needs to write the
/// mode a `CreateFile`/`ChangePerm` operation recorded, not whatever the anchored create primitive
/// defaults new files to). No-follow, matching every other anchored open — a symlink at the final
/// component is refused rather than chmod'd through.
pub(crate) fn set_regular_file_mode_required(
    root: &MutationRoot,
    relative: &Path,
    mode: u32,
) -> Result<()> {
    ACTIVE_DURABILITY.set_permission_bits(root, relative, mode)
}

/// Create or truncate a regular file, then sync it and its retained parent.
pub(crate) fn truncate_file_empty_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    ACTIVE_DURABILITY.durable_truncate_to_empty(root, relative)
}

/// Create, write, and durably publish an exclusive regular file.
pub(crate) fn create_new_file_required(
    root: &MutationRoot,
    relative: &Path,
    bytes: &[u8],
) -> std::io::Result<()> {
    ACTIVE_DURABILITY.create_exclusive(root, relative, bytes)
}

/// Remove a file and sync the exact parent handle that owned the unlink.
pub(crate) fn remove_file_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    remove_file_if_present_required(root, relative).map(|_| ())
}

/// Remove a file if present and sync its exact parent even for observed absence.
pub(crate) fn remove_file_if_present_required(
    root: &MutationRoot,
    relative: &Path,
) -> Result<bool> {
    ACTIVE_DURABILITY.remove_if_present(root, relative)
}

/// Attempt fallible removal where a destructor cannot report the result.
pub(crate) fn remove_file_cleanup_best_effort(root: &MutationRoot, relative: &Path) {
    let _ = remove_file_required(root, relative);
}

/// Remove a worktree file through its retained worktree-root authority.
pub(crate) fn remove_worktree_file_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    remove_file_required(root, relative)
}

/// Rename within one root, syncing destination before source.
///
/// RFC 102 Stage 4: orphaned by the same shape of rewire that orphaned G5 in Stage 3 --
/// `refs/publication.rs`'s candidate-write-then-promote dance (`refs/pointer.rs`'s old
/// `write_ref_pointer_candidate`/`promote_ref_pointer_candidate`) was this function's only
/// production caller, and Step 0 §13.3's ruling retired that whole mechanism (an append-only
/// pointer record has no candidate value to stage). Kept, not deleted: retiring a durability
/// primitive is the same RFC-level act it was for G5, not a stage side effect. Reported, not
/// decided -- see the review submission.
#[allow(dead_code)]
pub(crate) fn promote_file_required(
    root: &MutationRoot,
    source: &Path,
    destination: &Path,
) -> Result<()> {
    ACTIVE_DURABILITY.promote(root, source, destination)
}

/// Publish immutable, content-addressed bytes at `relative` without ever replacing existing
/// content — see `DurabilityContract::publish_immutable` for the guarantee.
///
/// RFC 102 Stage 3, design-v1.md §12.3: G5 has no production caller now that object writes go
/// through `index.rs`'s container append protocol instead. Kept as the clean, cross-platform entry
/// point `object_store/tests/immutable.rs` and `races.rs` re-target onto directly (naming
/// `LinuxDurability`/`MacosDurability`/`WindowsDurability` by hand in a test that runs on all three
/// would defeat the point of `ACTIVE_DURABILITY` picking the right one). Gated to match
/// `object_store::tests`' own gate exactly (`object_store.rs:123`), not just `#[cfg(test)]` -- DC-97
/// widened both to include Windows once `object_store.rs:123`'s own Linux/macOS-only reasoning was
/// found stale (it predated DC-87 making Windows a mutating platform). A bare `#[cfg(test)]` here
/// would leave this genuinely unused, and `-D warnings` genuinely dead, on whichever platform its
/// only callers don't compile for (`EXECUTION-ORDER.md` §6 rule 9's own cross-target clippy check,
/// added for exactly this class of platform-conditional dead code).
#[cfg(all(
    test,
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
pub(crate) fn publish_immutable_file(
    root: &MutationRoot,
    relative: &Path,
    candidate: &[u8],
    validate_existing: impl Fn(&[u8]) -> Result<()>,
) -> Result<()> {
    ACTIVE_DURABILITY.publish_immutable(root, relative, candidate, validate_existing)
}

/// Ensure a relative directory tree exists, durably, tolerating a concurrent creator (G8).
pub(crate) fn ensure_directory_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    ACTIVE_DURABILITY.ensure_directory(root, relative)
}

/// Durably confirm that `relative` — an existing regular file — is recorded in its containing
/// directory (DC-88). `relative` names the entry to confirm, not the directory to sync.
pub(crate) fn sync_directory_required(root: &MutationRoot, relative: &Path) -> Result<()> {
    ACTIVE_DURABILITY.durable_directory_entry(root, relative)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn io_error(error: rustix::io::Errno) -> PrikkError {
    PrikkError::from(std::io::Error::from(error))
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn prikk_to_io(error: PrikkError) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

/// Shared by `directory.rs`'s `PathOnlyAuthority` (`MutationRoot::ensure_root`'s fallback,
/// unrelated to the durability dispatch above) and `none::NoDurability` (every method). Gated to
/// match both callers' own gate exactly — visible in test builds on every target that is not
/// Windows (Windows has its own real authority and implementor, in test builds too), and for real
/// on every target that is none of Linux, macOS, or Windows.
#[cfg(any(
    all(test, not(target_os = "windows")),
    not(any(target_os = "linux", target_os = "macos", target_os = "windows"))
))]
fn unsupported_mutation<T>() -> Result<T> {
    Err(PrikkError::Io(
        "repository mutation requires Linux, macOS, or Windows root-scoped filesystem \
         capabilities"
            .to_string(),
    ))
}
