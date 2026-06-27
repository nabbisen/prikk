//! CLI argument parsing helpers.

use std::path::PathBuf;

use prikk_store::{DEFAULT_CHECKOUT_REF, DEFAULT_HISTORY_LIMIT};

/// Parsed log command arguments.
pub(crate) struct LogArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// Ref to inspect.
    pub(crate) ref_name: String,
    /// Maximum entries to display.
    pub(crate) limit: usize,
}

/// Parsed checkout command arguments.
pub(crate) struct CheckoutArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// Ref to inspect.
    pub(crate) ref_name: String,
    /// Checkout mode.
    pub(crate) mode: CheckoutMode,
}

/// Checkout command mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckoutMode {
    /// Read-only general checkout planning.
    PlanOnly,
    /// Read-only snapshot manifest validation.
    SnapshotPlan,
    /// Opt-in snapshot materialization.
    SnapshotMaterialize,
}

/// Parsed worktree-status command arguments.
pub(crate) struct WorktreeStatusArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// Ref to use as baseline.
    pub(crate) ref_name: String,
}

/// Parsed doctor command arguments.
pub(crate) struct DoctorArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// Whether to repair incomplete trailing WAL bytes.
    pub(crate) repair_wal_tail: bool,
    /// Whether to reconstruct a missing heads/main pointer from valid logs.
    pub(crate) repair_main_ref: bool,
}

/// Parse `prikk log` arguments.
pub(crate) fn parse_log_args(args: Vec<String>) -> std::result::Result<LogArgs, String> {
    let mut path = None;
    let mut ref_name = "heads/main".to_string();
    let mut limit = DEFAULT_HISTORY_LIMIT;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("log --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("log --ref must not be empty".to_string());
                }
                ref_name = value;
            }
            "--limit" => {
                let Some(value) = iter.next() else {
                    return Err("log --limit requires a value".to_string());
                };
                limit = value
                    .parse::<usize>()
                    .map_err(|_| "log --limit must be a non-negative integer".to_string())?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown log argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("log accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    Ok(LogArgs {
        root: optional_path_or_current(path)?,
        ref_name,
        limit,
    })
}

/// Parse `prikk checkout` arguments.
pub(crate) fn parse_checkout_args(
    args: Vec<String>,
) -> std::result::Result<CheckoutArgs, String> {
    let mut mode = None;
    let mut path = None;
    let mut ref_name = DEFAULT_CHECKOUT_REF.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--plan-only" => set_checkout_mode(&mut mode, CheckoutMode::PlanOnly)?,
            "--snapshot-plan" => set_checkout_mode(&mut mode, CheckoutMode::SnapshotPlan)?,
            "--snapshot-materialize" => {
                set_checkout_mode(&mut mode, CheckoutMode::SnapshotMaterialize)?
            }
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("checkout --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("checkout --ref must not be empty".to_string());
                }
                ref_name = value;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown checkout argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("checkout accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    let Some(mode) = mode else {
        return Err(
            concat!(
                "PR-018 supports `prikk checkout --plan-only`, `--snapshot-plan`, or ",
                "`--snapshot-materialize`",
            )
                .to_string(),
        );
    };
    Ok(CheckoutArgs {
        root: optional_path_or_current(path)?,
        ref_name,
        mode,
    })
}

/// Parse `prikk worktree-status` arguments.
pub(crate) fn parse_worktree_status_args(
    args: Vec<String>,
) -> std::result::Result<WorktreeStatusArgs, String> {
    let mut path = None;
    let mut ref_name = DEFAULT_CHECKOUT_REF.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("worktree-status --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("worktree-status --ref must not be empty".to_string());
                }
                ref_name = value;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown worktree-status argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("worktree-status accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    Ok(WorktreeStatusArgs { root: optional_path_or_current(path)?, ref_name })
}

/// Parse `prikk doctor` arguments.
pub(crate) fn parse_doctor_args(args: Vec<String>) -> std::result::Result<DoctorArgs, String> {
    let mut repair_wal_tail = false;
    let mut repair_main_ref = false;
    let mut path = None;
    for arg in args {
        match arg.as_str() {
            "--repair-wal-tail" => repair_wal_tail = true,
            "--repair-main-ref" => repair_main_ref = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown doctor argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("doctor accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    Ok(DoctorArgs {
        root: optional_path_or_current(path)?,
        repair_wal_tail,
        repair_main_ref,
    })
}

/// Parse the narrow empty commit scaffold arguments.
pub(crate) fn parse_empty_commit_message(args: Vec<String>) -> std::result::Result<String, String> {
    let mut allow_empty = false;
    let mut message = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--allow-empty" => allow_empty = true,
            "-m" | "--message" => {
                let Some(value) = iter.next() else {
                    return Err("commit message option requires a value".to_string());
                };
                message = Some(value);
            }
            other => return Err(format!("unknown commit argument: {other}")),
        }
    }
    if !allow_empty {
        return Err("PR-018 supports only `prikk commit --allow-empty -m <message>`".to_string());
    }
    let Some(message) = message else {
        return Err("empty commit requires -m <message>".to_string());
    };
    if message.trim().is_empty() {
        return Err("commit message must not be empty".to_string());
    }
    Ok(message)
}

/// Return an optional path or the current working directory.
pub(crate) fn optional_path_or_current(
    path: Option<String>,
) -> std::result::Result<PathBuf, String> {
    match path {
        Some(path) => Ok(PathBuf::from(path)),
        None => current_dir(),
    }
}

/// Return the current working directory.
pub(crate) fn current_dir() -> std::result::Result<PathBuf, String> {
    std::env::current_dir().map_err(|err| err.to_string())
}

fn set_checkout_mode(
    mode: &mut Option<CheckoutMode>,
    next: CheckoutMode,
) -> std::result::Result<(), String> {
    if mode.is_some() {
        return Err("checkout accepts only one mode flag".to_string());
    }
    *mode = Some(next);
    Ok(())
}
