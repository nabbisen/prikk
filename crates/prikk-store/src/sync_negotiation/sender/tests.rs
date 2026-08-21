//! RFC 116 stage 3 handoff §5: rows 1-6, plus §4's absence-is-not-a-refusal pair carried forward
//! from stage 2's review. Each row's control mutates the single narrowest line that should break
//! the property under test.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use prikk_error::Result;

use super::sender_test_support::{
    adopt, author_signer, cleanup, create_file_patch, maintainer_signer, repo, seal_patches_onto,
    write_blob,
};
use super::{SyncArtifactOutcome, build_sync_artifact};
use crate::sync_negotiation::{
    DEFAULT_HAVE_LIST_MAX_PATCH_COUNT, DEFAULT_HAVE_LIST_MAX_TOTAL_BYTES, build_have_list,
    decode_have_list,
};
use crate::{AcceptOptions, FileObjectStore, accept_exchange_artifact, seal_from_accepted_claim};

const TARGET_REF: &str = "heads/main";

/// §4: a ref the sender does not hold reports as already in sync, not a refusal.
#[test]
fn a_sender_absent_ref_reports_already_in_sync_not_a_refusal() -> Result<()> {
    let sender = repo("sender-stage3-absent-sender")?;
    let signer = maintainer_signer(0x10)?;
    adopt(&sender, &signer)?;

    let receiver = repo("sender-stage3-absent-sender-receiver")?;
    // The receiver's own have-list content is immaterial here -- the sender has nothing under
    // this ref at all, so the delta must be empty regardless of what the have-list says.
    let have_list_bytes = build_have_list(&receiver, TARGET_REF)?;

    let outcome = build_sync_artifact(&sender, TARGET_REF, &have_list_bytes, &signer)?;
    match outcome {
        SyncArtifactOutcome::AlreadyInSync { ref_name } => assert_eq!(ref_name, TARGET_REF),
        other => panic!("expected AlreadyInSync, got {other:?}"),
    }
    cleanup(&sender);
    cleanup(&receiver);
    Ok(())
}

/// §4: a have-list naming a ref the receiver does not hold (an empty list) produces the sender's
/// full reachable set as the delta, not a refusal.
#[test]
fn a_receiver_absent_ref_produces_the_full_reachable_set_as_the_delta() -> Result<()> {
    let sender = repo("sender-stage3-absent-receiver")?;
    let signer = maintainer_signer(0x11)?;
    adopt(&sender, &signer)?;
    let author = author_signer(0x12)?;
    let mut objects = FileObjectStore::new(sender.clone());
    let blob_a = write_blob(&mut objects, b"stage3 a\n")?;
    let patch_a = create_file_patch(&author, "a.txt", 0x13, blob_a)?;
    let blob_b = write_blob(&mut objects, b"stage3 b\n")?;
    let patch_b = create_file_patch(&author, "b.txt", 0x14, blob_b)?;
    seal_patches_onto(
        &sender,
        TARGET_REF,
        &[patch_a.clone(), patch_b.clone()],
        &signer,
    )?;

    let receiver = repo("sender-stage3-absent-receiver-side")?;
    let have_list_bytes = build_have_list(&receiver, TARGET_REF)?; // receiver holds nothing

    let outcome = build_sync_artifact(&sender, TARGET_REF, &have_list_bytes, &signer)?;
    match outcome {
        SyncArtifactOutcome::Artifact { report, .. } => {
            assert_eq!(report.delta_patch_count, 2);
            assert_eq!(report.export_report.patch_count, 2);
        }
        other => panic!("expected Artifact, got {other:?}"),
    }
    cleanup(&sender);
    cleanup(&receiver);
    Ok(())
}

