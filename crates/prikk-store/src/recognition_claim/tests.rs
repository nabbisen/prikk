//! RFC 115 Stage 2 tests for `check_recognition_claim_consistency`. §7 rows 3 and 4.

use prikk_object::{BlockKind, ObjectId, RecognitionClaimPayload};

use super::{RecognitionClaimConsistency, check_recognition_claim_consistency};
use crate::test_support::{signed_block, unique_temp_dir};
use crate::trust::load_maintainer_trust_policy;
use crate::{FileObjectStore, ObjectWriter, RepositoryLayout};

/// §7 row 4: a claim about a block the receiver does not hold is *accepted*, reading `BlockAbsent`
/// -- not an error, and not something that reads as a degraded case. This is the expected shape of
/// real exchange (design §3), not an edge case.
#[test]
fn claim_about_an_absent_block_reads_block_absent() -> prikk_error::Result<()> {
    let root = unique_temp_dir("rfc115-recognition-claim-absent");
    let layout = RepositoryLayout::init(root.clone())?;
    let store = FileObjectStore::new(layout);
    let claim = RecognitionClaimPayload {
        block_id: ObjectId::from_bytes([0x71; 32]),
        patch_ids: vec![ObjectId::from_bytes([0x72; 32])],
    };
    let outcome = check_recognition_claim_consistency(&store, &claim)?;
    assert_eq!(outcome, RecognitionClaimConsistency::BlockAbsent);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// §7 row 3, and design §3's own required refinement: a claim contradicting a block the receiver
/// *does* hold is a detected lie, reported with both sides named.
#[test]
fn claim_contradicting_a_held_block_is_contradicted() -> prikk_error::Result<()> {
    let root = unique_temp_dir("rfc115-recognition-claim-contradicted");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut store = FileObjectStore::new(layout);
    let real_patch = ObjectId::from_bytes([0x73; 32]);
    let block = signed_block(BlockKind::Normal, Vec::new(), vec![real_patch], None);
    let block_id = store.write_object(&block)?;

    let lying_patch = ObjectId::from_bytes([0x74; 32]);
    let claim = RecognitionClaimPayload {
        block_id,
        patch_ids: vec![lying_patch],
    };
    let outcome = check_recognition_claim_consistency(&store, &claim)?;
    assert_eq!(
        outcome,
        RecognitionClaimConsistency::Contradicted {
            claimed: vec![lying_patch],
            actual: vec![real_patch],
        }
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// The ordinary case: a claim whose `patch_ids` match a held block's own, reads `Consistent` --
/// and this must hold even though the block's own `patch_ids` are in authoring/WAL-replay order,
/// not sorted, unlike the claim's (see the module doc's ordering note). Two patches, sealed into
/// the block in *descending* id order, checked against a claim that lists them ascending (as every
/// `RecognitionClaimPayload` must).
#[test]
fn claim_matching_a_held_block_is_consistent_regardless_of_the_blocks_own_patch_order()
-> prikk_error::Result<()> {
    let root = unique_temp_dir("rfc115-recognition-claim-consistent");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut store = FileObjectStore::new(layout);
    let low = ObjectId::from_bytes([0x75; 32]);
    let high = ObjectId::from_bytes([0x76; 32]);
    // Block's own patch_ids deliberately descending -- authoring order, not sorted.
    let block = signed_block(BlockKind::Normal, Vec::new(), vec![high, low], None);
    let block_id = store.write_object(&block)?;

    let claim = RecognitionClaimPayload {
        block_id,
        patch_ids: vec![low, high],
    };
    let outcome = check_recognition_claim_consistency(&store, &claim)?;
    assert_eq!(outcome, RecognitionClaimConsistency::Consistent);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// §7 row 7: trust never expands across a claim check -- neither an absent-block claim, nor a
/// contradicted one, nor a consistent one may change the repository's adopted-key set. Checked
/// directly against `load_maintainer_trust_policy`, before and after each of the three outcomes.
#[test]
fn checking_a_recognition_claim_never_changes_the_adopted_key_set() -> prikk_error::Result<()> {
    let root = unique_temp_dir("rfc115-recognition-claim-trust-inert");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut store = FileObjectStore::new(layout.clone());

    // A receiver with an already-adopted maintainer -- the realistic case, and the one where an
    // unwanted expansion (a second key silently appearing) would actually be observable.
    crate::trust::add_trusted_maintainer(
        &layout,
        "rfc115-trust-inert-maintainer",
        "1111111111111111111111111111111111111111111111111111111111111111",
    )?;

    let patch = ObjectId::from_bytes([0x77; 32]);
    let block = signed_block(BlockKind::Normal, Vec::new(), vec![patch], None);
    let block_id = store.write_object(&block)?;

    let before = load_maintainer_trust_policy(&layout)?;

    let absent_claim = RecognitionClaimPayload {
        block_id: ObjectId::from_bytes([0x78; 32]),
        patch_ids: vec![patch],
    };
    check_recognition_claim_consistency(&store, &absent_claim)?;
    assert_eq!(load_maintainer_trust_policy(&layout)?, before);

    let contradicted_claim = RecognitionClaimPayload {
        block_id,
        patch_ids: vec![ObjectId::from_bytes([0x79; 32])],
    };
    check_recognition_claim_consistency(&store, &contradicted_claim)?;
    assert_eq!(load_maintainer_trust_policy(&layout)?, before);

    let consistent_claim = RecognitionClaimPayload {
        block_id,
        patch_ids: vec![patch],
    };
    check_recognition_claim_consistency(&store, &consistent_claim)?;
    assert_eq!(load_maintainer_trust_policy(&layout)?, before);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
