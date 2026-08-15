//! `prikk unlock` — stale-lock recovery (RFC 102 Stage 6 Step 2, design-v1.md §15.7 decision 3).
//!
//! Bare invocation only lists what is currently held — it never clears anything. Clearing one
//! specific lock (`--lock <path>`) always requires either an interactive `yes` confirmation or the
//! scripting escape `--yes`; there is no default that removes a lock. The tool never decides a lock
//! is safe to clear on its own -- see `prikk_store::unlock`'s own module doc for why (PID-based
//! auto-stealing was considered and rejected: a false positive there means two writers on one
//! container simultaneously, the exact race Step 2 exists to close).

use std::io::Write as _;
use std::path::PathBuf;

use prikk_store::{HeldLock, PidLiveness, clear_lock, find_held_lock, list_held_locks};

pub(crate) fn run_unlock(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    let mut args = args.into_iter();
    let mut target: Option<String> = None;
    let mut skip_confirmation = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--lock" => {
                target = Some(
                    args.next()
                        .ok_or_else(|| "--lock requires a path".to_string())?,
                );
            }
            "--yes" | "--force" => skip_confirmation = true,
            other => return Err(format!("unknown unlock argument: {other}")),
        }
    }

    let layout = crate::open_repository(root)?;
    let locks = list_held_locks(&layout).map_err(|err| err.to_string())?;

    let Some(target) = target else {
        print_locks(&locks);
        return Ok(());
    };

    // `find_held_lock` resolves both sides before comparing (a path reached through a symlinked temp
    // dir, home, or mount must still match) -- but `print_locks` below already emits every path in its
    // resolved form, so an operator who copies straight from a listing never needed this in the first
    // place. This only ever bites a path constructed independently of that listing.
    let target_path = PathBuf::from(&target);
    let Some(lock) = find_held_lock(&locks, &target_path) else {
        // Name the resolution explicitly when it changes something, rather than silently comparing
        // resolved forms behind the scenes and leaving an operator to wonder why a path that "looks
        // right" still didn't match -- if it resolves to something real, say what, so a raw/resolved
        // mismatch is visible instead of hidden.
        let resolved_note = match std::fs::canonicalize(&target_path) {
            Ok(resolved) if resolved != target_path => {
                format!(
                    " (resolves to {}, which also has no held lock)",
                    resolved.display()
                )
            }
            _ => String::new(),
        };
        return Err(format!(
            "no held lock at {target}{resolved_note} -- run `prikk unlock` with no arguments to list \
             what is currently held"
        ));
    };

    print_locks(std::slice::from_ref(lock));
    if !skip_confirmation && !confirm_interactively(lock) {
        println!("aborted: lock not cleared");
        return Ok(());
    }

    clear_lock(&layout, &lock.path).map_err(|err| err.to_string())?;
    println!("cleared: {}", lock.path.display());
    Ok(())
}

fn print_locks(locks: &[HeldLock]) {
    if locks.is_empty() {
        println!("no locks currently held");
        return;
    }
    for lock in locks {
        println!("{}", lock.path.display());
        println!("  kind: {}", lock.kind);
        match lock.recorded_pid {
            Some(pid) => println!("  recorded pid: {pid}"),
            None => println!("  recorded pid: (unparseable)"),
        }
        println!("  liveness: {}", describe_liveness(lock.liveness));
    }
    println!();
    println!(
        "liveness is advisory only: a positive result is reliable evidence the process is still \
         running, but a negative or unknown result is NOT proof it is safe to clear -- PID reuse \
         and container namespace isolation can both make a genuinely running process appear absent. \
         Clear a lock only if you have independently confirmed the process that created it is gone."
    );
}

fn describe_liveness(liveness: PidLiveness) -> &'static str {
    match liveness {
        PidLiveness::AppearsRunning => "appears running -- do not clear",
        PidLiveness::DoesNotAppearRunning => {
            "does not appear to be running (not proof it is safe to clear)"
        }
        PidLiveness::Unknown => "unknown (not proof it is safe to clear)",
    }
}

fn confirm_interactively(lock: &HeldLock) -> bool {
    print!(
        "Clearing this lock while its process is still running can corrupt this repository. \
         Type 'yes' to confirm clearing {}: ",
        lock.path.display()
    );
    if std::io::stdout().flush().is_err() {
        return false;
    }
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    input.trim() == "yes"
}
