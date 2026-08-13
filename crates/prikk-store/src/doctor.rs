//! Repository doctor diagnostics and narrowly-scoped repair helpers.
//!
//! Doctor repairs are deliberately conservative. Mutating repair is opt-in and limited to
//! incomplete active-WAL tail truncation. The former format-1 missing-pointer switch remains a
//! compatibility input but is explicitly refused.

use prikk_error::{PrikkError, Result};

use crate::layout::{RepositoryFormat, RepositoryLayout};
use crate::lock::ActiveLock;
use crate::refs::RefRecoveryRepair;
use crate::verify::{
    ActiveWalMetadataStatus, RepositoryVerification, StageStatus, verify_repository,
};
use crate::wal::{Wal, WalRepair};

/// Severity assigned to a doctor diagnostic issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorSeverity {
    /// Informational diagnostic that does not require user action.
    Info,
    /// Warning diagnostic that may require attention but does not prove corruption.
    Warning,
    /// Error diagnostic that blocks repository health.
    Error,
}

impl DoctorSeverity {
    /// Return a stable lower-case label for CLI output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// One doctor diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorIssue {
    /// Stable diagnostic code.
    pub code: &'static str,
    /// Diagnostic severity.
    pub severity: DoctorSeverity,
    /// Human-readable explanation.
    pub message: String,
    /// Suggested next action.
    pub recommendation: String,
}

impl DoctorIssue {
    /// Construct an informational diagnostic.
    #[must_use]
    pub fn info(
        code: &'static str,
        message: impl Into<String>,
        recommendation: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: DoctorSeverity::Info,
            message: message.into(),
            recommendation: recommendation.into(),
        }
    }

    /// Construct a warning diagnostic.
    #[must_use]
    pub fn warning(
        code: &'static str,
        message: impl Into<String>,
        recommendation: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: DoctorSeverity::Warning,
            message: message.into(),
            recommendation: recommendation.into(),
        }
    }

    /// Construct an error diagnostic.
    #[must_use]
    pub fn error(
        code: &'static str,
        message: impl Into<String>,
        recommendation: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: DoctorSeverity::Error,
            message: message.into(),
            recommendation: recommendation.into(),
        }
    }
}

/// Doctor report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    /// Repository verification summary, when verification completed.
    pub verification: Option<RepositoryVerification>,
    /// Diagnostics produced by doctor.
    pub issues: Vec<DoctorIssue>,
}

impl DoctorReport {
    /// Return true if no error-severity issue was found.
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == DoctorSeverity::Error)
    }

    /// Count issues with a given severity.
    #[must_use]
    pub fn count_by_severity(&self, severity: DoctorSeverity) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == severity)
            .count()
    }
}

/// Opt-in repair switches for doctor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoctorRepairOptions {
    /// Truncate incomplete trailing bytes from the active WAL after verification confirms that the
    /// prefix is valid.
    pub truncate_wal_tail: bool,
    /// Request the refused format-1 `heads/main` reconstruction compatibility path.
    pub reconstruct_main_ref: bool,
}

impl DoctorRepairOptions {
    /// Return options that perform no repair.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            truncate_wal_tail: false,
            reconstruct_main_ref: false,
        }
    }

    /// Return options that enable only safe active-WAL tail truncation.
    #[must_use]
    pub const fn truncate_wal_tail() -> Self {
        Self {
            truncate_wal_tail: true,
            reconstruct_main_ref: false,
        }
    }

    /// Return options that request the refused format-1 missing-pointer compatibility path.
    #[must_use]
    pub const fn reconstruct_main_ref() -> Self {
        Self {
            truncate_wal_tail: false,
            reconstruct_main_ref: true,
        }
    }
}

/// Report returned by an opt-in doctor repair run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorRepairReport {
    /// Doctor report before any repair action.
    pub before: DoctorReport,
    /// WAL repair summary.
    pub wal_repair: WalRepair,
    /// Ref pointer reconstruction summary, if requested.
    pub ref_repair: Option<RefRecoveryRepair>,
    /// Doctor report after repair action.
    pub after: DoctorReport,
}

