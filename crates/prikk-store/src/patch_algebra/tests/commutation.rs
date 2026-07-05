use super::*;

#[test]
fn independent_pair_commutes_only_with_replay_oracle_success() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "left.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "right.bin", blob(2), MODE_REGULAR);
    let left = replace_binary(11, node(1), blob(1), blob(3));
    let right = change_perm(17, node(2), MODE_REGULAR, MODE_EXECUTABLE);
    let evidence = TestTextResolver::empty().with_blob(blob(3), BlobKind::Binary, b"new".to_vec());

    match commute_pair_result(
        &baseline,
        &evidence,
        EvidenceScope::SealedCandidateRequired,
        &left,
        &right,
    )
    .expect("commutation evidence")
    {
        CommutationResult::Commutes { proof } => {
            assert_eq!(proof.left_op_seq, 11);
            assert_eq!(proof.right_op_seq, 17);
        }
        other => panic!("expected commutes, got {other:?}"),
    }
}

#[test]
fn concrete_ordered_dependency_does_not_commute() {
    let baseline = NodeLifecycleState::new();
    let left = create_file(1, "fresh.bin", node(1), blob(1), MODE_REGULAR);
    let right = change_perm(2, node(1), MODE_REGULAR, MODE_EXECUTABLE);
    let evidence =
        TestTextResolver::empty().with_blob(blob(1), BlobKind::Binary, b"fresh".to_vec());

    match commute_pair_result(
        &baseline,
        &evidence,
        EvidenceScope::UnsealedCandidateOptional,
        &left,
        &right,
    )
    .expect("commutation evidence")
    {
        CommutationResult::DoesNotCommute {
            pair_class: PairClass::OrderedDependency { required_order, .. },
        } => assert_eq!(required_order, RequiredOrder::LeftBeforeRight),
        other => panic!("expected ordered does-not-commute, got {other:?}"),
    }
}

#[test]
fn same_node_text_pair_never_commutes() {
    let mut baseline = NodeLifecycleState::new();
    let old = b"alpha beta gamma";
    seed_text(&mut baseline, node(1), "doc.txt", old, MODE_REGULAR);
    let evidence = TestTextResolver::new([(node(1), old.to_vec())]);
    let left = edit_text(1, node(1), old, b"alpha BETA gamma");
    let right = edit_text(2, node(1), old, b"alpha beta GAMMA");

    match commute_pair_result(
        &baseline,
        &evidence,
        EvidenceScope::SealedCandidateRequired,
        &left,
        &right,
    )
    .expect("commutation evidence")
    {
        CommutationResult::Unknown { reason } => {
            assert_eq!(reason, UnknownReason::SameNodeTextCommutationDeferred);
        }
        other => panic!("expected text deferral, got {other:?}"),
    }
}

#[test]
fn malformed_sealed_candidate_operation_is_evidence_error() {
    let baseline = NodeLifecycleState::new();
    let left = create_file(1, "../escape", node(1), blob(1), MODE_REGULAR);
    let right = create_file(2, "ok.bin", node(2), blob(2), MODE_REGULAR);

    match commute_pair_result(
        &baseline,
        &TestTextResolver::empty(),
        EvidenceScope::SealedCandidateRequired,
        &left,
        &right,
    ) {
        Err(EvidenceError::Malformed {
            scope,
            fact: EvidenceFact::Operation,
            ..
        }) => assert_eq!(scope, EvidenceScope::SealedCandidateRequired),
        other => panic!("expected sealed malformed evidence error, got {other:?}"),
    }
}

#[test]
fn malformed_unsealed_candidate_is_unknown_not_commutes() {
    let baseline = NodeLifecycleState::new();
    let left = create_file(1, "../escape", node(1), blob(1), MODE_REGULAR);
    let right = create_file(2, "ok.bin", node(2), blob(2), MODE_REGULAR);

    match commute_pair_result(
        &baseline,
        &TestTextResolver::empty(),
        EvidenceScope::UnsealedCandidateOptional,
        &left,
        &right,
    )
    .expect("unsealed malformed stays algebraic")
    {
        CommutationResult::Unknown { reason } => {
            assert_eq!(reason, UnknownReason::MalformedOperation);
        }
        other => panic!("expected unsealed malformed unknown, got {other:?}"),
    }
}

#[test]
fn missing_unsealed_create_evidence_is_unknown_not_commutes() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(9), "other.bin", blob(9), MODE_REGULAR);
    let left = create_file(1, "fresh.bin", node(1), blob(1), MODE_REGULAR);
    let right = replace_binary(2, node(9), blob(9), blob(10));

    match commute_pair_result(
        &baseline,
        &TestTextResolver::empty(),
        EvidenceScope::UnsealedCandidateOptional,
        &left,
        &right,
    )
    .expect("missing unsealed candidate evidence remains algebraic")
    {
        CommutationResult::Unknown { reason } => {
            assert_eq!(reason, UnknownReason::MissingCandidateEvidence);
        }
        other => panic!("expected missing candidate unknown, got {other:?}"),
    }
}

#[test]
fn missing_sealed_create_evidence_is_evidence_error() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(9), "other.bin", blob(9), MODE_REGULAR);
    let left = create_file(1, "fresh.bin", node(1), blob(1), MODE_REGULAR);
    let right = replace_binary(2, node(9), blob(9), blob(10));

    assert_evidence_error(
        commute_pair_result(
            &baseline,
            &TestTextResolver::empty(),
            EvidenceScope::SealedCandidateRequired,
            &left,
            &right,
        )
        .map(|result| match result {
            CommutationResult::DoesNotCommute { pair_class } => pair_class,
            CommutationResult::Commutes { .. } | CommutationResult::Unknown { .. } => {
                PairClass::Independent
            }
        }),
        EvidenceScope::SealedCandidateRequired,
        EvidenceFact::BlobKind,
    );
}