/// An empty delta because the receiver already has everything is also reported as already in
/// sync, not merely the sender-absent or receiver-absent shapes above.
#[test]
fn an_already_synced_ref_reports_already_in_sync() -> Result<()> {
    let sender = repo("sender-stage3-already-synced-sender")?;
    let sender_signer = maintainer_signer(0x15)?;
    adopt(&sender, &sender_signer)?;
    let author = author_signer(0x16)?;
    let mut sender_objects = FileObjectStore::new(sender.clone());
    let blob = write_blob(&mut sender_objects, b"stage3 synced\n")?;
    let patch = create_file_patch(&author, "synced.txt", 0x17, blob)?;
    seal_patches_onto(
        &sender,
        TARGET_REF,
        std::slice::from_ref(&patch),
        &sender_signer,
    )?;

    let receiver = repo("sender-stage3-already-synced-receiver")?;
    let receiver_signer = maintainer_signer(0x18)?;
    adopt(&receiver, &receiver_signer)?;
    let mut receiver_objects = FileObjectStore::new(receiver.clone());
    // Same content, independently written -- content-addressed, so this produces the same blob
    // and patch ids as the sender's own.
    let receiver_blob = write_blob(&mut receiver_objects, b"stage3 synced\n")?;
    assert_eq!(receiver_blob, blob);
    let receiver_patch = create_file_patch(&author, "synced.txt", 0x17, receiver_blob)?;
    assert_eq!(receiver_patch.object_id(), patch.object_id());
    seal_patches_onto(&receiver, TARGET_REF, &[receiver_patch], &receiver_signer)?;

    let have_list_bytes = build_have_list(&receiver, TARGET_REF)?;
    let outcome = build_sync_artifact(&sender, TARGET_REF, &have_list_bytes, &sender_signer)?;
    assert!(matches!(outcome, SyncArtifactOutcome::AlreadyInSync { .. }));
    cleanup(&sender);
    cleanup(&receiver);
    Ok(())
}

/// §5 row 1: the artifact carries exactly the delta, not the full reachable set -- the receiver
/// already holds `patch_a` (sealed on its own `heads/main`), so only `patch_b` is new.
#[test]
fn the_artifact_carries_exactly_the_delta_not_the_full_reachable_set() -> Result<()> {
    let sender = repo("sender-stage3-row1-sender")?;
    let sender_signer = maintainer_signer(0x20)?;
    adopt(&sender, &sender_signer)?;
    let author = author_signer(0x21)?;
    let mut sender_objects = FileObjectStore::new(sender.clone());
    let blob_a = write_blob(&mut sender_objects, b"row1 a\n")?;
    let patch_a = create_file_patch(&author, "row1-a.txt", 0x22, blob_a)?;
    let blob_b = write_blob(&mut sender_objects, b"row1 b\n")?;
    let patch_b = create_file_patch(&author, "row1-b.txt", 0x23, blob_b)?;
    seal_patches_onto(
        &sender,
        TARGET_REF,
        &[patch_a.clone(), patch_b.clone()],
        &sender_signer,
    )?;

    let receiver = repo("sender-stage3-row1-receiver")?;
    let receiver_signer = maintainer_signer(0x24)?;
    adopt(&receiver, &receiver_signer)?;
    let mut receiver_objects = FileObjectStore::new(receiver.clone());
    let receiver_blob_a = write_blob(&mut receiver_objects, b"row1 a\n")?;
    let receiver_patch_a = create_file_patch(&author, "row1-a.txt", 0x22, receiver_blob_a)?;
    assert_eq!(receiver_patch_a.object_id(), patch_a.object_id());
    seal_patches_onto(&receiver, TARGET_REF, &[receiver_patch_a], &receiver_signer)?;

    let have_list_bytes = build_have_list(&receiver, TARGET_REF)?;
    let outcome = build_sync_artifact(&sender, TARGET_REF, &have_list_bytes, &sender_signer)?;
    match outcome {
        SyncArtifactOutcome::Artifact { report, .. } => {
            assert_eq!(
                report.delta_patch_count, 1,
                "only patch_b is missing from the receiver"
            );
            assert_eq!(
                report.export_report.patch_count, 1,
                "the artifact's own patches section must carry only the delta, not the full \
                 two-patch block"
            );
        }
        other => panic!("expected Artifact, got {other:?}"),
    }
    cleanup(&sender);
    cleanup(&receiver);
    Ok(())
}

