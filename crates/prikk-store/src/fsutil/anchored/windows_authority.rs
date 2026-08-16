//! `WindowsAuthority`'s own module (DC-96 Windows Anchor Identity). Moved out of `directory.rs`
//! so its fields are genuinely private -- it used to share that module with the read-side walker
//! (`open_existing_windows_directory_for_read`), and field privacy binds at module granularity, so
//! privacy bought nothing while both lived there. **This module exposes no way to obtain a base
//! path from an authority without first verifying it is still the object it was bound to.** Every
//! resolver below (`resolve_existing`/`resolve_prepared`/`resolve_existing_for_read`) and both
//! `PlatformAuthority` walks (`ensure_child`/`open_child`) call [`WindowsAuthority::verify_anchor`]
//! before touching `self.path` -- there is no path accessor a future fourth walker could reach for
//! instead. That is what makes this a control rather than a convention (the distinction DC-90's own
//! module doc draws, applied one layer down): a second call site could be forgotten; a missing
//! accessor cannot compile around.
//!
//! **Detection, not prevention** (design-v1.md §5). Windows has no `openat` -- there is no
//! primitive that resolves a child by name against an already-open directory handle the way Linux
//! and macOS do. `verify_anchor` re-opens `self.path` and compares its identity
//! (`GetFileInformationByHandle`'s `(volume serial, file index)` pair, `prikk-ffi`) against the
//! identity captured when this authority was bound. A mismatch means the directory at that path is
//! not the one this authority validated -- refused, not silently followed. What this closes:
//! **anchor replacement between operations.** What it does not: a replacement racing the single
//! verify-then-open pair (the window is narrowed, not closed), and G1's already-documented
//! mid-walk reparse-point race (`windows.rs`'s own module doc), which this module does not touch.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use prikk_error::{PrikkError, Result};

use super::directory::{PlatformAuthority, relative_components};
use super::windows;

/// No retained handle -- a `File` would look like the Linux design and buy nothing, since Windows
/// cannot resolve a child through a directory handle, and an inert retained resource invites the
/// belief that it provides a guarantee it does not. `identity` is what actually stands in for it:
/// a value, re-checked against a fresh open on every use, not a capability held across calls.
#[derive(Clone)]
pub(super) struct WindowsAuthority {
    path: PathBuf,
    identity: prikk_ffi::FileIdentity,
}

impl WindowsAuthority {
    /// Re-open `self.path` no-follow, read its current identity, and fail closed if it no longer
    /// matches the identity captured when this authority was bound or last walked to. Called
    /// before every walk in this module, **before** the component loop when there is one -- a
    /// relative path with zero components (a file directly in the anchor) must still be checked,
    /// or the empty-relative case -- the failing tests' own exact shape -- would stay unverified.
    fn verify_anchor(&self) -> Result<()> {
        let file = windows::open_directory_no_follow(&self.path)?;
        let current = windows::identity_no_follow(&file, &self.path)?;
        if current != self.identity {
            return Err(PrikkError::Integrity(format!(
                "Windows anchor replaced: {} no longer identifies the directory this authority \
                 was bound to",
                self.path.display()
            )));
        }
        Ok(())
    }

    /// Resolve `relative` (creating any missing component) against this authority, verified
    /// first. Mirrors `prepare_directory_required`'s guarantee on the Unix side.
    pub(super) fn resolve_prepared(&self, relative: &Path) -> Result<PathBuf> {
        self.verify_anchor()?;
        let mut current = self.path.clone();
        for component in relative_components(relative)? {
            current.push(component);
            windows::ensure_directory_component_no_follow(&current)?;
        }
        Ok(current)
    }

    /// Resolve `relative` against this authority, requiring every component to already exist,
    /// verified first. Mirrors `open_existing_directory_required`.
    pub(super) fn resolve_existing(&self, relative: &Path) -> Result<PathBuf> {
        self.verify_anchor()?;
        let mut current = self.path.clone();
        for component in relative_components(relative)? {
            current.push(component);
            windows::open_directory_no_follow(&current)?;
        }
        Ok(current)
    }

    /// Resolve `relative` against this authority, verified first, returning `None` (not an error)
    /// as soon as any component is absent. Mirrors `open_existing_directory_for_read`. **Keeps
    /// the tolerant contract byte for byte** -- every `WindowsReader` method depends on `None`
    /// meaning "does not exist," not "error" -- this moved module, not meaning; unifying it with
    /// `resolve_existing`'s required contract is explicitly out of this increment's scope.
    pub(super) fn resolve_existing_for_read(&self, relative: &Path) -> Result<Option<PathBuf>> {
        self.verify_anchor()?;
        let mut current = self.path.clone();
        for component in relative_components(relative)? {
            current.push(component);
            if windows::stat_directory_no_follow(&current)?.is_none() {
                return Ok(None);
            }
        }
        Ok(Some(current))
    }
}

impl PlatformAuthority for WindowsAuthority {
    fn bind(path: &Path) -> Result<Self> {
        // Capture identity from the same handle `open_directory_no_follow` already validated --
        // not a second open, which would be a fresh race inside the constructor.
        let file = windows::open_directory_no_follow(path)?;
        let identity = windows::identity_no_follow(&file, path)?;
        Ok(Self {
            path: path.to_path_buf(),
            identity,
        })
    }

    fn same_as(&self, _self_path: &Arc<PathBuf>, other: &Self, _other_path: &Arc<PathBuf>) -> bool {
        self.path == other.path && self.identity == other.identity
    }

    fn ensure_child(&self, relative: &Path) -> Result<Self> {
        self.verify_anchor()?;
        let mut current = self.path.clone();
        let mut identity = self.identity;
        for component in relative_components(relative)? {
            current.push(component);
            let file = windows::ensure_directory_component_no_follow(&current)?;
            identity = windows::identity_no_follow(&file, &current)?;
        }
        Ok(Self {
            path: current,
            identity,
        })
    }

    fn open_child(&self, relative: &Path) -> Result<Self> {
        self.verify_anchor()?;
        let mut current = self.path.clone();
        let mut identity = self.identity;
        for component in relative_components(relative)? {
            current.push(component);
            let file = windows::open_directory_no_follow(&current)?;
            identity = windows::identity_no_follow(&file, &current)?;
        }
        Ok(Self {
            path: current,
            identity,
        })
    }
}

#[cfg(test)]
mod tests;
