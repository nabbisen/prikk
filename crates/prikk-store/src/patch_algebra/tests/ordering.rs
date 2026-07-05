use super::*;

#[test]
fn delete_then_create_same_path_is_ordered() {
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

    assert_order(
        classify_pair(&baseline, &left, &right),
        RequiredOrder::LeftBeforeRight,
    );
}

#[test]
fn stale_delete_then_create_same_path_is_conflict_not_ordered() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "same.bin", blob(1), MODE_REGULAR);
    let left = delete_file(
        1,
        "same.bin",
        node(1),
        NodeKind::BinaryFile,
        blob(9),
        MODE_REGULAR,
    );
    let right = create_file(2, "same.bin", node(2), blob(2), MODE_REGULAR);

    assert_conflict(
        classify_pair(&baseline, &left, &right),
        ConflictWitnessKind::LiveStateMismatch,
    );
}

#[test]
fn create_then_delete_same_path_is_ordered_without_renumbering() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "same.bin", blob(1), MODE_REGULAR);
    let left = create_file(41, "same.bin", node(2), blob(2), MODE_REGULAR);
    let right = delete_file(
        99,
        "same.bin",
        node(1),
        NodeKind::BinaryFile,
        blob(1),
        MODE_REGULAR,
    );

    assert_order(
        classify_pair(&baseline, &left, &right),
        RequiredOrder::RightBeforeLeft,
    );
    assert_eq!(left.op_seq, 41);
    assert_eq!(right.op_seq, 99);
}

#[test]
fn mutate_then_delete_is_ordered_when_delete_preimage_matches_post_mutation() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "tool", blob(1), MODE_REGULAR);
    let left = change_perm(1, node(1), MODE_REGULAR, MODE_EXECUTABLE);
    let right = delete_file(
        2,
        "tool",
        node(1),
        NodeKind::BinaryFile,
        blob(1),
        MODE_EXECUTABLE,
    );

    assert_order(
        classify_pair(&baseline, &left, &right),
        RequiredOrder::LeftBeforeRight,
    );
    let mut ordered_state = baseline.clone();
    apply_for_oracle(&mut ordered_state, &left).expect("mutation applies");
    apply_for_oracle(&mut ordered_state, &right).expect("delete applies after mutation");
    let mut reversed_state = baseline.clone();
    assert!(apply_for_oracle(&mut reversed_state, &right).is_err());
}
