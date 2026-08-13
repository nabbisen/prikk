//! History inspection tests.

use prikk_object::{
    BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope, ObjectType,
};

use crate::{
    FileObjectStore, ObjectWriter, RefPublication, RefStore, RepositoryLayout, load_ref_history,
    verify_repository,
};

use crate::test_support::{
    maintainer_signature, rollback_patch_blob_envelope, rollback_patch_envelope,
    signed_ref_state_envelope, signed_ref_update_envelope, unique_temp_dir,
};

#[test]
fn history_reports_published_ref_state_chain_newest_first() {
    let root = unique_temp_dir("history-chain");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let root_block = signed_block_envelope(BlockKind::Root, Vec::new(), Vec::new());
        let root_block_id = root_block.object_id();
        assert!(object_store.write_object(&root_block).is_ok());

        let ref_store = RefStore::new(layout.clone());
        let ref_state_1 = signed_ref_state_envelope("heads/main", None, root_block_id, 1);
        let ref_state_1_id = ref_state_1.object_id();
        let ref_update_1 =
            signed_ref_update_envelope("heads/main", None, ref_state_1_id, root_block_id, 1);
        let publication_1 = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state: ref_state_1,
            ref_update: ref_update_1,
        };
        assert!(ref_store.publish(&publication_1).is_ok());

        let normal_block =
            signed_block_envelope(BlockKind::Normal, vec![root_block_id], Vec::new());
        let normal_block_id = normal_block.object_id();
        assert!(object_store.write_object(&normal_block).is_ok());
        let ref_state_2 =
            signed_ref_state_envelope("heads/main", Some(ref_state_1_id), normal_block_id, 2);
        let ref_state_2_id = ref_state_2.object_id();
        let ref_update_2 = signed_ref_update_envelope(
            "heads/main",
            Some(ref_state_1_id),
            ref_state_2_id,
            normal_block_id,
            2,
        );
        let publication_2 = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: Some(ref_state_1_id),
            ref_state: ref_state_2,
            ref_update: ref_update_2,
        };
        assert!(ref_store.publish(&publication_2).is_ok());

        let history = load_ref_history(&layout, "heads/main", 20);
        assert!(history.is_ok());
        if let Ok(history) = history {
            assert_eq!(history.entries.len(), 2);
            assert_eq!(
                history.entries.first().map(|entry| entry.block_id),
                Some(normal_block_id)
            );
            assert_eq!(
                history.entries.first().map(|entry| entry.update_seq),
                Some(2)
            );
            assert_eq!(
                history.entries.get(1).map(|entry| entry.block_id),
                Some(root_block_id)
            );
            assert_eq!(
                history.entries.get(1).map(|entry| entry.block_kind),
                Some(BlockKind::Root)
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn history_respects_limit() {
    let root = unique_temp_dir("history-limit");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let block = signed_block_envelope(BlockKind::Root, Vec::new(), Vec::new());
        let block_id = block.object_id();
        assert!(object_store.write_object(&block).is_ok());
        let ref_store = RefStore::new(layout.clone());
        let ref_state = signed_ref_state_envelope("heads/main", None, block_id, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, block_id, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        assert!(ref_store.publish(&publication).is_ok());
        let history = load_ref_history(&layout, "heads/main", 0);
        assert!(history.is_ok());
        assert_eq!(history.ok().map(|value| value.entries.len()), Some(0));
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn history_and_verify_classify_sealed_rollback_block() {
    let root = unique_temp_dir("history-rollback-block");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let rollback_patch = rollback_patch_envelope();
        let rollback_patch_id = rollback_patch.object_id();
        assert!(
            object_store
                .write_object(&rollback_patch_blob_envelope())
                .is_ok()
        );
        assert!(object_store.write_object(&rollback_patch).is_ok());

        let root = crate::derive_next_state_root(&object_store, None, &[rollback_patch_id]);
        assert!(root.is_ok());
        let block = signed_block_envelope_with_root(
            BlockKind::Root,
            Vec::new(),
            vec![rollback_patch_id],
            root.unwrap_or(MerkleRoot([0; 32])),
        );
        let block_id = block.object_id();
        assert!(object_store.write_object(&block).is_ok());

        let ref_store = RefStore::new(layout.clone());
        let ref_state = signed_ref_state_envelope("heads/main", None, block_id, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, block_id, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        assert!(ref_store.publish(&publication).is_ok());

        let history = load_ref_history(&layout, "heads/main", 20);
        assert!(history.is_ok());
        if let Ok(history) = history {
            let entry = history.entries.first();
            assert!(entry.is_some());
            if let Some(entry) = entry {
                assert!(entry.is_rollback_block);
                assert_eq!(entry.rollback_patch_count, 1);
            }
        }

        let verification = verify_repository(&layout);
        assert!(verification.is_ok());
        if let Ok(verification) = verification {
            assert_eq!(verification.checked_rollback_blocks, Some(1));
            assert_eq!(verification.checked_sealed_rollback_patches, Some(1));
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

fn signed_block_envelope(
    kind: BlockKind,
    parent_block_ids: Vec<prikk_object::ObjectId>,
    patch_ids: Vec<prikk_object::ObjectId>,
) -> ObjectEnvelope {
    signed_block_envelope_with_root(
        kind,
        parent_block_ids,
        patch_ids,
        crate::compute_state_root(&[]).unwrap_or(MerkleRoot([0_u8; 32])),
    )
}

fn signed_block_envelope_with_root(
    kind: BlockKind,
    parent_block_ids: Vec<prikk_object::ObjectId>,
    patch_ids: Vec<prikk_object::ObjectId>,
    state_merkle_root: MerkleRoot,
) -> ObjectEnvelope {
    let payload = BlockPayload {
        parent_block_ids,
        kind,
        patch_ids,
        state_merkle_root,
        snapshot_blob_ref: None,
        mainline_parent_id: None,
        merge_baseline_block_id: None,
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::Block, 2, payload_bytes.unwrap_or_default());
    assert!(envelope.add_signature(maintainer_signature()).is_ok());
    envelope
}