/// §5 row 2: the claim carries the block's full, verbatim `patch_ids` -- proven by a delta that is
/// a **proper subset** of the claimed block's own patches, so a trimmed claim would actually
/// differ from a full one (a delta that happens to equal the whole block, as in a simpler fixture,
/// would make a trim-to-delta mutation invisible). The receiver already holds `patch_a` on its own
/// `heads/main` (so the delta for `heads/main` is `{patch_b}` only) **and** already holds the
/// *exact same two-patch block* -- by id -- under a decoy ref, so
/// `accept_exchange_artifact`'s own `check_recognition_claim_consistency` call fires against it.
/// A verbatim claim (`patch_ids = [patch_a, patch_b]`) reads `Consistent` and accept succeeds; a
/// claim trimmed to the delta (`patch_ids = [patch_b]`) would read `Contradicted` and accept would
/// refuse the whole exchange.
#[test]
fn claims_carry_the_blocks_full_verbatim_patch_ids_and_parent_block_ids() -> Result<()> {
    let sender = repo("sender-stage3-row2-sender")?;
    let sender_signer = maintainer_signer(0x30)?;
    adopt(&sender, &sender_signer)?;
    let author = author_signer(0x31)?;
    let mut sender_objects = FileObjectStore::new(sender.clone());
    let blob_a = write_blob(&mut sender_objects, b"row2 a\n")?;
    let patch_a = create_file_patch(&author, "row2-a.txt", 0x32, blob_a)?;
    let blob_b = write_blob(&mut sender_objects, b"row2 b\n")?;
    let patch_b = create_file_patch(&author, "row2-b.txt", 0x33, blob_b)?;
    seal_patches_onto(
        &sender,
        TARGET_REF,
        &[patch_a.clone(), patch_b.clone()],
        &sender_signer,
    )?;

    let receiver = repo("sender-stage3-row2-receiver")?;
    let receiver_signer = maintainer_signer(0x34)?;
    adopt(&receiver, &receiver_signer)?;
    let mut receiver_objects = FileObjectStore::new(receiver.clone());
    let receiver_blob_a = write_blob(&mut receiver_objects, b"row2 a\n")?;
    let receiver_patch_a = create_file_patch(&author, "row2-a.txt", 0x32, receiver_blob_a)?;
    let receiver_blob_b = write_blob(&mut receiver_objects, b"row2 b\n")?;
    let receiver_patch_b = create_file_patch(&author, "row2-b.txt", 0x33, receiver_blob_b)?;
    assert_eq!(receiver_patch_a.object_id(), patch_a.object_id());
    assert_eq!(receiver_patch_b.object_id(), patch_b.object_id());
    // `patch_a` alone, sealed onto the receiver's own `heads/main` -- makes the delta for
    // `heads/main` exactly `{patch_b}`, a proper subset of the claimed block's own two patches.
    seal_patches_onto(
        &receiver,
        TARGET_REF,
        std::slice::from_ref(&receiver_patch_a),
        &receiver_signer,
    )?;
    // The identical two-patch block, verbatim, under a decoy ref -- so the receiver already holds
    // it as an object even though `heads/main` itself only has `patch_a`.
    seal_patches_onto(
        &receiver,
        "heads/decoy",
        &[receiver_patch_a, receiver_patch_b],
        &receiver_signer,
    )?;

    let have_list_bytes = build_have_list(&receiver, TARGET_REF)?; // heads/main has patch_a only
    let outcome = build_sync_artifact(&sender, TARGET_REF, &have_list_bytes, &sender_signer)?;
    let bytes = match outcome {
        SyncArtifactOutcome::Artifact { bytes, .. } => bytes,
        other => panic!("expected Artifact, got {other:?}"),
    };

    let accept_result =
        accept_exchange_artifact(&receiver, &bytes, &AcceptOptions::default_limits());
    let report = accept_result.expect(
        "a verbatim claim about a block the receiver already holds must read Consistent, not \
         Contradicted",
    );
    assert_eq!(report.claim_count, 1);
    cleanup(&sender);
    cleanup(&receiver);
    Ok(())
}

/// §5 row 3: an untrusted signer cannot produce an artifact.
#[test]
fn an_untrusted_signer_cannot_produce_an_artifact() -> Result<()> {
    let sender = repo("sender-stage3-row3")?;
    let signer = maintainer_signer(0x40)?;
    adopt(&sender, &signer)?;
    let author = author_signer(0x41)?;
    let mut objects = FileObjectStore::new(sender.clone());
    let blob = write_blob(&mut objects, b"row3\n")?;
    let patch = create_file_patch(&author, "row3.txt", 0x42, blob)?;
    seal_patches_onto(&sender, TARGET_REF, &[patch], &signer)?;

    let receiver = repo("sender-stage3-row3-receiver")?;
    let have_list_bytes = build_have_list(&receiver, TARGET_REF)?;

    let unadopted = maintainer_signer(0x43)?; // never adopted in `sender`
    let result = build_sync_artifact(&sender, TARGET_REF, &have_list_bytes, &unadopted);
    assert!(result.is_err(), "an unadopted signer must refuse to build");
    cleanup(&sender);
    cleanup(&receiver);
    Ok(())
}