/// Run doctor diagnostics for a repository layout.
#[must_use]
pub fn doctor_repository(layout: &RepositoryLayout) -> DoctorReport {
    let mut issues = Vec::new();
    match verify_repository(layout) {
        Ok(verification) => {
            issues.push(DoctorIssue::info(
                "PRIKK-DOCTOR-VERIFY-OK",
                "repository structural verification scan completed",
                "review the remaining diagnostics before deciding whether action is required",
            ));
            // DC-95 Stage 2 Level 1: a stage that failed or could not evaluate is blocking by
            // construction (severity derives from the stage outcome itself, not a per-field decision
            // here) -- this is what preserves `repair_repository`'s refusal gate now that
            // `verify_repository` no longer aborts on the first hard error.
            for outcome in &verification.stage_outcomes {
                let message = match &outcome.status {
                    StageStatus::Evaluated => continue,
                    StageStatus::Failed { message } => {
                        format!("verification stage {} failed: {message}", outcome.stage)
                    }
                    StageStatus::NotEvaluated { blocked_by } => {
                        format!(
                            "verification stage {} could not run because stage {blocked_by} did not evaluate",
                            outcome.stage
                        )
                    }
                    StageStatus::Halted { after } => {
                        format!(
                            "verification stage {} was not attempted because stage {after} failed and halted the walk (--stop-on-first-error)",
                            outcome.stage
                        )
                    }
                };
                issues.push(DoctorIssue::error(
                    "PRIKK-DOCTOR-VERIFY-STAGE-INCOMPLETE",
                    message,
                    "preserve the repository and inspect the failing stage before attempting repair",
                ));
            }
            if layout.format() == RepositoryFormat::LegacyV1 {
                issues.push(DoctorIssue::warning(
                    "PRIKK-DOCTOR-LEGACY-FORMAT",
                    "repository is open in format-1 legacy read-only mode; scaffold roots are not verifiable state commitments",
                    "preserve the repository bytes or initialize a new format-2 repository and deliberately re-author the worktree",
                ));
            }
            if verification
                .trailing_partial_wal_bytes
                .is_some_and(|n| n != 0)
            {
                issues.push(DoctorIssue::warning(
                    "PRIKK-DOCTOR-WAL-TRAILING-PARTIAL",
                    format!(
                        concat!(
                            "active WAL has {} trailing byte(s) that look like an incomplete ",
                            "final record"
                        ),
                        verification.trailing_partial_wal_bytes.unwrap_or_default()
                    ),
                    "run `prikk doctor --repair-wal-tail` to truncate only the incomplete \
                     final WAL bytes",
                ));
            }
            for issue in &verification.publication_trust_issues {
                issues.push(DoctorIssue::error(
                    issue.code,
                    issue.message.clone(),
                    "configure trusted MAINTAINER keys and re-run verification; doctor will not \
                     auto-trust keys or repair signatures",
                ));
            }
            for issue in &verification.signature_envelope_issues {
                issues.push(DoctorIssue::warning(
                    issue.code,
                    format!("{}: {}", issue.source, issue.message),
                    "preserve the format-1 bytes for inspection; do not normalize or reuse the envelope for mutation",
                ));
            }
            for path in &verification.object_temp_paths {
                let name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("<non-UTF-8 object temp>");
                issues.push(DoctorIssue::warning(
                    "PRIKK-DOCTOR-OBJECT-TEMP-DEBRIS",
                    format!("non-authoritative object publication temp remains: {name}"),
                    "preserve it for inspection; doctor does not infer ownership or remove object temps",
                ));
            }
            for issue in &verification.ref_publication_issues {
                let recommendation = match issue.code {
                    "PRIKK-VERIFY-REF-POINTER-LEADS-LOG"
                    | "PRIKK-VERIFY-REF-LEGACY-LOG-LEADS"
                    | "PRIKK-VERIFY-REF-ACTIVE-CLEANUP-PENDING" => {
                        "run signer-backed `prikk seal --allow-no-audit` for the affected ref; doctor does not sign or append"
                    }
                    "PRIKK-VERIFY-REF-POINTER-MISSING" => {
                        "preserve the repository; use signer-backed seal retry only with matching retained active state, otherwise restore from backup"
                    }
                    "PRIKK-VERIFY-REF-LEGACY-TIMESTAMP" => {
                        "treat the value as non-authoritative legacy data; do not normalize signed bytes in place"
                    }
                    "PRIKK-VERIFY-REF-DIVERGENCE" => {
                        "preserve the repository for manual recovery; signer-backed retry is not authorized without exact retained evidence"
                    }
                    _ => {
                        "preserve the candidate for inspection; doctor does not infer ownership or remove it"
                    }
                };
                let doctor_issue = if issue.blocking {
                    DoctorIssue::error(issue.code, issue.message.clone(), recommendation)
                } else {
                    DoctorIssue::warning(issue.code, issue.message.clone(), recommendation)
                };
                issues.push(doctor_issue);
            }
            add_active_wal_metadata_issues(&verification, &mut issues);
            DoctorReport {
                verification: Some(verification),
                issues,
            }
        }
        Err(error) => {
            issues.push(issue_for_verification_error(error));
            DoctorReport {
                verification: None,
                issues,
            }
        }
    }
}

