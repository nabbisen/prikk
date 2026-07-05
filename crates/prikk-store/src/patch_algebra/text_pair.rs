use prikk_object::{NodeId, text_span_hash};

use crate::node_lifecycle::NodeLifecycleState;
use crate::text_span;

use super::preimage::baseline_text_blob_for_mode;
use super::types::{
    Action, BaselineTextResolver, ConflictWitnessKind, OperationFacts, PairClass, UnknownReason,
};
use super::witness::{conflict, conflict_with_span, unknown_from_facts};

pub(super) fn classify_mode_and_text_edit<R: BaselineTextResolver>(
    baseline: &NodeLifecycleState,
    text_resolver: &R,
    left: &OperationFacts,
    right: &OperationFacts,
    node_id: NodeId,
    old_mode: u32,
    edit: &Action,
) -> PairClass {
    let Some(current_blob_id) = baseline_text_blob_for_mode(baseline, node_id, old_mode) else {
        return conflict(
            ConflictWitnessKind::LiveStateMismatch,
            left,
            right,
            Some(node_id),
            None,
        );
    };
    let Some(current_text) = text_resolver.text_content(node_id, current_blob_id) else {
        return unknown_from_facts(
            UnknownReason::SameNodeTextCommutationDeferred,
            left,
            right,
            Some(node_id),
            None,
        );
    };
    match edit {
        Action::EditText {
            span_id,
            old_span_hash,
            left_anchor_hash,
            right_anchor_hash,
            old_span_text,
            ..
        } => {
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
                &current_text,
                old_span_text,
                left_anchor_hash,
                right_anchor_hash,
                span_id,
                node_id,
                old_span_hash,
            ) {
                Ok(_) => PairClass::Independent,
                Err(_) => conflict_with_span(
                    ConflictWitnessKind::TextAnchorStale,
                    left,
                    right,
                    node_id,
                    *span_id,
                ),
            }
        }
        _ => unreachable!("caller passes only EditText actions"),
    }
}
