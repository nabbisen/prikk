use super::*;

#[test]
fn different_node_binary_replacements_are_independent_and_commute() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "a.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "b.bin", blob(2), MODE_REGULAR);
    let left = replace_binary(17, node(1), blob(1), blob(3));
    let right = replace_binary(23, node(2), blob(2), blob(4));

    assert_eq!(
        classify_pair(&baseline, &left, &right),
        PairClass::Independent
    );
    assert_swapped_orders_equal(&baseline, &left, &right);
    assert_eq!(left.op_seq, 17);
    assert_eq!(right.op_seq, 23);
}

#[test]
fn different_node_replace_binary_with_stale_blob_is_conflict_not_independent() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "a.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "b.bin", blob(2), MODE_REGULAR);
    let left = replace_binary(1, node(1), blob(9), blob(3));
    let right = replace_binary(2, node(2), blob(2), blob(4));

    assert_conflict(
        classify_pair(&baseline, &left, &right),
        ConflictWitnessKind::BlobMismatch,
    );
}

#[test]
fn different_node_change_perm_with_stale_mode_is_conflict_not_independent() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "a.bin", blob(1), MODE_REGULAR);
    seed_binary(&mut baseline, node(2), "b.bin", blob(2), MODE_REGULAR);
    let left = change_perm(1, node(1), MODE_EXECUTABLE, MODE_REGULAR);
    let right = replace_binary(2, node(2), blob(2), blob(4));

    assert_conflict(
        classify_pair(&baseline, &left, &right),
        ConflictWitnessKind::ModeMismatch,
    );
}

#[test]
fn different_node_text_edits_are_independent_when_resolver_proves_preimages() {
    let mut baseline = NodeLifecycleState::new();
    let left_old = b"alpha beta gamma";
    let right_old = b"one two three";
    seed_text(&mut baseline, node(1), "left.txt", left_old, MODE_REGULAR);
    seed_text(&mut baseline, node(2), "right.txt", right_old, MODE_REGULAR);
    let text_resolver =
        TestTextResolver::new([(node(1), left_old.to_vec()), (node(2), right_old.to_vec())]);
    let left = edit_text(1, node(1), left_old, b"alpha BETA gamma");
    let right = edit_text(2, node(2), right_old, b"one TWO three");

    assert_eq!(
        classify_pair_with_text_resolver(&baseline, &text_resolver, &left, &right),
        PairClass::Independent
    );
    assert_swapped_orders_equal_with_text(&baseline, &text_resolver, &left, &right);
    assert_evidence_error(
        classify_pair_result(&baseline, &left, &right),
        EvidenceScope::SealedBaselineRequired,
        EvidenceFact::BaselineText,
    );
}

#[test]
fn same_node_mode_and_binary_content_are_independent_when_preimages_match() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "tool", blob(1), MODE_REGULAR);
    let left = change_perm(4, node(1), MODE_REGULAR, MODE_EXECUTABLE);
    let right = replace_binary(8, node(1), blob(1), blob(2));

    assert_eq!(
        classify_pair(&baseline, &left, &right),
        PairClass::Independent
    );
    assert_swapped_orders_equal(&baseline, &left, &right);
}
