use super::*;

#[test]
fn missing_sealed_replace_binary_new_blob_is_evidence_error() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "left.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "right.bin", blob(2), MODE_REGULAR);
    let left = replace_binary(1, node(1), blob(1), blob(3));
    let right = change_perm(2, node(2), MODE_REGULAR, MODE_EXECUTABLE);

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

#[test]
fn missing_unsealed_replace_binary_new_blob_is_unknown() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "left.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "right.bin", blob(2), MODE_REGULAR);
    let left = replace_binary(1, node(1), blob(1), blob(3));
    let right = change_perm(2, node(2), MODE_REGULAR, MODE_EXECUTABLE);

    match commute_pair_result(
        &baseline,
        &TestTextResolver::empty(),
        EvidenceScope::UnsealedCandidateOptional,
        &left,
        &right,
    )
    .expect("unsealed missing replacement evidence remains algebraic")
    {
        CommutationResult::Unknown { reason } => {
            assert_eq!(reason, UnknownReason::MissingCandidateEvidence);
        }
        other => panic!("expected missing replacement evidence unknown, got {other:?}"),
    }
}

#[test]
fn non_binary_replace_binary_new_blob_is_evidence_error() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "left.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "right.bin", blob(2), MODE_REGULAR);
    let left = replace_binary(1, node(1), blob(1), blob(3));
    let right = change_perm(2, node(2), MODE_REGULAR, MODE_EXECUTABLE);
    let evidence = TestTextResolver::empty().with_blob(blob(3), BlobKind::Text, b"text".to_vec());

    match commute_pair_result(
        &baseline,
        &evidence,
        EvidenceScope::SealedCandidateRequired,
        &left,
        &right,
    ) {
        Err(EvidenceError::WrongBlobKind {
            scope,
            blob_id,
            expected,
            actual,
        }) => {
            assert_eq!(scope, EvidenceScope::SealedCandidateRequired);
            assert_eq!(blob_id, blob(3));
            assert_eq!(expected, BlobKind::Binary);
            assert_eq!(actual, BlobKind::Text);
        }
        other => panic!("expected wrong replacement blob kind, got {other:?}"),
    }
}

#[test]
fn wrong_object_type_replace_binary_new_blob_is_evidence_error() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "left.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "right.bin", blob(2), MODE_REGULAR);
    let left = replace_binary(1, node(1), blob(1), blob(3));
    let right = change_perm(2, node(2), MODE_REGULAR, MODE_EXECUTABLE);
    let evidence = TestTextResolver::empty().with_blob_kind_evidence(
        blob(3),
        Evidence::WrongObjectType {
            scope: EvidenceScope::SealedCandidateRequired,
            object_id: blob(3),
            expected: ObjectType::Blob,
            actual: ObjectType::Patch,
        },
    );

    match commute_pair_result(
        &baseline,
        &evidence,
        EvidenceScope::SealedCandidateRequired,
        &left,
        &right,
    ) {
        Err(EvidenceError::WrongObjectType {
            scope,
            object_id,
            expected,
            actual,
        }) => {
            assert_eq!(scope, EvidenceScope::SealedCandidateRequired);
            assert_eq!(object_id, blob(3));
            assert_eq!(expected, ObjectType::Blob);
            assert_eq!(actual, ObjectType::Patch);
        }
        other => panic!("expected wrong replacement object type, got {other:?}"),
    }
}

#[test]
fn malformed_replace_binary_new_blob_is_evidence_error() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "left.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "right.bin", blob(2), MODE_REGULAR);
    let left = replace_binary(1, node(1), blob(1), blob(3));
    let right = change_perm(2, node(2), MODE_REGULAR, MODE_EXECUTABLE);
    let evidence = TestTextResolver::empty().with_blob_kind_evidence(
        blob(3),
        Evidence::Malformed {
            scope: EvidenceScope::SealedCandidateRequired,
            fact: EvidenceFact::BlobKind,
            object_id: Some(blob(3)),
            reason: "bad blob payload".to_string(),
        },
    );

    match commute_pair_result(
        &baseline,
        &evidence,
        EvidenceScope::SealedCandidateRequired,
        &left,
        &right,
    ) {
        Err(EvidenceError::Malformed {
            scope,
            fact,
            object_id,
            ..
        }) => {
            assert_eq!(scope, EvidenceScope::SealedCandidateRequired);
            assert_eq!(fact, EvidenceFact::BlobKind);
            assert_eq!(object_id, Some(blob(3)));
        }
        other => panic!("expected malformed replacement blob evidence, got {other:?}"),
    }
}
