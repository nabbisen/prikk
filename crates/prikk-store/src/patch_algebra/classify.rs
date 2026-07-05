use prikk_object::{NodeId, NodeKind};

use super::create::classify_create_then_mutate;
use super::delete::classify_mutate_then_delete;
use super::facts::{deferred_reason, operation_facts};
use super::preimage::{
    baseline_file_matches, invalid_preimage_class, is_create_after_delete_valid,
    is_delete_preimage_valid,
};
use super::text_pair::classify_mode_and_text_edit;
use super::types::{
    Action, BaselineTextResolver, ConflictWitnessKind, NoBaselineTextResolver, OperationFacts,
    PairClass, RequiredOrder, UnknownReason,
};
use super::witness::{
    common_node, conflict, conflict_with_span, first_intersection, ordered, unknown,
    unknown_from_facts,
};
use crate::node_lifecycle::NodeLifecycleState;
use crate::patch_replay::decode::DecodedPatchOperation;

pub(crate) fn classify_pair(
    baseline: &NodeLifecycleState,
    left: &DecodedPatchOperation,
    right: &DecodedPatchOperation,
) -> PairClass {
    classify_pair_with_text_resolver(baseline, &NoBaselineTextResolver, left, right)
}

pub(crate) fn classify_pair_with_text_resolver<R: BaselineTextResolver>(
    baseline: &NodeLifecycleState,
    text_resolver: &R,
    left: &DecodedPatchOperation,
    right: &DecodedPatchOperation,
) -> PairClass {
    let left_facts = match operation_facts(left) {
        Ok(facts) => facts,
        Err(reason) => return unknown(reason, left, right, None, None),
    };
    let right_facts = match operation_facts(right) {
        Ok(facts) => facts,
        Err(reason) => return unknown(reason, left, right, None, None),
    };

    if let Some(reason) =
        deferred_reason(&left_facts.action).or(deferred_reason(&right_facts.action))
    {
        return unknown(
            reason,
            left,
            right,
            common_node(&left_facts, &right_facts),
            None,
        );
    }

    if let Some(class) = classify_path_relation(baseline, &left_facts, &right_facts) {
        return class;
    }

    match (left_facts.node_id, right_facts.node_id) {
        (Some(left_node), Some(right_node)) if left_node == right_node => classify_same_node(
            baseline,
            text_resolver,
            &left_facts,
            &right_facts,
            left_node,
        ),
        _ => classify_cross_node(baseline, text_resolver, &left_facts, &right_facts),
    }
}

fn classify_path_relation(
    baseline: &NodeLifecycleState,
    left: &OperationFacts,
    right: &OperationFacts,
) -> Option<PairClass> {
    if !left
        .path_effects
        .newly_occupied
        .is_disjoint(&right.path_effects.newly_occupied)
    {
        let path = first_intersection(
            &left.path_effects.newly_occupied,
            &right.path_effects.newly_occupied,
        );
        return Some(conflict(
            ConflictWitnessKind::SamePathCreate,
            left,
            right,
            common_node(left, right),
            path,
        ));
    }

    if !left
        .path_effects
        .freed
        .is_disjoint(&right.path_effects.required_free)
    {
        let path = first_intersection(&left.path_effects.freed, &right.path_effects.required_free);
        if !is_delete_preimage_valid(baseline, left)
            || !is_create_after_delete_valid(baseline, left, right)
        {
            return Some(conflict(
                ConflictWitnessKind::LiveStateMismatch,
                left,
                right,
                common_node(left, right),
                path,
            ));
        }
        return Some(ordered(
            RequiredOrder::LeftBeforeRight,
            ConflictWitnessKind::LiveStateMismatch,
            left,
            right,
            common_node(left, right),
            path,
        ));
    }

    if !right
        .path_effects
        .freed
        .is_disjoint(&left.path_effects.required_free)
    {
        let path = first_intersection(&right.path_effects.freed, &left.path_effects.required_free);
        if !is_delete_preimage_valid(baseline, right)
            || !is_create_after_delete_valid(baseline, right, left)
        {
            return Some(conflict(
                ConflictWitnessKind::LiveStateMismatch,
                left,
                right,
                common_node(left, right),
                path,
            ));
        }
        return Some(ordered(
            RequiredOrder::RightBeforeLeft,
            ConflictWitnessKind::LiveStateMismatch,
            left,
            right,
            common_node(left, right),
            path,
        ));
    }

    if !left
        .path_effects
        .freed
        .is_disjoint(&right.path_effects.freed)
    {
        let path = first_intersection(&left.path_effects.freed, &right.path_effects.freed);
        return Some(conflict(
            ConflictWitnessKind::DeleteMutationConflict,
            left,
            right,
            common_node(left, right),
            path,
        ));
    }

    None
}

fn classify_cross_node<R: BaselineTextResolver>(
    baseline: &NodeLifecycleState,
    text_resolver: &R,
    left: &OperationFacts,
    right: &OperationFacts,
) -> PairClass {
    if let Some(class) = invalid_preimage_class(baseline, text_resolver, left, right) {
        return class;
    }
    if let Some(class) = invalid_preimage_class(baseline, text_resolver, right, left) {
        return class;
    }
    PairClass::Independent
}

fn classify_same_node<R: BaselineTextResolver>(
    baseline: &NodeLifecycleState,
    text_resolver: &R,
    left: &OperationFacts,
    right: &OperationFacts,
    node_id: NodeId,
) -> PairClass {
    if let Some(class) = classify_create_then_mutate(baseline, text_resolver, left, right, node_id)
    {
        return class;
    }
    if let Some(class) = classify_mutate_then_delete(baseline, left, right, node_id) {
        return class;
    }
    match (&left.action, &right.action) {
        (
            Action::EditText {
                span_id: left_span, ..
            },
            Action::EditText {
                span_id: right_span,
                ..
            },
        ) if left_span == right_span => conflict_with_span(
            ConflictWitnessKind::TextSpanOverlap,
            left,
            right,
            node_id,
            *left_span,
        ),
        (Action::EditText { .. }, Action::EditText { .. }) => unknown_from_facts(
            UnknownReason::SameNodeTextCommutationDeferred,
            left,
            right,
            Some(node_id),
            None,
        ),
        (Action::ChangePerm { old_mode, .. }, Action::ReplaceBinary { old_blob_id, .. })
        | (Action::ReplaceBinary { old_blob_id, .. }, Action::ChangePerm { old_mode, .. }) => {
            if baseline_file_matches(
                baseline,
                node_id,
                NodeKind::BinaryFile,
                *old_blob_id,
                *old_mode,
            ) {
                PairClass::Independent
            } else {
                conflict(
                    ConflictWitnessKind::LiveStateMismatch,
                    left,
                    right,
                    Some(node_id),
                    None,
                )
            }
        }
        (Action::ChangePerm { old_mode, .. }, edit @ Action::EditText { .. })
        | (edit @ Action::EditText { .. }, Action::ChangePerm { old_mode, .. }) => {
            classify_mode_and_text_edit(
                baseline,
                text_resolver,
                left,
                right,
                node_id,
                *old_mode,
                edit,
            )
        }
        _ => conflict(
            ConflictWitnessKind::UnknownRelation,
            left,
            right,
            Some(node_id),
            None,
        ),
    }
}
