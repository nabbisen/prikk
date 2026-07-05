use prikk_object::{BlobKind, NodeId, ObjectId, text_span_hash};

use crate::node_lifecycle::NodeLifecycleState;
use crate::text_span;

use super::preimage::invalid_preimage_class;
use super::types::{
    Action, BaselineTextResolver, ConflictWitnessKind, NoBaselineTextResolver, OperationFacts,
    PairClass, RequiredOrder, UnknownReason,
};
use super::witness::{conflict, conflict_with_span, ordered, unknown_from_facts};

pub(super) fn classify_create_then_mutate<R: BaselineTextResolver>(
    baseline: &NodeLifecycleState,
    evidence: &R,
    left: &OperationFacts,
    right: &OperationFacts,
    node_id: NodeId,
) -> Option<PairClass> {
    if let Some(class) = invalid_create_in_same_node_pair(baseline, left, right) {
        return Some(class);
    }
    if let Some(class) = invalid_create_in_same_node_pair(baseline, right, left) {
        return Some(class);
    }
    match (&left.action, &right.action) {
        (
            Action::CreateFile { mode, .. },
            Action::ChangePerm {
                old_mode,
                new_mode: _,
                ..
            },
        ) if *mode == *old_mode => Some(ordered(
            RequiredOrder::LeftBeforeRight,
            ConflictWitnessKind::LiveStateMismatch,
            left,
            right,
            Some(node_id),
            None,
        )),
        (
            Action::ChangePerm {
                old_mode,
                new_mode: _,
                ..
            },
            Action::CreateFile { mode, .. },
        ) if *mode == *old_mode => Some(ordered(
            RequiredOrder::RightBeforeLeft,
            ConflictWitnessKind::LiveStateMismatch,
            left,
            right,
            Some(node_id),
            None,
        )),
        (Action::CreateFile { blob_id, .. }, Action::ReplaceBinary { old_blob_id, .. }) => {
            Some(classify_create_then_replace_binary(
                evidence,
                left,
                right,
                node_id,
                RequiredOrder::LeftBeforeRight,
                *blob_id,
                *old_blob_id,
            ))
        }
        (Action::ReplaceBinary { old_blob_id, .. }, Action::CreateFile { blob_id, .. }) => {
            Some(classify_create_then_replace_binary(
                evidence,
                left,
                right,
                node_id,
                RequiredOrder::RightBeforeLeft,
                *blob_id,
                *old_blob_id,
            ))
        }
        (Action::CreateFile { blob_id, .. }, edit @ Action::EditText { .. }) => {
            Some(classify_create_then_edit_text(
                evidence,
                left,
                right,
                node_id,
                RequiredOrder::LeftBeforeRight,
                *blob_id,
                edit,
            ))
        }
        (edit @ Action::EditText { .. }, Action::CreateFile { blob_id, .. }) => {
            Some(classify_create_then_edit_text(
                evidence,
                left,
                right,
                node_id,
                RequiredOrder::RightBeforeLeft,
                *blob_id,
                edit,
            ))
        }
        (Action::CreateFile { .. }, _) | (_, Action::CreateFile { .. }) => Some(conflict(
            ConflictWitnessKind::NodeIdReuse,
            left,
            right,
            Some(node_id),
            None,
        )),
        _ => None,
    }
}

fn invalid_create_in_same_node_pair(
    baseline: &NodeLifecycleState,
    create_candidate: &OperationFacts,
    peer: &OperationFacts,
) -> Option<PairClass> {
    if !matches!(create_candidate.action, Action::CreateFile { .. }) {
        return None;
    }
    invalid_preimage_class(baseline, &NoBaselineTextResolver, create_candidate, peer)
}

fn classify_create_then_replace_binary<R: BaselineTextResolver>(
    evidence: &R,
    left: &OperationFacts,
    right: &OperationFacts,
    node_id: NodeId,
    required_order: RequiredOrder,
    create_blob_id: ObjectId,
    replace_old_blob_id: ObjectId,
) -> PairClass {
    if create_blob_id != replace_old_blob_id {
        return conflict(
            ConflictWitnessKind::BlobMismatch,
            left,
            right,
            Some(node_id),
            None,
        );
    }
    match evidence.blob_kind(create_blob_id) {
        Some(BlobKind::Binary) => ordered(
            required_order,
            ConflictWitnessKind::LiveStateMismatch,
            left,
            right,
            Some(node_id),
            None,
        ),
        Some(BlobKind::Text | BlobKind::Snapshot) => conflict(
            ConflictWitnessKind::KindMismatch,
            left,
            right,
            Some(node_id),
            None,
        ),
        None => unknown_from_facts(
            UnknownReason::FuturePreconditionDeferred,
            left,
            right,
            Some(node_id),
            None,
        ),
    }
}

fn classify_create_then_edit_text<R: BaselineTextResolver>(
    evidence: &R,
    left: &OperationFacts,
    right: &OperationFacts,
    node_id: NodeId,
    required_order: RequiredOrder,
    create_blob_id: ObjectId,
    edit: &Action,
) -> PairClass {
    let Some((kind, content)) = evidence.blob_content(create_blob_id) else {
        return unknown_from_facts(
            UnknownReason::FuturePreconditionDeferred,
            left,
            right,
            Some(node_id),
            None,
        );
    };
    if kind != BlobKind::Text {
        return conflict(
            ConflictWitnessKind::KindMismatch,
            left,
            right,
            Some(node_id),
            None,
        );
    }
    let Action::EditText {
        span_id,
        old_span_hash,
        left_anchor_hash,
        right_anchor_hash,
        old_span_text,
        ..
    } = edit
    else {
        unreachable!("caller passes only EditText actions");
    };
    if text_span_hash(old_span_text) != *old_span_hash {
        return conflict_with_span(
            ConflictWitnessKind::TextAnchorStale,
            left,
            right,
            node_id,
            *span_id,
        );
    }
    match text_span::locate_text_span(
        &content,
        old_span_text,
        left_anchor_hash,
        right_anchor_hash,
        span_id,
        node_id,
        old_span_hash,
    ) {
        Ok(_) => ordered(
            required_order,
            ConflictWitnessKind::LiveStateMismatch,
            left,
            right,
            Some(node_id),
            None,
        ),
        Err(_) => conflict_with_span(
            ConflictWitnessKind::TextAnchorStale,
            left,
            right,
            node_id,
            *span_id,
        ),
    }
}
