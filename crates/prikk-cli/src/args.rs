//! CLI argument parsing helpers.

use std::path::PathBuf;

use prikk_store::{DEFAULT_CHECKOUT_REF, DEFAULT_HISTORY_LIMIT};

mod checkout;
mod merge_evidence;
mod merge_execute;

pub(crate) use checkout::{CheckoutMode, parse_checkout_args};
pub(crate) use merge_evidence::{
    MergeEvidenceTargetArg, parse_merge_evidence_args, parse_merge_plan_args,
};
pub(crate) use merge_execute::parse_merge_execute_args;

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
    /// Whether `--repair-main-ref` was supplied. Always refused -- no repair is implemented.
    pub(crate) repair_main_ref: bool,
}

/// `prikk verify`'s output format (RFC 118 stage 5). `Prose` is the default and must remain
/// byte-identical to the pre-stage-5 output; `Json` is additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerifyOutputFormat {
    /// The original, human-readable line-per-fact report (unchanged by RFC 118 stage 5).
    Prose,
    /// `verify-report-v1` (RFC 118 stage 5): schema version, verdict, and one entry per
    /// `VerificationStage::ALL`.
    Json,
}

/// Parsed verify command arguments.
pub(crate) struct VerifyArgs {
    /// Repository root.
    pub(crate) root: PathBuf,
    /// DC-95 Stage 2 Level 1: stop at the first stage that fails or cannot evaluate, rather than
    /// accumulating findings across all twelve. Preserves the pre-Stage-2 bounded-walk behavior for
    /// a large, badly-damaged repository where a full accumulating scan would be costly.
    pub(crate) stop_on_first_error: bool,
    /// Output format. Default `Prose` (RFC 118 stage 5).
    pub(crate) format: VerifyOutputFormat,
}

/// Parse `prikk verify` arguments.
pub(crate) fn parse_verify_args(args: Vec<String>) -> std::result::Result<VerifyArgs, String> {
    let mut stop_on_first_error = false;
    let mut format = VerifyOutputFormat::Prose;
    let mut path = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--stop-on-first-error" => stop_on_first_error = true,
            "--format" => {
                let Some(value) = iter.next() else {
                    return Err("verify --format requires a value".to_string());
                };
                format = match value.as_str() {
                    "json" => VerifyOutputFormat::Json,
                    other => return Err(format!("verify --format does not support {other:?}")),
                };
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown verify argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("verify accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    Ok(VerifyArgs {
        root: optional_path_or_current(path)?,
        stop_on_first_error,
        format,
    })
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
