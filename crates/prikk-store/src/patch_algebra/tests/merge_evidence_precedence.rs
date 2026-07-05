use super::*;

#[test]
fn required_sealed_baseline_evidence_beats_malformed_unsealed_candidate_in_merge_report() {
    let old = b"alpha beta";
    let mut baseline = NodeLifecycleState::new();
    seed_text(&mut baseline, node(1), "doc.txt", old, MODE_REGULAR);
    let left = [create_file(1, "/absolute", node(2), blob(2), MODE_REGULAR)];
    let right = [edit_text(2, node(1), old, b"alpha BETA")];

    let report = analyze_merge_evidence(
        blob(0xb0),
        None,
        &baseline,
        &TestTextResolver::empty(),
        EvidenceScope::UnsealedCandidateOptional,
        &left,
        &right,
    );

    assert_eq!(report.outcome, MergeEvidenceOutcome::EvidenceFailure);
    assert_eq!(
        first_item(&report.items).evidence_scope,
        Some(MergeEvidenceScope::SealedBaseline)
    );
    assert_eq!(
        first_item(&report.items).reason_code,
        MergeEvidenceReasonCode::MissingRequiredEvidence
    );
}

#[test]
fn required_sealed_baseline_evidence_beats_malformed_unsealed_candidate_in_pair_report() {
    let old = b"alpha beta";
    let mut baseline = NodeLifecycleState::new();
    seed_text(&mut baseline, node(1), "doc.txt", old, MODE_REGULAR);
    let left = create_file(1, "/absolute", node(2), blob(2), MODE_REGULAR);
    let right = edit_text(2, node(1), old, b"alpha BETA");

    let report = analyze_pair_merge_evidence(
        blob(0xb0),
        None,
        &baseline,
        &TestTextResolver::empty(),
        EvidenceScope::UnsealedCandidateOptional,
        &left,
        &right,
    );

    assert_eq!(report.outcome, MergeEvidenceOutcome::EvidenceFailure);
    assert_eq!(
        first_item(&report.items).evidence_scope,
        Some(MergeEvidenceScope::SealedBaseline)
    );
    assert_eq!(
        first_item(&report.items).reason_code,
        MergeEvidenceReasonCode::MissingRequiredEvidence
    );
}

#[test]
fn malformed_sealed_candidate_operation_is_evidence_failure_in_merge_report() {
    let baseline = NodeLifecycleState::new();
    let left = [rename_path(1, node(1), "/absolute", "moved")];
    let right = [change_perm(2, node(2), MODE_REGULAR, MODE_EXECUTABLE)];

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
        MergeEvidenceReasonCode::MalformedRequiredEvidence
    );
}

#[test]
fn malformed_sealed_candidate_operation_is_evidence_failure_in_pair_report() {
    let baseline = NodeLifecycleState::new();
    let left = rename_path(1, node(1), "/absolute", "moved");
    let right = change_perm(2, node(2), MODE_REGULAR, MODE_EXECUTABLE);

    let report = analyze_pair_merge_evidence(
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
        MergeEvidenceReasonCode::MalformedRequiredEvidence
    );
}

fn first_item(items: &[MergeEvidenceItem]) -> &MergeEvidenceItem {
    items.first().expect("report item")
}
