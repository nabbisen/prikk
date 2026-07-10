mod analysis;
mod error;
mod input;
mod mapping;
mod types;

pub(crate) use analysis::analyze_merge_evidence;
#[cfg(test)]
pub(crate) use analysis::analyze_pair_merge_evidence;
#[cfg(test)]
pub(super) use input::sort_report_items;
pub(crate) use types::{
    MergeEvidenceItem, MergeEvidenceOperationKind, MergeEvidenceOutcome, MergeEvidenceProofPhase,
    MergeEvidenceReasonCode, MergeEvidenceReport, MergeEvidenceScope, MergeEvidenceSide,
};
