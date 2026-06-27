//! CLI argument parsing helpers.

use std::path::PathBuf;

use prikk_store::{DEFAULT_CHECKOUT_REF, DEFAULT_HISTORY_LIMIT};

/// Parsed commit command arguments.
pub(crate) struct CommitArgs {
    /// Commit mode.
    pub(crate) mode: CommitMode,
    /// Commit message.
    pub(crate) message: String,
    /// Baseline ref for worktree commits.
    pub(crate) ref_name: String,
    /// Whether to prefer conservative full-file text edit generation.
    pub(crate) text_edits: bool,
}

/// Commit command mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommitMode {
    /// Append the existing placeholder empty patch.
    AllowEmpty,
    /// Generate a minimal patch from snapshot-baseline worktree changes.
    FromWorktree,
}

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
    /// Read-only supported patch replay planning.
    PatchPlan,
    /// Opt-in materialization from supported patch replay.
    PatchMaterialize,
    /// Read-only deletion plan for explicit patch-removed files.
    PatchDeletePlan,
    /// Opt-in patch materialization plus explicit patch-removed file deletion.
    PatchMaterializeDelete,
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
            },
            "--patch-plan" => set_checkout_mode(&mut mode, CheckoutMode::PatchPlan)?,
            "--patch-materialize" => {
                set_checkout_mode(&mut mode, CheckoutMode::PatchMaterialize)?
            },
            "--patch-delete-plan" => {
                set_checkout_mode(&mut mode, CheckoutMode::PatchDeletePlan)?
            },
            "--patch-materialize-delete" => {
                set_checkout_mode(&mut mode, CheckoutMode::PatchMaterializeDelete)?
            },
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("checkout --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("checkout --ref must not be empty".to_string());
                }
                ref_name = value;
            },
            other if other.starts_with('-') => {
                return Err(format!("unknown checkout argument: {other}"));
            },
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
                "PR-025 supports `prikk checkout --plan-only`, `--snapshot-plan`, ",
                "`--snapshot-materialize`, `--patch-plan`, `--patch-materialize`, ",
                "`--patch-delete-plan`, or `--patch-materialize-delete`",
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
            },
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

/// Parse commit scaffold arguments.
pub(crate) fn parse_commit_args(args: Vec<String>) -> std::result::Result<CommitArgs, String> {
    let mut mode = None;
    let mut message = None;
    let mut ref_name = DEFAULT_CHECKOUT_REF.to_string();
    let mut text_edits = false;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--allow-empty" => set_commit_mode(&mut mode, CommitMode::AllowEmpty)?,
            "--from-worktree" => set_commit_mode(&mut mode, CommitMode::FromWorktree)?,
            "--text-edits" => text_edits = true,
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("commit --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("commit --ref must not be empty".to_string());
                }
                ref_name = value;
            }
            "-m" | "--message" => {
                let Some(value) = iter.next() else {
                    return Err("commit message option requires a value".to_string());
                };
                message = Some(value);
            }
            other => return Err(format!("unknown commit argument: {other}")),
        }
    }
    let Some(mode) = mode else {
        return Err(
            "PR-025 supports `prikk commit --allow-empty -m <message>` or `--from-worktree [--text-edits] -m <message>`"
                .to_string(),
        );
    };
    let Some(message) = message else {
        return Err("commit requires -m <message>".to_string());
    };
    if message.trim().is_empty() {
        return Err("commit message must not be empty".to_string());
    }
    if text_edits && mode != CommitMode::FromWorktree {
        return Err("commit --text-edits requires --from-worktree".to_string());
    }
    Ok(CommitArgs { mode, message, ref_name, text_edits })
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

fn set_commit_mode(
    mode: &mut Option<CommitMode>,
    next: CommitMode,
) -> std::result::Result<(), String> {
    if mode.is_some() {
        return Err("commit accepts only one mode flag".to_string());
    }
    *mode = Some(next);
    Ok(())
}
