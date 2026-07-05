use super::*;

#[test]
fn create_file_path_effect_declares_required_free() {
    let operation = create_file(7, "src/lib.rs", node(1), blob(1), MODE_REGULAR);

    let effects = path_effects(&operation).expect("path effects");

    let path = path("src/lib.rs");
    assert!(effects.required_free.contains(&path));
    assert!(effects.newly_occupied.contains(&path));
    assert!(effects.occupied_after.contains(&path));
    assert!(!effects.freed.contains(&path));
}

#[test]
fn create_file_to_occupied_baseline_path_is_conflict_not_independent() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(
        &mut baseline,
        node(1),
        "occupied.bin",
        blob(1),
        MODE_REGULAR,
    );
    seed_binary(&mut baseline, node(2), "other.bin", blob(2), MODE_REGULAR);
    let left = create_file(1, "occupied.bin", node(3), blob(3), MODE_REGULAR);
    let right = replace_binary(2, node(2), blob(2), blob(4));

    assert_conflict(
        classify_pair(&baseline, &left, &right),
        ConflictWitnessKind::SamePathCreate,
    );
}

#[test]
fn same_path_create_create_is_conflict() {
    let baseline = NodeLifecycleState::new();
    let left = create_file(1, "new.bin", node(1), blob(1), MODE_REGULAR);
    let right = create_file(2, "new.bin", node(2), blob(2), MODE_REGULAR);

    assert_conflict(
        classify_pair(&baseline, &left, &right),
        ConflictWitnessKind::SamePathCreate,
    );
}

#[test]
fn create_then_replace_binary_same_node_is_ordered_when_create_preimage_is_valid() {
    let baseline = NodeLifecycleState::new();
    let left = create_file(1, "fresh.bin", node(1), blob(1), MODE_REGULAR);
    let right = replace_binary(2, node(1), blob(1), blob(2));
    let evidence = TestTextResolver::empty()
        .with_blob(blob(1), BlobKind::Binary, b"old".to_vec())
        .with_blob(blob(2), BlobKind::Binary, b"new".to_vec());

    assert_order(
        classify_pair_with_text_resolver(&baseline, &evidence, &left, &right),
        RequiredOrder::LeftBeforeRight,
    );
    assert_unknown(
        classify_pair(&baseline, &left, &right),
        UnknownReason::FuturePreconditionDeferred,
    );
    let mut ordered_state = baseline.clone();
    let mut ordered_texts = BTreeMap::new();
    apply_for_oracle_with_evidence(&mut ordered_state, &mut ordered_texts, &evidence, &left)
        .expect("create applies");
    apply_for_oracle_with_evidence(&mut ordered_state, &mut ordered_texts, &evidence, &right)
        .expect("replace applies after create");
    let mut reversed_state = baseline.clone();
    let mut reversed_texts = BTreeMap::new();
    assert!(
        apply_for_oracle_with_evidence(
            &mut reversed_state,
            &mut reversed_texts,
            &evidence,
            &right,
        )
        .is_err()
    );
}

#[test]
fn create_text_then_replace_binary_is_not_ordered() {
    let baseline = NodeLifecycleState::new();
    let left = create_file(1, "fresh.txt", node(1), blob(1), MODE_REGULAR);
    let right = replace_binary(2, node(1), blob(1), blob(2));
    let evidence = TestTextResolver::empty().with_blob(blob(1), BlobKind::Text, b"old".to_vec());

    assert_conflict(
        classify_pair_with_text_resolver(&baseline, &evidence, &left, &right),
        ConflictWitnessKind::KindMismatch,
    );
}

#[test]
fn create_text_then_edit_text_is_ordered_only_with_text_evidence() {
    let baseline = NodeLifecycleState::new();
    let old_text = b"alpha beta gamma";
    let left = create_file(1, "fresh.txt", node(1), blob(1), MODE_REGULAR);
    let right = edit_text(2, node(1), old_text, b"alpha BETA gamma");
    let evidence = TestTextResolver::empty().with_blob(blob(1), BlobKind::Text, old_text.to_vec());

    assert_order(
        classify_pair_with_text_resolver(&baseline, &evidence, &left, &right),
        RequiredOrder::LeftBeforeRight,
    );
    assert_unknown(
        classify_pair(&baseline, &left, &right),
        UnknownReason::FuturePreconditionDeferred,
    );
    let mut ordered_state = baseline.clone();
    let mut ordered_texts = BTreeMap::new();
    apply_for_oracle_with_evidence(&mut ordered_state, &mut ordered_texts, &evidence, &left)
        .expect("create applies");
    apply_for_oracle_with_evidence(&mut ordered_state, &mut ordered_texts, &evidence, &right)
        .expect("edit applies after create");
}

#[test]
fn create_binary_then_edit_text_is_not_ordered() {
    let baseline = NodeLifecycleState::new();
    let old_text = b"alpha beta gamma";
    let left = create_file(1, "fresh.bin", node(1), blob(1), MODE_REGULAR);
    let right = edit_text(2, node(1), old_text, b"alpha BETA gamma");
    let evidence =
        TestTextResolver::empty().with_blob(blob(1), BlobKind::Binary, old_text.to_vec());

    assert_conflict(
        classify_pair_with_text_resolver(&baseline, &evidence, &left, &right),
        ConflictWitnessKind::KindMismatch,
    );
}

#[test]
fn invalid_create_then_same_node_mutate_is_conflict_not_ordered() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(
        &mut baseline,
        node(2),
        "occupied.bin",
        blob(9),
        MODE_REGULAR,
    );
    let left = create_file(1, "occupied.bin", node(1), blob(1), MODE_REGULAR);
    let right = change_perm(2, node(1), MODE_REGULAR, MODE_EXECUTABLE);

    assert_conflict(
        classify_pair(&baseline, &left, &right),
        ConflictWitnessKind::SamePathCreate,
    );
}
