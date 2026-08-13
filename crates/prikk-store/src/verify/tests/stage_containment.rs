//! DC-95 Stage 2, Level 1: acceptance-criteria tests for scope containment itself (`stage-2-design-
//! v1.md` §11, the implementation handoff §6) -- distinct from Stage 1's per-check reachability
//! tests (`ref_cluster.rs`, `wal_cluster.rs`, `verify/tests.rs`), which prove a specific check is
//! load-bearing. These prove the *containment machinery* is sound: independent failures in
//! different stages are both reported, `NotEvaluated` correctly names its blocking dependency, and
//! the one correctness fix the Step 0 ruling required (`trust_is_valid` must not read `true` from a
//! trust check that never ran) actually holds.

use prikk_error::{PrikkError, Result};
use prikk_object::{
    BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope, ObjectId, ObjectType,
    RefKind, RefStatePayload, RefUpdatePayload,
};

use crate::maintainer_signing::MaintainerSigner;
use crate::test_support::{
    maintainer_signature as fake_maintainer_signature, sample_object_id,
    signed_patch_blob_envelope, signed_patch_envelope, unique_temp_dir,
};
use crate::{
    Ed25519MaintainerSigner, FileObjectStore, ObjectWriter, RefPublication, RefStore,
    RepositoryLayout, StageOutcome, StageStatus, VerificationStage, VerifyOptions, Wal,
    add_trusted_maintainer, maintainer_signature, verify_repository,
    verify_repository_with_options, write_active_ref_metadata,
};

fn trusted_signer(seed_label: &str, byte: u8) -> Result<Ed25519MaintainerSigner> {
    Ed25519MaintainerSigner::from_seed(seed_label, &[byte; 32])
}

fn adopt(layout: &RepositoryLayout, signer: &Ed25519MaintainerSigner) -> Result<()> {
    let public_key_hex: String = signer
        .public_key_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    add_trusted_maintainer(layout, signer.key_id(), &public_key_hex)?;
    Ok(())
}

/// Local copy of `ref_cluster.rs`'s own private helper of the same shape -- kept file-local rather
/// than shared, matching this test tree's existing convention of small per-file helpers.
fn build_signed_ref_update(
    ref_name: &str,
    old_ref_state_id: Option<ObjectId>,
    new_ref_state_id: ObjectId,
    new_target_object_id: ObjectId,
    update_seq: u64,
    signer: &Ed25519MaintainerSigner,
) -> Result<ObjectEnvelope> {
    let payload = RefUpdatePayload {
        ref_name: ref_name.to_string(),
        old_ref_state_id,
        new_ref_state_id,
        new_target_object_id,
        update_seq,
        created_at: 0,
        author_key_id: signer.key_id().to_string(),
    };
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::RefUpdate, 1, payload.to_canonical_bytes()?);
    let id = envelope.object_id();
    envelope.add_signature(maintainer_signature(signer, ObjectType::RefUpdate, id)?)?;
    Ok(envelope)
}

fn find_stage(outcomes: &[StageOutcome], stage: VerificationStage) -> &StageOutcome {
    outcomes
        .iter()
        .find(|outcome| outcome.stage == stage)
        .unwrap_or_else(|| panic!("expected a StageOutcome for stage {stage}, found none"))
}

