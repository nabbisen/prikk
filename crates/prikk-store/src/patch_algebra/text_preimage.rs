use prikk_object::{NodeId, NodeKind, text_span_hash};

use crate::node_lifecycle::{NodeContent, NodeLifecycleState};
use crate::text_span;

use super::preimage::PreimageStatus;
use super::types::{BaselineTextResolver, ConflictWitnessKind, UnknownReason};

pub(super) struct TextPreimage<'a> {
    pub(super) span_id: [u8; 32],
    pub(super) old_span_hash: &'a [u8; 32],
    pub(super) left_anchor_hash: &'a [u8; 32],
    pub(super) right_anchor_hash: &'a [u8; 32],
    pub(super) old_span_text: &'a [u8],
}

pub(super) fn validate_text_preimage<R: BaselineTextResolver>(
    baseline: &NodeLifecycleState,
    text_resolver: &R,
    node_id: NodeId,
    preimage: TextPreimage<'_>,
) -> PreimageStatus {
    if text_span_hash(preimage.old_span_text) != *preimage.old_span_hash {
        return PreimageStatus::Conflict {
            kind: ConflictWitnessKind::TextAnchorStale,
            node_id: Some(node_id),
            path: None,
        };
    }
    let Some(live) = baseline.live_node(&node_id) else {
        return PreimageStatus::Conflict {
            kind: ConflictWitnessKind::LiveStateMismatch,
            node_id: Some(node_id),
            path: None,
        };
    };
    if live.kind != NodeKind::TextFile {
        return PreimageStatus::Conflict {
            kind: ConflictWitnessKind::KindMismatch,
            node_id: Some(node_id),
            path: Some(live.path.clone()),
        };
    }
    let NodeContent::File { blob_id, .. } = &live.content else {
        return PreimageStatus::Conflict {
            kind: ConflictWitnessKind::KindMismatch,
            node_id: Some(node_id),
            path: Some(live.path.clone()),
        };
    };
    let Some(current_text) = text_resolver.text_content(node_id, *blob_id) else {
        return PreimageStatus::Unknown {
            reason: UnknownReason::SameNodeTextCommutationDeferred,
            node_id: Some(node_id),
            path: Some(live.path.clone()),
        };
    };
    match text_span::locate_text_span(
        &current_text,
        preimage.old_span_text,
        preimage.left_anchor_hash,
        preimage.right_anchor_hash,
        &preimage.span_id,
        node_id,
        preimage.old_span_hash,
    ) {
        Ok(_) => PreimageStatus::Valid,
        Err(_) => PreimageStatus::Conflict {
            kind: ConflictWitnessKind::TextAnchorStale,
            node_id: Some(node_id),
            path: Some(live.path.clone()),
        },
    }
}
