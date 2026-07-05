use super::*;

#[test]
fn same_node_mode_and_text_edit_is_independent_only_when_span_matches_baseline() {
    let mut baseline = NodeLifecycleState::new();
    let old_text = b"alpha beta gamma";
    seed_text(&mut baseline, node(1), "note.txt", old_text, MODE_REGULAR);
    let text_resolver = TestTextResolver::new([(node(1), old_text.to_vec())]);
    let left = change_perm(1, node(1), MODE_REGULAR, MODE_EXECUTABLE);
    let right = edit_text(2, node(1), old_text, b"alpha BETA gamma");

    assert_eq!(
        classify_pair_with_text_resolver(&baseline, &text_resolver, &left, &right),
        PairClass::Independent
    );
    assert_swapped_orders_equal_with_text(&baseline, &text_resolver, &left, &right);

    let stale = edit_text(3, node(1), b"alpha stale gamma", b"alpha STALE gamma");
    match classify_pair_with_text_resolver(&baseline, &text_resolver, &left, &stale) {
        PairClass::Conflict { witness } => {
            assert_eq!(witness.kind, ConflictWitnessKind::TextAnchorStale);
        }
        other => panic!("expected stale text conflict, got {other:?}"),
    }
    assert_unknown(
        classify_pair(&baseline, &left, &right),
        UnknownReason::SameNodeTextCommutationDeferred,
    );
}

#[test]
fn same_node_identical_text_span_is_conflict_not_independent() {
    let mut baseline = NodeLifecycleState::new();
    seed_text(
        &mut baseline,
        node(1),
        "note.txt",
        b"alpha beta gamma",
        MODE_REGULAR,
    );
    let left = edit_text(1, node(1), b"alpha beta gamma", b"alpha BETA gamma");
    let mut right = edit_text(2, node(1), b"alpha beta gamma", b"alpha BETTER gamma");
    if let (
        DecodedOperationKind::EditText {
            span_id: right_span,
            ..
        },
        DecodedOperationKind::EditText {
            span_id: left_span, ..
        },
    ) = (&mut right.kind, &left.kind)
    {
        *right_span = *left_span;
    }

    match classify_pair(&baseline, &left, &right) {
        PairClass::Conflict { witness } => {
            assert_eq!(witness.kind, ConflictWitnessKind::TextSpanOverlap);
            assert!(witness.text_span.is_some());
        }
        other => panic!("expected conflict, got {other:?}"),
    }
}

#[test]
fn same_node_distinct_text_spans_are_unknown_not_independent() {
    let mut baseline = NodeLifecycleState::new();
    seed_text(
        &mut baseline,
        node(1),
        "note.txt",
        b"alpha beta gamma delta",
        MODE_REGULAR,
    );
    let left = edit_text(
        1,
        node(1),
        b"alpha beta gamma delta",
        b"alpha BETA gamma delta",
    );
    let right = edit_text(
        2,
        node(1),
        b"alpha beta gamma delta",
        b"alpha beta GAMMA delta",
    );

    assert_unknown(
        classify_pair(&baseline, &left, &right),
        UnknownReason::SameNodeTextCommutationDeferred,
    );
}

#[test]
fn same_node_two_mode_changes_to_different_modes_conflict() {
    let mut baseline = NodeLifecycleState::new();
    seed_binary(&mut baseline, node(1), "tool", blob(1), MODE_REGULAR);
    let left = change_perm(1, node(1), MODE_REGULAR, MODE_EXECUTABLE);
    let right = change_perm(2, node(1), MODE_REGULAR, 0o100600);

    assert_conflict(
        classify_pair(&baseline, &left, &right),
        ConflictWitnessKind::UnknownRelation,
    );
}
