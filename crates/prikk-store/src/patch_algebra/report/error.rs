use prikk_object::NodeId;

use super::types::{
    MergeEvidenceItem, MergeEvidenceOutcome, MergeEvidenceProofPhase, MergeEvidenceReasonCode,
    MergeEvidenceSide,
};
use crate::patch_algebra::evidence_types::{EvidenceError, EvidenceFact, EvidenceScope};

pub(super) fn evidence_error_report(
    error: &EvidenceError,
) -> (MergeEvidenceOutcome, Vec<MergeEvidenceItem>) {
    let scope = evidence_error_scope(error);
    let outcome = match scope {
        EvidenceScope::SealedBaselineRequired | EvidenceScope::SealedCandidateRequired => {
            MergeEvidenceOutcome::EvidenceFailure
        }
        EvidenceScope::UnsealedCandidateOptional => MergeEvidenceOutcome::InvalidCandidate,
    };
    (
        outcome,
        vec![MergeEvidenceItem {
            side: MergeEvidenceSide::Report,
            operation_index: None,
            peer_operation_index: None,
            op_seq: None,
            peer_op_seq: None,
            operation_kind: None,
            node_id: evidence_error_node_id(error),
            path: None,
            witness_kind: None,
            outcome,
            evidence_scope: Some(scope.into()),
            proof_phase: MergeEvidenceProofPhase::Classification,
            reason_code: evidence_error_reason_code(error, outcome),
        }],
    )
}

fn evidence_error_scope(error: &EvidenceError) -> EvidenceScope {
    match error {
        EvidenceError::Missing { scope, .. }
        | EvidenceError::WrongObjectType { scope, .. }
        | EvidenceError::WrongBlobKind { scope, .. }
        | EvidenceError::Malformed { scope, .. }
        | EvidenceError::Unreadable { scope, .. } => *scope,
    }
}

fn evidence_error_node_id(error: &EvidenceError) -> Option<NodeId> {
    match error {
        EvidenceError::Missing { node_id, .. } => *node_id,
        EvidenceError::WrongObjectType { .. }
        | EvidenceError::WrongBlobKind { .. }
        | EvidenceError::Malformed { .. }
        | EvidenceError::Unreadable { .. } => None,
    }
}

fn evidence_error_reason_code(
    error: &EvidenceError,
    outcome: MergeEvidenceOutcome,
) -> MergeEvidenceReasonCode {
    if outcome == MergeEvidenceOutcome::InvalidCandidate {
        return match error {
            EvidenceError::Missing { fact, .. } if *fact != EvidenceFact::Operation => {
                MergeEvidenceReasonCode::InsufficientUnsealedCandidateEvidence
            }
            _ => MergeEvidenceReasonCode::InvalidUnsealedCandidate,
        };
    }
    match error {
        EvidenceError::Missing { .. } => MergeEvidenceReasonCode::MissingRequiredEvidence,
        EvidenceError::WrongObjectType { .. } | EvidenceError::WrongBlobKind { .. } => {
            MergeEvidenceReasonCode::WrongTypeRequiredEvidence
        }
        EvidenceError::Malformed { .. } => MergeEvidenceReasonCode::MalformedRequiredEvidence,
        EvidenceError::Unreadable { .. } => MergeEvidenceReasonCode::UnreadableRequiredEvidence,
    }
}