/// Acceptance criterion 1 (design §11, handoff §6.1): a `Failed` stage does not suppress a later,
/// independent stage's own findings. Two defects with nothing in common -- an object-id-mismatched
/// Blob (`Objects`) and a checksum-corrupted active WAL record (`WalReplay`) -- planted in the same
/// repository. Both must appear `Failed` in the same report.
#[test]
fn verify_repository_reports_two_independent_stage_failures_together() -> Result<()> {
    let root = unique_temp_dir("stage2-two-independent-failures");
    let layout = RepositoryLayout::init(root.clone())?;

    // Objects: an id-mismatched Blob, same technique as `verify_repository_detects_object_id_
    // mismatch` (`verify/tests.rs`).
    let blob = prikk_object::BlobPayload::new(prikk_object::BlobKind::Text, b"payload".to_vec());
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, blob.to_canonical_bytes()?);
    envelope.add_signature(fake_maintainer_signature())?;
    let wrong_id = sample_object_id("stage2-wrong-id");
    let misplaced = layout.object_path(ObjectType::Blob, wrong_id);
    std::fs::create_dir_all(
        misplaced
            .parent()
            .ok_or_else(|| PrikkError::Io("misplaced object path has no parent".to_string()))?,
    )?;
    std::fs::write(
        &misplaced,
        crate::file_codec::encode_envelope_file(&envelope)?,
    )?;

    // WalReplay: a real, well-formed patch, then a corrupted checksum byte, same technique as
    // `verify_repository_detects_wal_checksum_mismatch` (`wal_cluster.rs`).
    let mut objects = FileObjectStore::new(layout.clone());
    objects.write_object(&signed_patch_blob_envelope())?;
    let patch = signed_patch_envelope();
    let wal = Wal::for_layout(&layout);
    wal.append_patch(&patch)?;
    let mut bytes = std::fs::read(wal.path())?;
    let last_byte = bytes
        .last_mut()
        .ok_or_else(|| PrikkError::Io("WAL file unexpectedly empty".to_string()))?;
    *last_byte ^= 0x01;
    std::fs::write(wal.path(), &bytes)?;

    let report = verify_repository(&layout)?;
    assert!(report.has_stage_failure());
    assert!(matches!(
        find_stage(&report.stage_outcomes, VerificationStage::Objects).status,
        StageStatus::Failed { .. }
    ));
    assert!(matches!(
        find_stage(&report.stage_outcomes, VerificationStage::WalReplay).status,
        StageStatus::Failed { .. }
    ));

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// Acceptance criterion 2 (design §11, handoff §6.2): every stage whose own execution genuinely
/// requires `WalReplay`'s output is `NotEvaluated`, naming `WalReplay` as the blocker -- not
/// silently absent, not defaulted to `Evaluated`. Covers all six real dependents from Step 0's
/// corrected graph (`stage-2-level-1-step0-ruling-v1.md` §1): `WalPersistence`, `RollbackDrafts`,
/// `WalRecordSchema`, `ActiveWalMetadata`, `PublicationReclassification`, `WalOrdering`.
#[test]
fn verify_repository_marks_every_wal_replay_dependent_as_not_evaluated() -> Result<()> {
    let root = unique_temp_dir("stage2-wal-replay-dependents");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    objects.write_object(&signed_patch_blob_envelope())?;
    let patch = signed_patch_envelope();
    let wal = Wal::for_layout(&layout);
    wal.append_patch(&patch)?;
    let mut bytes = std::fs::read(wal.path())?;
    let last_byte = bytes
        .last_mut()
        .ok_or_else(|| PrikkError::Io("WAL file unexpectedly empty".to_string()))?;
    *last_byte ^= 0x01;
    std::fs::write(wal.path(), &bytes)?;

    let report = verify_repository(&layout)?;
    assert!(matches!(
        find_stage(&report.stage_outcomes, VerificationStage::WalReplay).status,
        StageStatus::Failed { .. }
    ));
    for dependent in [
        VerificationStage::WalPersistence,
        VerificationStage::RollbackDrafts,
        VerificationStage::WalRecordSchema,
        VerificationStage::ActiveWalMetadata,
        VerificationStage::PublicationReclassification,
        VerificationStage::WalOrdering,
    ] {
        let outcome = find_stage(&report.stage_outcomes, dependent);
        match &outcome.status {
            StageStatus::NotEvaluated { blocked_by } => {
                assert_eq!(
                    *blocked_by,
                    VerificationStage::WalReplay,
                    "stage {dependent} should name WalReplay as its blocker"
                );
            }
            other => panic!("expected stage {dependent} to be NotEvaluated, got: {other:?}"),
        }
    }
    // Objects, Refs, RefUpdateSchemaTrust, CommitIndex, LifecycleCache have no dependency on
    // WalReplay and must still evaluate.
    for independent in [
        VerificationStage::Objects,
        VerificationStage::Refs,
        VerificationStage::RefUpdateSchemaTrust,
        VerificationStage::CommitIndex,
        VerificationStage::LifecycleCache,
    ] {
        assert!(
            matches!(
                find_stage(&report.stage_outcomes, independent).status,
                StageStatus::Evaluated
            ),
            "stage {independent} does not depend on WalReplay and should still evaluate"
        );
    }

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// The correctness fix the Step 0 ruling required (`stage-2-level-1-step0-ruling-v1.md` §2-§3), the
/// reason Level 1's implementation exists to get right: an accumulator's emptiness proves "none
/// found" only if its producer ran to completion. `trust_verifier.issues.is_empty()` alone would
/// read `true` when `Objects` failed *before* checking a single Block or RefState -- not because
/// trust was validated, but because it was never attempted. `require_retained_evidence` must not
/// treat that as proof.
///
/// Construction: the `LEGACY-LOG-LEADS` retained-evidence shape from DC-95 Stage 1 round 10 (real
/// WAL/active-metadata evidence a Block's `patch_ids` genuinely match) -- normally enough for
/// `require_retained_evidence` to leave the issue as `LEGACY-LOG-LEADS` rather than reclassifying
/// it. Then, separately, an object-id-mismatched Blob is planted under the `Blob` object-type
/// directory. `persisted_object_types` (`layout.rs`) scans `Patch`, `Block`, `RefState` before
/// `Blob` -- so this defect does *not* interrupt the Block/RefState trust checks this fixture's own
/// retained-evidence proof depends on, and both stay real. What this test isolates is different:
/// with `Objects` itself `Failed` (for a reason unrelated to trust), the fix requires
/// `trust_is_valid` to read `false` regardless of what `trust_verifier.issues` contains, because
/// the objects stage's own failure means "no stage guarantees every relevant object was checked" --
/// not "every check that ran found nothing." The issue must still be reclassified to `DIVERGENCE`.
#[test]
fn verify_repository_does_not_treat_an_incomplete_objects_stage_as_proof_of_trust() -> Result<()> {
    let root = unique_temp_dir("stage2-trust-is-valid-fix");
    let layout = RepositoryLayout::init(root.clone())?;
    let signer = trusted_signer("stage2-trust-is-valid-fix", 0x18)?;
    adopt(&layout, &signer)?;

    let patch = signed_patch_envelope();
    let patch_id = patch.object_id();
    let mut objects = FileObjectStore::new(layout.clone());
    objects.write_object(&signed_patch_blob_envelope())?;
    objects.write_object(&patch)?;
    write_active_ref_metadata(&layout, "heads/main")?;
    Wal::for_layout(&layout).append_patch(&patch)?;

    let block_payload = BlockPayload {
        parent_block_ids: Vec::new(),
        kind: BlockKind::Root,
        patch_ids: vec![patch_id],
        state_merkle_root: MerkleRoot([0; 32]),
        snapshot_blob_ref: None,
        mainline_parent_id: None,
        merge_baseline_block_id: None,
    };
    let mut block_envelope =
        ObjectEnvelope::unsigned(ObjectType::Block, 1, block_payload.to_canonical_bytes()?);
    let block_id = block_envelope.object_id();
    block_envelope.add_signature(maintainer_signature(&signer, ObjectType::Block, block_id)?)?;
    let block_path = layout.object_path(ObjectType::Block, block_id);
    std::fs::create_dir_all(
        block_path
            .parent()
            .ok_or_else(|| PrikkError::Io("legacy Block path has no parent".to_string()))?,
    )?;
    std::fs::write(
        &block_path,
        crate::file_codec::encode_envelope_file(&block_envelope)?,
    )?;

    let state1 = RefStatePayload {
        ref_name: "heads/main".to_string(),
        kind: RefKind::Branch,
        target_object_id: block_id,
        update_seq: 1,
        previous_ref_state_id: None,
        required_attestation_ids: Vec::new(),
        closed: false,
    };
    let mut ref_state1 =
        ObjectEnvelope::unsigned(ObjectType::RefState, 1, state1.to_canonical_bytes()?);
    let state1_id = ref_state1.object_id();
    ref_state1.add_signature(maintainer_signature(
        &signer,
        ObjectType::RefState,
        state1_id,
    )?)?;
    let update1 = build_signed_ref_update("heads/main", None, state1_id, block_id, 1, &signer)?;
    RefStore::new(layout.clone()).publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_state: ref_state1,
        ref_update: update1,
    })?;
    let pointer_after_first = std::fs::read(layout.ref_pointer_path("heads/main"))?;

    let state2 = RefStatePayload {
        ref_name: "heads/main".to_string(),
        kind: RefKind::Branch,
        target_object_id: block_id,
        update_seq: 2,
        previous_ref_state_id: Some(state1_id),
        required_attestation_ids: Vec::new(),
        closed: false,
    };
    let mut ref_state2 =
        ObjectEnvelope::unsigned(ObjectType::RefState, 1, state2.to_canonical_bytes()?);
    let state2_id = ref_state2.object_id();
    ref_state2.add_signature(maintainer_signature(
        &signer,
        ObjectType::RefState,
        state2_id,
    )?)?;
    let update2 = build_signed_ref_update(
        "heads/main",
        Some(state1_id),
        state2_id,
        block_id,
        2,
        &signer,
    )?;
    RefStore::new(layout.clone()).publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: Some(state1_id),
        ref_state: ref_state2,
        ref_update: update2,
    })?;

    std::fs::write(layout.ref_pointer_path("heads/main"), &pointer_after_first)?;
    std::fs::write(layout.format_path(), b"1\n")?;

    // Separately: an object-id-mismatched Blob, scanned after Patch/Block/RefState per
    // `persisted_object_types`'s own order, so it does not interrupt this fixture's own trust
    // checks -- it only makes the whole `Objects` stage `Failed` once those checks are done.
    let stray_blob_payload =
        prikk_object::BlobPayload::new(prikk_object::BlobKind::Text, b"unrelated".to_vec());
    let mut stray_blob = ObjectEnvelope::unsigned(
        ObjectType::Blob,
        1,
        stray_blob_payload.to_canonical_bytes()?,
    );
    stray_blob.add_signature(maintainer_signature(
        &signer,
        ObjectType::Blob,
        stray_blob.object_id(),
    )?)?;
    let wrong_id = sample_object_id("stage2-trust-fix-wrong-id");
    let misplaced = layout.object_path(ObjectType::Blob, wrong_id);
    std::fs::create_dir_all(
        misplaced
            .parent()
            .ok_or_else(|| PrikkError::Io("misplaced blob path has no parent".to_string()))?,
    )?;
    std::fs::write(
        &misplaced,
        crate::file_codec::encode_envelope_file(&stray_blob)?,
    )?;

    let legacy_layout = RepositoryLayout::open(root.clone())?;
    let report = verify_repository(&legacy_layout)?;

    assert!(matches!(
        find_stage(&report.stage_outcomes, VerificationStage::Objects).status,
        StageStatus::Failed { .. }
    ));
    // PublicationReclassification does not hard-depend on Objects (only on Refs/WalReplay/
    // ActiveWalMetadata) -- it still runs, but with trust_is_valid forced false.
    assert!(matches!(
        find_stage(
            &report.stage_outcomes,
            VerificationStage::PublicationReclassification
        )
        .status,
        StageStatus::Evaluated
    ));
    assert!(
        report
            .ref_publication_issues
            .iter()
            .any(|issue| issue.code == "PRIKK-VERIFY-REF-DIVERGENCE" && issue.blocking),
        "expected the issue reclassified to DIVERGENCE (trust not provably valid), got: {:?}",
        report.ref_publication_issues
    );
    assert!(
        !report
            .ref_publication_issues
            .iter()
            .any(|issue| issue.code == "PRIKK-VERIFY-REF-LEGACY-LOG-LEADS"),
        "the specific code must not survive when Objects never proved trust: {:?}",
        report.ref_publication_issues
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// `--stop-on-first-error` (`VerifyOptions::stop_on_first_error`), preserving today's bounded walk:
/// once the *first* stage in pipeline order fails, every later stage becomes blocking. Same fixture as
/// `verify_repository_reports_two_independent_stage_failures_together` (an id-mismatched Blob in
/// `Objects`, a checksum-corrupted active WAL record in `WalReplay`), which proves the *opposite*
/// under default options: both `Failed` independently. Here, with the flag set, `Objects` runs first,
/// fails, and the halt suppresses `WalReplay` from ever attempting its own (also genuinely broken)
/// check.
///
/// The required fix from `stage-2-level-1-implementation-review-v1.md` §4: a stage preempted by the
/// halt is `Halted { after: Objects }`, not `NotEvaluated { blocked_by: Objects }` -- `Objects` is not
/// a real dependency of, say, `LifecycleCache`, so naming it as a `blocked_by` would assert a
/// dependency-graph edge that does not exist. Stages with a *real* dependency on something other than
/// `Objects` still report that true dependency via `NotEvaluated`, even though the walk is halted: e.g.
/// `WalPersistence` genuinely depends on `WalReplay`, and `WalReplay` itself never evaluated (it was
/// pre-empted, `Halted { after: Objects }`) -- so `WalPersistence` is `NotEvaluated { blocked_by:
/// WalReplay }`, an accurate claim regardless of *why* `WalReplay` didn't evaluate, discoverable by
/// following the chain one stage at a time rather than by this stage reaching past its own dependency.
#[test]
fn verify_repository_with_options_halts_every_later_stage_when_stop_on_first_error_is_set()
-> Result<()> {
    let root = unique_temp_dir("stage2-stop-on-first-error");
    let layout = RepositoryLayout::init(root.clone())?;

    // Objects: an id-mismatched Blob, same technique as the two-independent-failures test above.
    let blob = prikk_object::BlobPayload::new(prikk_object::BlobKind::Text, b"payload".to_vec());
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, blob.to_canonical_bytes()?);
    envelope.add_signature(fake_maintainer_signature())?;
    let wrong_id = sample_object_id("stage2-stop-wrong-id");
    let misplaced = layout.object_path(ObjectType::Blob, wrong_id);
    std::fs::create_dir_all(
        misplaced
            .parent()
            .ok_or_else(|| PrikkError::Io("misplaced object path has no parent".to_string()))?,
    )?;
    std::fs::write(
        &misplaced,
        crate::file_codec::encode_envelope_file(&envelope)?,
    )?;

    // WalReplay: a real, well-formed patch, then a corrupted checksum byte -- independently broken,
    // same as the two-independent-failures test, so this stage would fail on its own if it ran.
    let mut objects = FileObjectStore::new(layout.clone());
    objects.write_object(&signed_patch_blob_envelope())?;
    let patch = signed_patch_envelope();
    let wal = Wal::for_layout(&layout);
    wal.append_patch(&patch)?;
    let mut bytes = std::fs::read(wal.path())?;
    let last_byte = bytes
        .last_mut()
        .ok_or_else(|| PrikkError::Io("WAL file unexpectedly empty".to_string()))?;
    *last_byte ^= 0x01;
    std::fs::write(wal.path(), &bytes)?;

    let report = verify_repository_with_options(
        &layout,
        VerifyOptions {
            stop_on_first_error: true,
        },
    )?;
    assert!(report.has_stage_failure());
    assert!(matches!(
        find_stage(&report.stage_outcomes, VerificationStage::Objects).status,
        StageStatus::Failed { .. }
    ));
    // Stages with no real dependency of their own were simply pre-empted by the halt -- Halted, naming
    // Objects as the stage that triggered the stop, not a fabricated dependency.
    for halted in [
        VerificationStage::Refs,
        VerificationStage::WalReplay,
        VerificationStage::CommitIndex,
        VerificationStage::LifecycleCache,
    ] {
        let outcome = find_stage(&report.stage_outcomes, halted);
        match &outcome.status {
            StageStatus::Halted { after } => {
                assert_eq!(
                    *after,
                    VerificationStage::Objects,
                    "stage {halted} should name Objects as the stage that halted the walk"
                );
            }
            other => panic!("expected stage {halted} to be Halted, got: {other:?}"),
        }
    }
    // Stages with a real dependency still name that true dependency, even though it was itself
    // pre-empted rather than genuinely failed -- the claim "I could not run because WalReplay did not
    // evaluate" remains true regardless of why WalReplay didn't evaluate.
    for (dependent, dependency) in [
        (
            VerificationStage::RefUpdateSchemaTrust,
            VerificationStage::Refs,
        ),
        (
            VerificationStage::WalPersistence,
            VerificationStage::WalReplay,
        ),
        (
            VerificationStage::RollbackDrafts,
            VerificationStage::WalReplay,
        ),
        (
            VerificationStage::WalRecordSchema,
            VerificationStage::WalReplay,
        ),
        (
            VerificationStage::ActiveWalMetadata,
            VerificationStage::WalReplay,
        ),
        (
            VerificationStage::PublicationReclassification,
            VerificationStage::WalReplay,
        ),
        (VerificationStage::WalOrdering, VerificationStage::WalReplay),
    ] {
        let outcome = find_stage(&report.stage_outcomes, dependent);
        match &outcome.status {
            StageStatus::NotEvaluated { blocked_by } => {
                assert_eq!(
                    *blocked_by, dependency,
                    "stage {dependent} should name its real dependency {dependency}, not the stage that halted the walk"
                );
            }
            other => panic!("expected stage {dependent} to be NotEvaluated, got: {other:?}"),
        }
    }

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
