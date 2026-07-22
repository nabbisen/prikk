//! Joint ref pointer and ref-log verification.

use std::collections::BTreeSet;
use std::path::Path;

use prikk_error::{PrikkError, Result};

use crate::fsutil::{EntryKind, list_directory};
use crate::layout::RepositoryLayout;
use crate::object_store::FileObjectStore;
use crate::signature_diagnostics::{
    SignatureEnvelopeIssue, SignatureEnvelopeSource, classify_signature_envelope,
};

mod scan;

use scan::{LogState, PointerState, read_logs, read_pointers};

/// One recognized interrupted-publication or local-debris condition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefPublicationIssue {
    /// Stable diagnostic code.
    pub code: &'static str,
    /// Ref name when the condition belongs to one ref.
    pub ref_name: Option<String>,
    /// Human-readable diagnosis without host paths.
    pub message: String,
    /// Whether verification must return non-zero and unrelated mutation must remain blocked.
    pub blocking: bool,
}

/// Ref verification counters and publication-state issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RefVerification {
    pub pointer_count: usize,
    pub log_record_count: usize,
    pub ref_update_envelopes: Vec<prikk_object::ObjectEnvelope>,
    pub publication_issues: Vec<RefPublicationIssue>,
    pub signature_envelope_issues: Vec<SignatureEnvelopeIssue>,
}

pub(crate) fn verify_refs(layout: &RepositoryLayout) -> Result<RefVerification> {
    let objects = FileObjectStore::new(layout.clone());
    let pointers = read_pointers(layout, &objects)?;
    let (logs, log_record_count, ref_log_envelopes) = read_logs(layout, &objects, &pointers)?;
    let mut ref_update_envelopes = Vec::with_capacity(ref_log_envelopes.len());
    let mut signature_envelope_issues = Vec::new();
    for record in ref_log_envelopes {
        signature_envelope_issues.extend(classify_signature_envelope(
            &record.envelope,
            SignatureEnvelopeSource::RefLog {
                ref_name: record.ref_name,
                sequence: record.sequence,
                object_id: record.envelope.object_id(),
            },
        )?);
        ref_update_envelopes.push(record.envelope);
    }
    let mut names = BTreeSet::new();
    names.extend(pointers.keys().cloned());
    names.extend(logs.keys().cloned());
    let mut publication_issues = candidate_issues(layout)?;
    for ref_name in names {
        classify_ref_state(
            &ref_name,
            pointers.get(&ref_name),
            logs.get(&ref_name),
            &mut publication_issues,
        )?;
    }
    Ok(RefVerification {
        pointer_count: pointers.len(),
        log_record_count,
        ref_update_envelopes,
        publication_issues,
        signature_envelope_issues,
    })
}

fn classify_ref_state(
    ref_name: &str,
    pointer: Option<&PointerState>,
    log: Option<&LogState>,
    issues: &mut Vec<RefPublicationIssue>,
) -> Result<()> {
    if log.is_some_and(|state| state.has_legacy_timestamp) {
        issues.push(RefPublicationIssue {
            code: "PRIKK-VERIFY-REF-LEGACY-TIMESTAMP",
            ref_name: Some(ref_name.to_string()),
            message: "format-1 RefUpdate uses a non-canonical legacy timestamp".to_string(),
            blocking: false,
        });
    }
    match (pointer, log) {
        (Some(pointer), Some(log)) if Some(pointer.id) == log.tip => {
            if log.trailing_partial_bytes != 0 {
                return Err(PrikkError::Integrity(format!(
                    "ref {ref_name} has an incomplete log tail without a pointer lead"
                )));
            }
            Ok(())
        }
        (Some(pointer), log)
            if pointer.payload.previous_ref_state_id == log.and_then(|state| state.tip)
                && pointer.payload.update_seq == next_log_sequence(log)? =>
        {
            let partial = log.map_or(0, |state| state.trailing_partial_bytes);
            issues.push(blocking_issue(
                "PRIKK-VERIFY-REF-POINTER-LEADS-LOG",
                ref_name,
                if partial == 0 {
                    "authoritative pointer leads committed ref log by one transition".to_string()
                } else {
                    format!(
                        "authoritative pointer leads ref log by one transition with {partial} incomplete trailing byte(s)"
                    )
                },
            ));
            Ok(())
        }
        (Some(pointer), Some(log)) if log.previous_tip == Some(pointer.id) => {
            issues.push(blocking_issue(
                "PRIKK-VERIFY-REF-LEGACY-LOG-LEADS",
                ref_name,
                "format-1 ref log leads the authoritative pointer by one transition".to_string(),
            ));
            Ok(())
        }
        (None, Some(log)) if log.record_count == 1 && log.previous_tip.is_none() => {
            issues.push(blocking_issue(
                "PRIKK-VERIFY-REF-POINTER-MISSING",
                ref_name,
                "format-1 ref pointer is missing while committed log history exists".to_string(),
            ));
            Ok(())
        }
        (None, None) => Ok(()),
        _ => Err(PrikkError::Integrity(format!(
            "unexplained pointer/log divergence for ref {ref_name}"
        ))),
    }
}

fn next_log_sequence(log: Option<&LogState>) -> Result<u64> {
    u64::try_from(log.map_or(0, |state| state.record_count))
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| PrikkError::Integrity("ref-log sequence overflow".to_string()))
}

fn candidate_issues(layout: &RepositoryLayout) -> Result<Vec<RefPublicationIssue>> {
    let mut issues = Vec::new();
    let relative = Path::new("refs/tmp");
    for entry in list_directory(layout.repository_mutation_root(), relative)? {
        if entry.kind == EntryKind::Regular {
            issues.push(RefPublicationIssue {
                code: "PRIKK-VERIFY-REF-CANDIDATE-DEBRIS",
                ref_name: None,
                message: "non-authoritative ref pointer candidate remains".to_string(),
                blocking: false,
            });
        } else {
            return Err(PrikkError::Integrity(
                "unexpected non-file in ref candidate directory".to_string(),
            ));
        }
    }
    Ok(issues)
}

fn blocking_issue(code: &'static str, ref_name: &str, message: String) -> RefPublicationIssue {
    RefPublicationIssue {
        code,
        ref_name: Some(ref_name.to_string()),
        message,
        blocking: true,
    }
}
