//! Rollback draft verification tests.

use prikk_object::PatchPurpose;

use crate::{
    Ed25519AuthorSigner, FileObjectStore, ObjectWriter, RepositoryLayout, Wal,
    append_rollback_draft, verify_active_rollback_draft, verify_repository,
};

use crate::test_support::publish_snapshot_then_patch_block;
use crate::test_support::{
    rollback_patch_envelope, signed_block, signed_patch_envelope, signed_ref_state_envelope,
    signed_ref_update_envelope, unique_temp_dir,
};
use crate::{RefPublication, RefStore};

fn test_signer() -> Ed25519AuthorSigner {
    Ed25519AuthorSigner::from_seed("rollback-author-key", &[9_u8; 32])
}

#[test]
fn rollback_draft_verify_matches_current_inverse_plan() {
    let root = unique_temp_dir("rollback-draft-verify");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let published = publish_snapshot_then_patch_block(&layout);
        assert!(published.is_ok());
        let signer = test_signer();
        let draft = append_rollback_draft(&layout, "heads/main", "rollback verify", &signer);
        assert!(draft.is_ok());
        let verification = verify_active_rollback_draft(&layout, "heads/main");
        assert!(verification.is_ok());
        if let Ok(verification) = verification {
            assert_eq!(verification.ref_name, "heads/main");
            assert_eq!(verification.wal_sequence, 1);
            assert_eq!(verification.author_key_id, "rollback-author-key");
            assert_eq!(verification.inverse_operation_count, 2);
            assert_eq!(verification.decoded_operation_count, 2);
        }
        let repository = verify_repository(&layout);
        assert!(repository.is_ok());
        if let Ok(repository) = repository {
            assert_eq!(repository.checked_rollback_draft_records, 1);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rollback_draft_verify_refuses_plain_active_patch() {
    let root = unique_temp_dir("rollback-draft-verify-plain");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let published = publish_snapshot_then_patch_block(&layout);
        assert!(published.is_ok());
        let wal = Wal::new(layout.default_queue_wal_path());
        let append = wal.append_patch(&signed_patch_envelope());
        assert!(append.is_ok());
        let verification = verify_active_rollback_draft(&layout, "heads/main");
        assert!(verification.is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn sealed_history_classifies_payload_purpose_not_legacy_key_id() {
    let root = unique_temp_dir("rollback-draft-purpose-classification");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let rollback_patch = rollback_patch_envelope();
        assert_eq!(
            PatchPurpose::decode_from_patch_payload(&rollback_patch.canonical_payload),
            Ok(PatchPurpose::RollbackDraft)
        );
        let rollback_patch_id = object_store.write_object(&rollback_patch);
        assert!(rollback_patch_id.is_ok());
        if let Ok(rollback_patch_id) = rollback_patch_id {
            let block = signed_block(
                prikk_object::BlockKind::Root,
                Vec::new(),
                vec![rollback_patch_id],
                None,
            );
            let block_id = object_store.write_object(&block);
            assert!(block_id.is_ok());
            if let Ok(block_id) = block_id {
                let ref_store = RefStore::new(layout.clone());
                let ref_state = signed_ref_state_envelope("heads/main", None, block_id, 1);
                let ref_state_id = ref_state.object_id();
                let ref_update =
                    signed_ref_update_envelope("heads/main", None, ref_state_id, block_id, 1);
                let published = ref_store.publish(&RefPublication {
                    ref_name: "heads/main".to_string(),
                    expected_previous_ref_state_id: None,
                    ref_state,
                    ref_update,
                });
                assert!(published.is_ok());

                let verification = verify_repository(&layout);
                assert!(verification.is_ok());
                if let Ok(verification) = verification {
                    assert_eq!(verification.checked_rollback_blocks, 1);
                    assert_eq!(verification.checked_sealed_rollback_patches, 1);
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn legacy_key_id_without_payload_purpose_is_not_classified() {
    let root = unique_temp_dir("rollback-draft-legacy-clean-break");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let mut legacy = signed_patch_envelope();
        legacy.signatures.clear();
        assert!(
            legacy
                .add_signature(crate::test_support::legacy_rollback_marker_signature())
                .is_ok()
        );
        assert_eq!(
            PatchPurpose::decode_from_patch_payload(&legacy.canonical_payload),
            Ok(PatchPurpose::Normal)
        );
        let legacy_patch_id = object_store.write_object(&legacy);
        assert!(legacy_patch_id.is_ok());
        if let Ok(legacy_patch_id) = legacy_patch_id {
            let block = signed_block(
                prikk_object::BlockKind::Root,
                Vec::new(),
                vec![legacy_patch_id],
                None,
            );
            let block_id = object_store.write_object(&block);
            assert!(block_id.is_ok());
            if let Ok(block_id) = block_id {
                let ref_store = RefStore::new(layout.clone());
                let ref_state = signed_ref_state_envelope("heads/main", None, block_id, 1);
                let ref_state_id = ref_state.object_id();
                let ref_update =
                    signed_ref_update_envelope("heads/main", None, ref_state_id, block_id, 1);
                let published = ref_store.publish(&RefPublication {
                    ref_name: "heads/main".to_string(),
                    expected_previous_ref_state_id: None,
                    ref_state,
                    ref_update,
                });
                assert!(published.is_ok());
                let verification = verify_repository(&layout);
                assert!(verification.is_ok());
                if let Ok(verification) = verification {
                    assert_eq!(verification.checked_rollback_blocks, 0);
                    assert_eq!(verification.checked_sealed_rollback_patches, 0);
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
}
