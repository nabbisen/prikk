use super::*;

#[test]
fn confluent_report_includes_required_baseline_and_sequence_summaries() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "left.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "right.bin", blob(2), MODE_REGULAR);
    let left = [replace_binary(10, node(1), blob(1), blob(3))];
    let right = [change_perm(20, node(2), MODE_REGULAR, MODE_EXECUTABLE)];
    let evidence = TestTextResolver::empty().with_blob(blob(3), BlobKind::Binary, b"new".to_vec());

    let report = analyze_merge_evidence(
        blob(0xb0),
        Some(blob(0xa0)),
        &baseline,
        &evidence,
        EvidenceScope::SealedCandidateRequired,
        &left,
        &right,
    );

    assert_eq!(report.baseline_block_id, blob(0xb0));
    assert_eq!(report.replay_horizon, Some(blob(0xa0)));
    assert_eq!(report.outcome, MergeEvidenceOutcome::Confluent);
    assert_eq!(report.left_sequence.label, "left");
    assert_eq!(report.left_sequence.operation_count, 1);
    let left_operation = report
        .left_sequence
        .operations
        .first()
        .expect("left operation summary");
    assert_eq!(
        left_operation.operation_kind,
        MergeEvidenceOperationKind::ReplaceBinary
    );
    assert_eq!(left_operation.op_seq, 10);
    assert_eq!(
        first_item(&report.items).reason_code,
        MergeEvidenceReasonCode::ProvenConfluent
    );
}

#[test]
fn concrete_conflict_report_is_not_generic_not_confluent() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "tool", blob(1), MODE_REGULAR);
    let left = [change_perm(1, node(1), MODE_REGULAR, MODE_EXECUTABLE)];
    let right = [change_perm(2, node(1), MODE_REGULAR, 0o100600)];

    let report = analyze_merge_evidence(
        blob(0xb0),
        None,
        &baseline,
        &TestTextResolver::empty(),
        EvidenceScope::SealedCandidateRequired,
        &left,
        &right,
    );

    assert_eq!(report.outcome, MergeEvidenceOutcome::Conflict);
    assert_eq!(
        first_item(&report.items).reason_code,
        MergeEvidenceReasonCode::PairConflict
    );
    let item = first_item(&report.items);
    assert_eq!(item.operation_index, Some(0));
    assert_eq!(item.peer_operation_index, Some(0));
    assert_eq!(item.op_seq, Some(1));
    assert_eq!(item.peer_op_seq, Some(2));
}

#[test]
fn ordered_dependency_report_has_dedicated_outcome() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "same.bin", blob(1), MODE_REGULAR);
    let left = delete_file(
        1,
        "same.bin",
        node(1),
        NodeKind::BinaryFile,
        blob(1),
        MODE_REGULAR,
    );
    let right = create_file(2, "same.bin", node(2), blob(2), MODE_REGULAR);
    let evidence =
        TestTextResolver::empty().with_blob(blob(2), BlobKind::Binary, b"fresh".to_vec());

    let report = analyze_pair_merge_evidence(
        blob(0xb0),
        None,
        &baseline,
        &evidence,
        EvidenceScope::UnsealedCandidateOptional,
        &left,
        &right,
    );

    assert_eq!(report.outcome, MergeEvidenceOutcome::OrderedDependency);
    assert_eq!(
        first_item(&report.items).reason_code,
        MergeEvidenceReasonCode::OrderedDependency
    );
}

#[test]
fn unsupported_operation_report_does_not_expose_unknown() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "left.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "right.bin", blob(2), MODE_REGULAR);
    let left = [rename_path(1, node(1), "left.bin", "moved.bin")];
    let right = [replace_binary(2, node(2), blob(2), blob(3))];
    let evidence = TestTextResolver::empty().with_blob(blob(3), BlobKind::Binary, b"new".to_vec());

    let report = analyze_merge_evidence(
        blob(0xb0),
        None,
        &baseline,
        &evidence,
        EvidenceScope::SealedCandidateRequired,
        &left,
        &right,
    );

    assert_eq!(report.outcome, MergeEvidenceOutcome::Unsupported);
    assert_eq!(
        first_item(&report.items).reason_code,
        MergeEvidenceReasonCode::UnsupportedOperation
    );
    assert!(!format!("{report:?}").contains("Unknown"));
}

