#![allow(clippy::indexing_slicing)]

use prikk_error::Result;

use super::{PidLiveness, clear_lock, list_held_locks};
use crate::RepositoryLayout;
use crate::layout::LockableContainer;
use crate::lock::{ActiveLock, RefLock, acquire_container_locks};
use crate::test_support::unique_temp_dir;

#[test]
fn a_fresh_repository_holds_no_locks() -> Result<()> {
    let root = unique_temp_dir("unlock-empty");
    let layout = RepositoryLayout::init(root.clone())?;
    assert!(list_held_locks(&layout)?.is_empty());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// Every lock kind is enumerated, and the current process's own PID (definitely alive, it is the
/// process running this test) is correctly reported as `AppearsRunning` -- the one case the advisory
/// check can prove reliably rather than merely claim.
#[test]
fn every_held_lock_kind_is_enumerated_with_its_own_pid_live() -> Result<()> {
    let root = unique_temp_dir("unlock-enumerate-all-kinds");
    let layout = RepositoryLayout::init(root.clone())?;

    let active = ActiveLock::acquire(&layout)?;
    let ref_lock = RefLock::acquire(&layout, "heads/main")?;
    let container_lock = acquire_container_locks(&layout, &[LockableContainer::TrustPolicy])?;

    let locks = list_held_locks(&layout)?;
    assert_eq!(locks.len(), 3);
    for lock in &locks {
        assert_eq!(lock.recorded_pid, Some(std::process::id()));
        assert_eq!(lock.liveness, PidLiveness::AppearsRunning);
    }
    let kinds: Vec<&str> = locks.iter().map(|lock| lock.kind.as_str()).collect();
    assert!(kinds.contains(&"active"));
    assert!(kinds.contains(&"ref"));
    assert!(kinds.contains(&"container:trust-policy"));

    drop(active);
    drop(ref_lock);
    drop(container_lock);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// A lock recording a PID that does not exist on this host is reported `DoesNotAppearRunning` --
/// advisory, not a claim the lock is safe to clear (see the module's own doc for why).
#[test]
fn a_lock_recording_a_nonexistent_pid_is_reported_as_not_appearing_to_run() -> Result<()> {
    let root = unique_temp_dir("unlock-dead-pid");
    let layout = RepositoryLayout::init(root.clone())?;
    let path = layout.default_active_lock_path();
    // A PID within the valid range but astronomically unlikely to name a real process in a test
    // environment -- `test_kill_process` returns `ESRCH` for it.
    std::fs::write(&path, "pid=999999\nkind=active\nnote=test\n")?;

    let locks = list_held_locks(&layout)?;
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].recorded_pid, Some(999_999));
    assert_eq!(locks[0].liveness, PidLiveness::DoesNotAppearRunning);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// `clear_lock` removes exactly the path it is given, no confirmation or liveness check of its own --
/// that is the caller's job, per the module's own "enumerate and report, decide elsewhere" split. After
/// clearing, the lock is re-acquirable.
#[test]
fn clear_lock_removes_the_named_lock_and_it_becomes_reacquirable() -> Result<()> {
    let root = unique_temp_dir("unlock-clear");
    let layout = RepositoryLayout::init(root.clone())?;
    let active = ActiveLock::acquire(&layout)?;
    let path = active.path().to_path_buf();
    // Simulate a crash: the lock file survives, but nothing still holds it in-process.
    std::mem::forget(active);

    assert!(ActiveLock::acquire(&layout).is_err());
    clear_lock(&layout, &path)?;
    assert!(list_held_locks(&layout)?.is_empty());
    assert!(ActiveLock::acquire(&layout).is_ok());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// A missing `pid=` line (malformed body) is `Unknown`, not misread as any specific PID -- fails
/// closed on ambiguity rather than guessing.
#[test]
fn a_malformed_lock_body_reports_unknown_liveness() -> Result<()> {
    let root = unique_temp_dir("unlock-malformed-body");
    let layout = RepositoryLayout::init(root.clone())?;
    let path = layout.default_active_lock_path();
    std::fs::write(&path, "not a lock body at all")?;

    let locks = list_held_locks(&layout)?;
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].recorded_pid, None);
    assert_eq!(locks[0].liveness, PidLiveness::Unknown);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-41 crash window 4, end to end: a container lock survives a simulated crash (the in-process
/// guard never runs `Drop`, matching what a real crash leaves behind), wedging both the compactor and
/// its writer -- and this module's own recovery path is what un-wedges it, not a special case for
/// container locks. Ties `unlock.rs` and `compact.rs` together rather than testing either in
/// isolation, since that is the actual recovery story an operator lives through.
#[test]
fn a_wedged_container_lock_blocks_compaction_until_unlock_clears_it() -> Result<()> {
    let root = unique_temp_dir("unlock-wedged-container-lock");
    let layout = RepositoryLayout::init(root.clone())?;

    let held = acquire_container_locks(&layout, &[LockableContainer::RefPointerIndex])?;
    let path = layout.lockable_container_lock_path(LockableContainer::RefPointerIndex);
    // Simulate a crash: the lock file survives, but nothing still holds it in-process -- exactly
    // what a real crash leaves, not a synthetic shortcut.
    std::mem::forget(held);

    assert!(crate::compact_ref_pointer_index(&layout).is_err());
    assert!(crate::plan_compact_ref_pointer_index(&layout).is_err());

    let locks = list_held_locks(&layout)?;
    assert_eq!(locks.len(), 1);
    assert_eq!(locks[0].path, path);
    assert_eq!(locks[0].kind, "container:ref-pointer-index");

    clear_lock(&layout, &path)?;
    assert!(list_held_locks(&layout)?.is_empty());

    let report = crate::compact_ref_pointer_index(&layout)?;
    assert_eq!(report.entries_before, 0);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
