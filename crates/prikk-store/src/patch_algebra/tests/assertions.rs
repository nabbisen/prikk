use super::*;
use crate::patch_replay::decode::DecodedPatchOperation;

pub(super) fn assert_swapped_orders_equal(
    baseline: &NodeLifecycleState,
    left: &DecodedPatchOperation,
    right: &DecodedPatchOperation,
) {
    let mut left_then_right = baseline.clone();
    apply_for_oracle(&mut left_then_right, left).expect("left applies");
    apply_for_oracle(&mut left_then_right, right).expect("right applies");

    let mut right_then_left = baseline.clone();
    apply_for_oracle(&mut right_then_left, right).expect("right applies");
    apply_for_oracle(&mut right_then_left, left).expect("left applies");

    assert_eq!(left_then_right, right_then_left);
}

pub(super) fn assert_swapped_orders_equal_with_text(
    baseline: &NodeLifecycleState,
    text_resolver: &TestTextResolver,
    left: &DecodedPatchOperation,
    right: &DecodedPatchOperation,
) {
    let mut left_then_right = baseline.clone();
    let mut left_text = text_resolver.texts.clone();
    apply_for_oracle_with_text(&mut left_then_right, &mut left_text, left).expect("left applies");
    apply_for_oracle_with_text(&mut left_then_right, &mut left_text, right).expect("right applies");

    let mut right_then_left = baseline.clone();
    let mut right_text = text_resolver.texts.clone();
    apply_for_oracle_with_text(&mut right_then_left, &mut right_text, right)
        .expect("right applies");
    apply_for_oracle_with_text(&mut right_then_left, &mut right_text, left).expect("left applies");

    assert_eq!(left_then_right, right_then_left);
    assert_eq!(left_text, right_text);
}

pub(super) fn assert_order(class: PairClass, required_order: RequiredOrder) {
    match class {
        PairClass::OrderedDependency {
            required_order: actual,
            witness,
        } => {
            assert_eq!(actual, required_order);
            assert_eq!(witness.kind, ConflictWitnessKind::LiveStateMismatch);
        }
        other => panic!("expected ordered dependency, got {other:?}"),
    }
}

pub(super) fn assert_unknown(class: PairClass, reason: UnknownReason) {
    match class {
        PairClass::Unknown {
            reason: actual,
            witness,
        } => {
            assert_eq!(actual, reason);
            assert_ne!(witness.kind, ConflictWitnessKind::SamePathCreate);
        }
        other => panic!("expected unknown, got {other:?}"),
    }
}

pub(super) fn assert_conflict(class: PairClass, kind: ConflictWitnessKind) {
    match class {
        PairClass::Conflict { witness } => assert_eq!(witness.kind, kind),
        other => panic!("expected conflict {kind:?}, got {other:?}"),
    }
}

pub(super) fn assert_evidence_error(
    result: Result<PairClass, EvidenceError>,
    scope: EvidenceScope,
    fact: EvidenceFact,
) {
    match result {
        Err(EvidenceError::Missing {
            scope: actual_scope,
            fact: actual_fact,
            ..
        }) => {
            assert_eq!(actual_scope, scope);
            assert_eq!(actual_fact, fact);
        }
        Err(other) => panic!("expected missing evidence error, got {other:?}"),
        Ok(class) => panic!("expected evidence error, got {class:?}"),
    }
}
