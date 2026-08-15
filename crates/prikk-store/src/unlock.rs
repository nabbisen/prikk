//! Stale-lock recovery (RFC 102 Stage 6 Step 2, design-v1.md §15.7 decision 3, handoff §4):
//! `lock.rs::lock_body`'s own text says it outright -- *"note=PR-007 lock has no stale-lock stealing
//! yet"* -- and a lock file surviving a crash wedges that lock permanently. `doctor.rs:405`'s own
//! repair path acquires `ActiveLock`, so the tool meant to repair the repository was blocked by the
//! very thing needing repair, with no recovery until this module.
//!
//! **Rejected: PID-based auto-stealing.** The tempting design is "if the recorded `pid=` isn't
//! running, the lock is safe to steal automatically." The two failure directions are not symmetric:
//!
//! - **False negative** (stale lock, liveness check says "still running"): no worse than today's
//!   permanent wedge. Annoying, not dangerous.
//! - **False positive** (lock genuinely held, liveness check wrongly says "not running", auto-steals
//!   it): **two writers now believe they hold exclusive access to the same container simultaneously**
//!   -- the exact race Step 2 exists to close, reintroduced by the mechanism meant to keep the
//!   repository usable. PID reuse after a reboot, and PID-namespace isolation across containers (a
//!   process id meaningful inside one container's namespace is not the same number space the host or
//!   a different container sees), both make this a real, not theoretical, failure path for a tool
//!   whose deployment context includes CI containers.
//!
//! An auto-stealing mechanism whose failure mode is silent data corruption, built to fix a failure
//! mode that is merely inconvenient, is the wrong trade.
//!
//! **So this module never removes a lock on its own.** `list_held_locks` only enumerates and reports;
//! `clear_lock` only removes the exact path it is given, once. Prompting for confirmation and deciding
//! whether the `--yes`/`--force` scripting escape applies are `prikk unlock`'s own job (`prikk-cli`),
//! not this module's -- keeping the decision in the caller that can actually see a terminal keeps this
//! module a pure, easily-tested primitive.
//!
//! **The liveness check is advisory, and the asymmetry in how it is trusted is the whole point:** a
//! *positive* result (`AppearsRunning`) is reliable evidence to refuse -- `kill(pid, 0)` succeeding, or
//! failing with `EPERM`, both mean the process genuinely exists. A *negative* result
//! (`DoesNotAppearRunning`) is **not** evidence the lock is safe to clear, for the PID-reuse/namespace
//! reasons above -- it is information for the operator, never authorization for the tool.

use std::path::{Path, PathBuf};

use prikk_error::Result;

use crate::fsutil::{EntryKind, list_directory, read_file_if_exists, remove_file_required};
use crate::layout::{LockableContainer, RepositoryLayout};

/// Best-effort, advisory-only liveness of a lock's recorded `pid=`. See the module doc for why a
/// negative or unknown result must never be treated as authorization to clear the lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidLiveness {
    /// The recorded PID genuinely exists on this host right now -- reliable: refuse to clear.
    AppearsRunning,
    /// The recorded PID does not appear to exist on this host -- **not proof it is safe to clear**
    /// (PID reuse, container namespace isolation both make "not found here" compatible with "still
    /// running somewhere that matters").
    DoesNotAppearRunning,
    /// The check could not be performed (unparseable `pid=` value, or no liveness primitive on this
    /// platform).
    Unknown,
}

