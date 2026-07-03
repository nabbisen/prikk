//! Repository doctor diagnostics and narrowly-scoped repair helpers.
//!
//! PR-014 keeps doctor repairs deliberately conservative. Mutating repairs are opt-in and
//! limited to incomplete active-WAL tail truncation plus reconstruction of a missing `heads/main`
//! pointer from already-verified ref-log and RefState data.

use prikk_error::{PrikkError, Result};

use crate::layout::RepositoryLayout;
use crate::refs::{RefRecoveryRepair, RefStore};
use crate::verify::{RepositoryVerification, verify_repository};
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
    /// Reconstruct `heads/main` from a valid ref log when the pointer file is missing.
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

    /// Return options that enable only guarded `heads/main` pointer reconstruction.
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
                "repository verification completed without integrity errors",
                "no repair action is required",
            ));
            if verification.trailing_partial_wal_bytes != 0 {
                issues.push(DoctorIssue::warning(
                    "PRIKK-DOCTOR-WAL-TRAILING-PARTIAL",
                    format!(
                        concat!(
                            "active WAL has {} trailing byte(s) that look like an incomplete ",
                            "final record"
                        ),
                        verification.trailing_partial_wal_bytes
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
            add_missing_main_ref_issue(layout, &mut issues);
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
    let before = doctor_repository(layout);
    if !before.is_healthy() {
        return Err(PrikkError::Integrity(
            "doctor repair refused because repository verification has errors".to_string(),
        ));
    }
    let wal_repair = if options.truncate_wal_tail {
        let wal = Wal::new(layout.default_queue_wal_path());
        wal.truncate_trailing_partial()?
    } else {
        WalRepair {
            preserved_records: 0,
            truncated_bytes: 0,
        }
    };
    let ref_repair = if options.reconstruct_main_ref {
        let ref_store = RefStore::new(layout.clone());
        Some(ref_store.reconstruct_missing_ref_from_log("heads/main")?)
    } else {
        None
    };
    let after = doctor_repository(layout);
    Ok(DoctorRepairReport {
        before,
        wal_repair,
        ref_repair,
        after,
    })
}

fn add_missing_main_ref_issue(layout: &RepositoryLayout, issues: &mut Vec<DoctorIssue>) {
    let ref_store = RefStore::new(layout.clone());
    match ref_store.recoverable_missing_ref("heads/main") {
        Ok(Some(candidate)) => issues.push(DoctorIssue::warning(
            "PRIKK-DOCTOR-REF-POINTER-MISSING",
            format!(
                "heads/main pointer is missing but ref log can recover RefState {} at update {}",
                candidate.ref_state_id, candidate.update_seq
            ),
            "run `prikk doctor --repair-main-ref` to reconstruct only the missing \
             heads/main pointer from the verified ref log",
        )),
        Ok(None) => {}
        Err(error) => issues.push(DoctorIssue::error(
            "PRIKK-DOCTOR-REF-RECOVERY-ERROR",
            format!("heads/main ref recovery analysis failed: {error}"),
            "preserve the repository and inspect refs/logs before attempting ref repair",
        )),
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