/// §5 row 5: building an artifact adopts no key and changes no trust.
#[test]
fn building_an_artifact_adopts_no_key_and_changes_no_trust() -> Result<()> {
    let sender = repo("sender-stage3-row5")?;
    let signer = maintainer_signer(0x50)?;
    adopt(&sender, &signer)?;
    let author = author_signer(0x51)?;
    let mut objects = FileObjectStore::new(sender.clone());
    let blob = write_blob(&mut objects, b"row5\n")?;
    let patch = create_file_patch(&author, "row5.txt", 0x52, blob)?;
    seal_patches_onto(&sender, TARGET_REF, &[patch], &signer)?;

    let receiver = repo("sender-stage3-row5-receiver")?;
    let have_list_bytes = build_have_list(&receiver, TARGET_REF)?;

    let before = crate::trust::load_maintainer_trust_policy(&sender)?;
    let outcome = build_sync_artifact(&sender, TARGET_REF, &have_list_bytes, &signer)?;
    assert!(matches!(outcome, SyncArtifactOutcome::Artifact { .. }));
    let after = crate::trust::load_maintainer_trust_policy(&sender)?;
    assert_eq!(before, after, "the adopted-maintainer set must not change");
    cleanup(&sender);
    cleanup(&receiver);
    Ok(())
}

/// §5 row 6, the one that proves the loop closes: build on the sender, accept and seal on the
/// receiver, then assert the delta's patches are reachable from the receiver's own ref tip
/// afterward -- not merely that accept and seal returned `Ok`.
#[test]
fn round_trip_build_accept_seal_lands_the_delta() -> Result<()> {
    let sender = repo("sender-stage3-row6-sender")?;
    let sender_signer = maintainer_signer(0x60)?;
    adopt(&sender, &sender_signer)?;
    let author = author_signer(0x61)?;
    let mut sender_objects = FileObjectStore::new(sender.clone());
    let blob = write_blob(&mut sender_objects, b"row6\n")?;
    let patch = create_file_patch(&author, "row6.txt", 0x62, blob)?;
    seal_patches_onto(
        &sender,
        TARGET_REF,
        std::slice::from_ref(&patch),
        &sender_signer,
    )?;

    let receiver = repo("sender-stage3-row6-receiver")?;
    let receiver_signer = maintainer_signer(0x63)?;
    adopt(&receiver, &receiver_signer)?;
    let have_list_bytes = build_have_list(&receiver, TARGET_REF)?; // receiver holds nothing

    let outcome = build_sync_artifact(&sender, TARGET_REF, &have_list_bytes, &sender_signer)?;
    let bytes = match outcome {
        SyncArtifactOutcome::Artifact { bytes, .. } => bytes,
        other => panic!("expected Artifact, got {other:?}"),
    };

    let accept_report =
        accept_exchange_artifact(&receiver, &bytes, &AcceptOptions::default_limits())?;
    assert_eq!(accept_report.claim_count, 1);
    let (claim_id, _) = accept_report.claim_signature_outcomes[0];

    let seal_outcome = seal_from_accepted_claim(&receiver, TARGET_REF, claim_id, &receiver_signer)?;
    assert!(matches!(
        seal_outcome,
        crate::SealFromAcceptedOutcome::Sealed { .. }
    ));

    // The assertion that matters: read the receiver's own ref tip back and confirm the patch is
    // reachable from it -- not merely that accept/seal returned Ok.
    let receiver_have_list_bytes = build_have_list(&receiver, TARGET_REF)?;
    let receiver_have_list = decode_have_list(
        &receiver_have_list_bytes,
        DEFAULT_HAVE_LIST_MAX_TOTAL_BYTES,
        DEFAULT_HAVE_LIST_MAX_PATCH_COUNT,
    )?;
    assert_eq!(
        receiver_have_list.patch_ids,
        vec![patch.object_id()],
        "the synced patch must be reachable from the receiver's own ref tip"
    );
    cleanup(&sender);
    cleanup(&receiver);
    Ok(())
}

/// A have-list naming a different ref than the one requested is refused -- a defensive check
/// beyond §5's own numbered rows: `have_list_bytes` is untrusted wire data, and building an
/// artifact for the wrong ref under an innocent-looking mismatch is exactly the class of defect
/// this project checks for at every other untrusted-input boundary.
#[test]
fn a_have_list_naming_a_different_ref_is_refused() -> Result<()> {
    let sender = repo("sender-stage3-ref-mismatch")?;
    let signer = maintainer_signer(0x70)?;
    adopt(&sender, &signer)?;

    let receiver = repo("sender-stage3-ref-mismatch-receiver")?;
    let have_list_bytes = build_have_list(&receiver, "heads/other")?;

    let result = build_sync_artifact(&sender, TARGET_REF, &have_list_bytes, &signer);
    assert!(
        result.is_err(),
        "a have-list naming a different ref than requested must be refused"
    );
    cleanup(&sender);
    cleanup(&receiver);
    Ok(())
}