/// One lock file found on disk, parsed from its own body (`lock.rs::lock_body`'s format:
/// `pid=<n>\nkind=<k>\nnote=...\n`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldLock {
    /// The lock file's own path -- pass this to `clear_lock` to remove it.
    pub path: PathBuf,
    /// The `kind=` field recorded in the lock body (`"active"`, `"ref"`, or one of
    /// `lock::container_lock_kind`'s strings).
    pub kind: String,
    /// The `pid=` field, if the body parsed cleanly.
    pub recorded_pid: Option<u32>,
    /// Best-effort, advisory liveness of `recorded_pid` -- see `PidLiveness`'s own doc.
    pub liveness: PidLiveness,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn check_pid_liveness(pid: u32) -> PidLiveness {
    let Ok(raw) = i32::try_from(pid) else {
        return PidLiveness::Unknown;
    };
    let Some(rustix_pid) = rustix::process::Pid::from_raw(raw) else {
        return PidLiveness::Unknown;
    };
    match rustix::process::test_kill_process(rustix_pid) {
        Ok(()) => PidLiveness::AppearsRunning,
        // `EPERM` means the kernel found a process to check permissions against -- it exists, this
        // caller simply cannot signal it. That is still existence, not absence.
        Err(rustix::io::Errno::PERM) => PidLiveness::AppearsRunning,
        Err(rustix::io::Errno::SRCH) => PidLiveness::DoesNotAppearRunning,
        Err(_) => PidLiveness::Unknown,
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn check_pid_liveness(_pid: u32) -> PidLiveness {
    PidLiveness::Unknown
}

fn parse_lock_body(bytes: &[u8]) -> (String, Option<u32>) {
    let body = String::from_utf8_lossy(bytes);
    let mut kind = String::new();
    let mut recorded_pid = None;
    for line in body.lines() {
        if let Some(value) = line.strip_prefix("kind=") {
            kind = value.to_string();
        } else if let Some(value) = line.strip_prefix("pid=") {
            recorded_pid = value.parse::<u32>().ok();
        }
    }
    (kind, recorded_pid)
}

fn read_lock_if_present(layout: &RepositoryLayout, path: &Path) -> Result<Option<HeldLock>> {
    let relative = layout.repository_relative(path)?;
    let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &relative)? else {
        return Ok(None);
    };
    let (kind, recorded_pid) = parse_lock_body(&bytes);
    let liveness = recorded_pid.map_or(PidLiveness::Unknown, check_pid_liveness);
    Ok(Some(HeldLock {
        path: path.to_path_buf(),
        kind,
        recorded_pid,
        liveness,
    }))
}

/// Enumerate every lock file currently present: the active-session lock, every per-ref lock, and
/// every one of the four container locks. Read-only -- never clears anything, matching the module's
/// own "enumerate and report, never decide" split.
pub fn list_held_locks(layout: &RepositoryLayout) -> Result<Vec<HeldLock>> {
    let mut locks = Vec::new();

    if let Some(lock) = read_lock_if_present(layout, &layout.default_active_lock_path())? {
        locks.push(lock);
    }

    let ref_locks_dir = layout.refs_dir().join("locks");
    let ref_locks_relative = layout.repository_relative(&ref_locks_dir)?;
    for entry in list_directory(layout.repository_mutation_root(), &ref_locks_relative)? {
        if entry.kind != EntryKind::Regular {
            continue;
        }
        let path = ref_locks_dir.join(&entry.name);
        if let Some(lock) = read_lock_if_present(layout, &path)? {
            locks.push(lock);
        }
    }

    for container in LockableContainer::ALL {
        if let Some(lock) =
            read_lock_if_present(layout, &layout.lockable_container_lock_path(container))?
        {
            locks.push(lock);
        }
    }

    Ok(locks)
}

/// Clear one specific lock file by path. The caller (`prikk unlock`) is responsible for obtaining
/// operator confirmation before calling this -- this function performs no confirmation, no liveness
/// check, and no safety gate of its own: by the time it is called, the decision has already been made
/// by a human who read `list_held_locks`'s own advisory. Removing a lock that is still genuinely held
/// lets two writers race the container it names -- that is the whole risk this module exists to keep
/// an operator, not a heuristic, deciding.
pub fn clear_lock(layout: &RepositoryLayout, path: &Path) -> Result<()> {
    let relative = layout.repository_relative(path)?;
    remove_file_required(layout.repository_mutation_root(), &relative)
}

#[cfg(test)]
mod tests;
