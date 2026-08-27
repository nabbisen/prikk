use super::*;

#[test]
fn report_items_have_deterministic_secondary_ordering() {
    let mut items = vec![
        report_item_fixture(MergeEvidenceSide::Right, 1, 30, node(3)),
        report_item_fixture(MergeEvidenceSide::Left, 2, 20, node(2)),
        report_item_fixture(MergeEvidenceSide::Left, 1, 10, node(1)),
    ];

    super::super::report::sort_report_items(&mut items);

    let first = items.first().expect("first item");
    let second = items.get(1).expect("second item");
    let third = items.get(2).expect("third item");
    assert_eq!(first.side, MergeEvidenceSide::Left);
    assert_eq!(first.operation_index, Some(1));
    assert_eq!(second.side, MergeEvidenceSide::Left);
    assert_eq!(second.operation_index, Some(2));
    assert_eq!(third.side, MergeEvidenceSide::Right);
    assert_eq!(third.operation_index, Some(1));
}

fn report_item_fixture(
    side: MergeEvidenceSide,
    index: usize,
    op_seq: u32,
    node_id: prikk_object::NodeId,
) -> MergeEvidenceItem {
    MergeEvidenceItem {
        side,
        operation_index: Some(index),
        peer_operation_index: None,
        op_seq: Some(op_seq),
        peer_op_seq: None,
        operation_kind: Some(MergeEvidenceOperationKind::ChangePerm),
        node_id: Some(node_id),
        path: None,
        witness_kind: None,
        outcome: MergeEvidenceOutcome::Deferred,
        evidence_scope: None,
        proof_phase: MergeEvidenceProofPhase::ComposedReplay,
        reason_code: MergeEvidenceReasonCode::ComposedReplayFailed,
    }
}