#[test]
fn same_node_text_transform_is_deferred_not_not_confluent() {
    let old = b"alpha beta gamma";
    let mut baseline = NodeLifecycleState::new();
    seed_text(&mut baseline, node(1), "doc.txt", old, MODE_REGULAR);
    let left = [edit_text(1, node(1), old, b"alpha BETA gamma")];
    let right = [edit_text(2, node(1), old, b"alpha beta GAMMA")];
    let evidence = TestTextResolver::new([(node(1), old.to_vec())]);

    let report = analyze_merge_evidence(
        blob(0xb0),
        None,
        &baseline,
        &evidence,
        EvidenceScope::SealedCandidateRequired,
        &left,
        &right,
    );

    assert_eq!(report.outcome, MergeEvidenceOutcome::Deferred);
    assert_eq!(
        first_item(&report.items).reason_code,
        MergeEvidenceReasonCode::SameNodeTextTransformDeferred
    );
}

#[test]
fn required_sealed_evidence_failure_has_highest_public_outcome() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "left.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "right.bin", blob(2), MODE_REGULAR);
    let left = [rename_path(1, node(1), "left.bin", "moved.bin")];
    let right = [replace_binary(2, node(2), blob(2), blob(3))];

    let report = analyze_merge_evidence(
        blob(0xb0),
        None,
        &baseline,
        &TestTextResolver::empty(),
        EvidenceScope::SealedCandidateRequired,
        &left,
        &right,
    );

    assert_eq!(report.outcome, MergeEvidenceOutcome::EvidenceFailure);
    assert_eq!(
        first_item(&report.items).evidence_scope,
        Some(MergeEvidenceScope::SealedCandidate)
    );
    assert_eq!(
        first_item(&report.items).reason_code,
        MergeEvidenceReasonCode::MissingRequiredEvidence
    );
}

#[test]
fn optional_unsealed_candidate_evidence_failure_is_invalid_candidate() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "bin", blob(1), MODE_REGULAR);
    let left = [replace_binary(1, node(1), blob(1), blob(2))];
    let right = [change_perm(2, node(1), MODE_REGULAR, MODE_EXECUTABLE)];

    let report = analyze_merge_evidence(
        blob(0xb0),
        None,
        &baseline,
        &TestTextResolver::empty(),
        EvidenceScope::UnsealedCandidateOptional,
        &left,
        &right,
    );

    assert_eq!(report.outcome, MergeEvidenceOutcome::InvalidCandidate);
    assert_eq!(
        first_item(&report.items).reason_code,
        MergeEvidenceReasonCode::InsufficientUnsealedCandidateEvidence
    );
}

#[test]
fn malformed_unsealed_candidate_operation_is_invalid_candidate() {
    let baseline = NodeLifecycleState::new();
    let left = [create_file(1, "/absolute", node(1), blob(1), MODE_REGULAR)];
    let right = [change_perm(2, node(2), MODE_REGULAR, MODE_EXECUTABLE)];

    let report = analyze_merge_evidence(
        blob(0xb0),
        None,
        &baseline,
        &TestTextResolver::empty(),
        EvidenceScope::UnsealedCandidateOptional,
        &left,
        &right,
    );

    assert_eq!(report.outcome, MergeEvidenceOutcome::InvalidCandidate);
    assert_eq!(
        first_item(&report.items).reason_code,
        MergeEvidenceReasonCode::InvalidUnsealedCandidate
    );
    assert_eq!(
        report
            .left_sequence
            .operations
            .first()
            .expect("left operation summary")
            .path,
        None
    );
}

fn first_item(items: &[MergeEvidenceItem]) -> &MergeEvidenceItem {
    items.first().expect("report item")
}