/// Run an explicitly requested, narrow repair action.
///
/// The repair is refused if verification fails for any reason other than a trailing partial WAL
/// record reported by normal replay. This preserves data until a future, more specific repair
/// command is implemented.
pub fn repair_repository(
    layout: &RepositoryLayout,
    options: DoctorRepairOptions,
) -> Result<DoctorRepairReport> {
    layout.require_current_format()?;
    if options.reconstruct_main_ref {
        return Err(PrikkError::Integrity(
            "format-1 missing-pointer doctor repair is unsupported in 0.18.0; preserve the repository for signer-backed retry or later recovery tooling"
                .to_string(),
        ));
    }
    let _active_lock = ActiveLock::acquire(layout)?;
    crate::refs::ensure_no_incomplete_publication(layout)?;
    let before = doctor_repository(layout);
    if !before.is_healthy() {
        return Err(PrikkError::Integrity(
            "doctor repair refused because repository verification has errors".to_string(),
        ));
    }
    let wal_repair = if options.truncate_wal_tail {
        let wal = Wal::for_layout(layout);
        wal.truncate_trailing_partial()?
    } else {
        WalRepair {
            preserved_records: 0,
            truncated_bytes: 0,
            preserved_patch_ids: Vec::new(),
        }
    };
    let ref_repair = None;
    let after = doctor_repository(layout);
    Ok(DoctorRepairReport {
        before,
        wal_repair,
        ref_repair,
        after,
    })
}

fn add_active_wal_metadata_issues(
    verification: &RepositoryVerification,
    issues: &mut Vec<DoctorIssue>,
) {
    // `None` (the active-WAL-metadata stage did not evaluate) is already surfaced, more precisely, by
    // the stage-outcome loop above this function's own call site.
    let Some(status) = &verification.active_wal_metadata_status else {
        return;
    };
    match status {
        ActiveWalMetadataStatus::MissingForNonEmptyWal => issues.push(DoctorIssue::error(
            "PRIKK-DOCTOR-ACTIVE-REF-METADATA-MISSING",
            "active WAL has records but active ref metadata is missing",
            "preserve the repository and inspect the active WAL before sealing or appending",
        )),
        ActiveWalMetadataStatus::InvalidForNonEmptyWal { reason } => {
            issues.push(DoctorIssue::error(
                "PRIKK-DOCTOR-ACTIVE-REF-METADATA-MALFORMED",
                format!("active WAL has records but active ref metadata is malformed: {reason}"),
                "preserve the repository and inspect the active WAL before sealing or appending",
            ));
        }
        ActiveWalMetadataStatus::ValidForEmptyWal { ref_name } => issues.push(
            DoctorIssue::warning(
                "PRIKK-DOCTOR-ACTIVE-REF-METADATA-DEBRIS",
                format!("active WAL is empty but stale ref metadata remains for {ref_name}"),
                "no repair is required; the next guarded active-WAL append will replace stale metadata",
            ),
        ),
        ActiveWalMetadataStatus::InvalidForEmptyWal { reason } => issues.push(
            DoctorIssue::warning(
                "PRIKK-DOCTOR-ACTIVE-REF-METADATA-MALFORMED-DEBRIS",
                format!("active WAL is empty but malformed ref metadata remains: {reason}"),
                "no repair is required; the next guarded active-WAL append will replace stale metadata",
            ),
        ),
        ActiveWalMetadataStatus::MissingForEmptyWal
        | ActiveWalMetadataStatus::ValidForNonEmptyWal { .. } => {}
    }
}

fn issue_for_verification_error(error: PrikkError) -> DoctorIssue {
    DoctorIssue::error(
        "PRIKK-DOCTOR-VERIFY-ERROR",
        format!("repository verification failed: {error}"),
        "do not run seal or publish operations; preserve the repository and inspect the \
         failing path before attempting repair",
    )
}

#[cfg(test)]
mod tests;
