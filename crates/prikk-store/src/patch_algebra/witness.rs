use std::collections::BTreeSet;

use prikk_object::NodeId;

use crate::patch_replay::decode::DecodedPatchOperation;
use crate::path::RepoPath;

use super::types::{
    ConflictWitness, ConflictWitnessKind, OperationFacts, PairClass, RequiredOrder, UnknownReason,
};

pub(super) fn ordered(
    required_order: RequiredOrder,
    kind: ConflictWitnessKind,
    left: &OperationFacts,
    right: &OperationFacts,
    node_id: Option<NodeId>,
    path: Option<RepoPath>,
) -> PairClass {
    PairClass::OrderedDependency {
        required_order,
        witness: witness(kind, left.op_seq, right.op_seq, node_id, path, None),
    }
}

pub(super) fn conflict(
    kind: ConflictWitnessKind,
    left: &OperationFacts,
    right: &OperationFacts,
    node_id: Option<NodeId>,
    path: Option<RepoPath>,
) -> PairClass {
    PairClass::Conflict {
        witness: witness(kind, left.op_seq, right.op_seq, node_id, path, None),
    }
}

pub(super) fn conflict_with_span(
    kind: ConflictWitnessKind,
    left: &OperationFacts,
    right: &OperationFacts,
    node_id: NodeId,
    span_id: [u8; 32],
) -> PairClass {
    PairClass::Conflict {
        witness: witness(
            kind,
            left.op_seq,
            right.op_seq,
            Some(node_id),
            None,
            Some(span_id),
        ),
    }
}

pub(super) fn unknown(
    reason: UnknownReason,
    left: &DecodedPatchOperation,
    right: &DecodedPatchOperation,
    node_id: Option<NodeId>,
    path: Option<RepoPath>,
) -> PairClass {
    PairClass::Unknown {
        reason,
        witness: witness(
            match reason {
                UnknownReason::MalformedOperation => ConflictWitnessKind::MalformedOperation,
                UnknownReason::RenameDeferred | UnknownReason::SymlinkDeferred => {
                    ConflictWitnessKind::UnsupportedOperation
                }
                _ => ConflictWitnessKind::UnknownRelation,
            },
            left.op_seq,
            right.op_seq,
            node_id,
            path,
            None,
        ),
    }
}

pub(super) fn unknown_from_facts(
    reason: UnknownReason,
    left: &OperationFacts,
    right: &OperationFacts,
    node_id: Option<NodeId>,
    path: Option<RepoPath>,
) -> PairClass {
    PairClass::Unknown {
        reason,
        witness: witness(
            ConflictWitnessKind::UnknownRelation,
            left.op_seq,
            right.op_seq,
            node_id,
            path,
            None,
        ),
    }
}

fn witness(
    kind: ConflictWitnessKind,
    left_op_seq: u32,
    right_op_seq: u32,
    node_id: Option<NodeId>,
    path: Option<RepoPath>,
    text_span: Option<[u8; 32]>,
) -> ConflictWitness {
    ConflictWitness {
        kind,
        left_op_seq,
        right_op_seq,
        node_id,
        path,
        expected: None,
        actual: None,
        text_span,
    }
}

pub(super) fn common_node(left: &OperationFacts, right: &OperationFacts) -> Option<NodeId> {
    match (left.node_id, right.node_id) {
        (Some(left_id), Some(right_id)) if left_id == right_id => Some(left_id),
        _ => None,
    }
}

pub(super) fn first_intersection(
    left: &BTreeSet<RepoPath>,
    right: &BTreeSet<RepoPath>,
) -> Option<RepoPath> {
    left.iter().find(|path| right.contains(*path)).cloned()
}
