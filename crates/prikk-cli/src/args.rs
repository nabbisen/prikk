//! CLI argument parsing helpers.

use std::path::PathBuf;

use prikk_store::{DEFAULT_CHECKOUT_REF, DEFAULT_HISTORY_LIMIT};

/// Parsed commit command arguments.
pub(crate) struct CommitArgs {
    /// Commit message.
    pub(crate) message: String,
    /// Baseline ref for worktree commits.
    pub(crate) ref_name: String,
    /// Compatibility flag retained for text edit generation.
    pub(crate) text_edits: bool,
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

/// Parsed inverse-plan command arguments.
pub(crate) struct InversePlanArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// Ref to inspect.
    pub(crate) ref_name: String,
}

/// Parsed rollback-preview command arguments.
pub(crate) struct RollbackPreviewArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// Ref to inspect.
    pub(crate) ref_name: String,
}

/// Parsed rollback-draft command arguments.
pub(crate) struct RollbackDraftArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// Ref to inspect.
    pub(crate) ref_name: String,
    /// Rollback draft message.
    pub(crate) message: String,
}

/// Parsed rollback-draft-verify command arguments.
pub(crate) struct RollbackDraftVerifyArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// Ref to inspect.
    pub(crate) ref_name: String,
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
pub(crate) fn parse_checkout_args(args: Vec<String>) -> std::result::Result<CheckoutArgs, String> {
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
            "--patch-plan" => set_checkout_mode(&mut mode, CheckoutMode::PatchPlan)?,
            "--patch-materialize" => set_checkout_mode(&mut mode, CheckoutMode::PatchMaterialize)?,
            "--patch-delete-plan" => set_checkout_mode(&mut mode, CheckoutMode::PatchDeletePlan)?,
            "--patch-materialize-delete" => {
                set_checkout_mode(&mut mode, CheckoutMode::PatchMaterializeDelete)?
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
        return Err(concat!(
            "checkout requires one mode flag: `--plan-only`, `--snapshot-plan`, ",
            "`--snapshot-materialize`, `--patch-plan`, `--patch-materialize`, ",
            "`--patch-delete-plan`, or `--patch-materialize-delete`",
        )
        .to_string());
    };
    Ok(CheckoutArgs {
        root: optional_path_or_current(path)?,
        ref_name,
        mode,
    })
}

/// Parse `prikk inverse-plan` arguments.
pub(crate) fn parse_inverse_plan_args(
    args: Vec<String>,
) -> std::result::Result<InversePlanArgs, String> {
    let mut path = None;
    let mut ref_name = DEFAULT_CHECKOUT_REF.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("inverse-plan --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("inverse-plan --ref must not be empty".to_string());
                }
                ref_name = value;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown inverse-plan argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("inverse-plan accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    Ok(InversePlanArgs {
        root: optional_path_or_current(path)?,
        ref_name,
    })
}

/// Parse `prikk rollback-preview` arguments.
pub(crate) fn parse_rollback_preview_args(
    args: Vec<String>,
) -> std::result::Result<RollbackPreviewArgs, String> {
    let mut path = None;
    let mut ref_name = DEFAULT_CHECKOUT_REF.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("rollback-preview --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("rollback-preview --ref must not be empty".to_string());
                }
                ref_name = value;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown rollback-preview argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("rollback-preview accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    Ok(RollbackPreviewArgs {
        root: optional_path_or_current(path)?,
        ref_name,
    })
}

/// Parse `prikk rollback-draft` arguments.
pub(crate) fn parse_rollback_draft_args(
    args: Vec<String>,
) -> std::result::Result<RollbackDraftArgs, String> {
    let mut path = None;
    let mut ref_name = DEFAULT_CHECKOUT_REF.to_string();
    let mut message = None;
    let mut append_inverse = false;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--append-inverse" => append_inverse = true,
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("rollback-draft --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("rollback-draft --ref must not be empty".to_string());
                }
                ref_name = value;
            }
            "-m" | "--message" => {
                let Some(value) = iter.next() else {
                    return Err("rollback-draft message option requires a value".to_string());
                };
                message = Some(value);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown rollback-draft argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("rollback-draft accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    if !append_inverse {
        return Err("rollback-draft requires --append-inverse".to_string());
    }
    let Some(message) = message else {
        return Err("rollback-draft requires -m <message>".to_string());
    };
    if message.trim().is_empty() {
        return Err("rollback-draft message must not be empty".to_string());
    }
    Ok(RollbackDraftArgs {
        root: optional_path_or_current(path)?,
        ref_name,
        message,
    })
}

/// Parse `prikk rollback-draft-verify` arguments.
pub(crate) fn parse_rollback_draft_verify_args(
    args: Vec<String>,
) -> std::result::Result<RollbackDraftVerifyArgs, String> {
    let mut path = None;
    let mut ref_name = DEFAULT_CHECKOUT_REF.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("rollback-draft-verify --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("rollback-draft-verify --ref must not be empty".to_string());
                }
                ref_name = value;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown rollback-draft-verify argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("rollback-draft-verify accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    Ok(RollbackDraftVerifyArgs {
        root: optional_path_or_current(path)?,
        ref_name,
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
    Ok(WorktreeStatusArgs {
        root: optional_path_or_current(path)?,
        ref_name,
    })
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

/// Parse commit arguments. Commit always authors a node-addressed patch from the worktree against the
/// baseline ref; `--from-worktree` is accepted for backward compatibility but is the only behavior.
pub(crate) fn parse_commit_args(args: Vec<String>) -> std::result::Result<CommitArgs, String> {
    let mut message = None;
    let mut ref_name = DEFAULT_CHECKOUT_REF.to_string();
    let mut text_edits = false;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            // Accepted for compatibility; worktree authoring is the only commit behavior.
            "--from-worktree" => {}
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
    let Some(message) = message else {
        return Err(
            "commit requires -m <message> (usage: prikk commit [--from-worktree] [--text-edits] \
             [--ref <name>] -m <message>)"
                .to_string(),
        );
    };
    if message.trim().is_empty() {
        return Err("commit message must not be empty".to_string());
    }
    Ok(CommitArgs {
        message,
        ref_name,
        text_edits,
    })
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
