//! Patch replay planning tests.

use prikk_object::{
    BlobKind, BlobPayload, BlockKind, BlockPayload, CanonicalEncode, CreateFile, DeleteNode,
    DeleteNodePreimage, MerkleRoot, NodeId, NodeKind, ObjectEnvelope, ObjectType, Operation,
    OperationKind, PatchPayload,
};

use crate::{
    FileObjectStore, ObjectWriter, RefPublication, RefStore, RepoPath, RepositoryLayout,
    SnapshotEntry, SnapshotManifest, prepare_patch_replay_plan,
};

use super::helpers::{
    dummy_signature, maintainer_signature, signed_ref_state_envelope, signed_ref_update_envelope,
    unique_temp_dir,
};

#[test]
fn patch_replay_applies_create_delete_and_replace() {
    let root = unique_temp_dir("patch-replay");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let result = publish_snapshot_then_patch_block(&layout);
        assert!(result.is_ok());
        let plan = prepare_patch_replay_plan(&layout, "heads/main");
        assert!(plan.is_ok());
        if let Ok(plan) = plan {
            assert_eq!(plan.block_count, 2);
            assert_eq!(plan.patch_count, 1);
            assert_eq!(plan.applied_operation_count, 2);
            assert_eq!(plan.file_count, 2);
            assert!(plan.paths.contains(&"README.md".to_string()));
            assert!(plan.paths.contains(&"extra.txt".to_string()));
            assert!(!plan.paths.contains(&"old.txt".to_string()));
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

/// Publish a snapshot root followed by a supported file-operation patch block.
pub(crate) fn publish_snapshot_then_patch_block(
    layout: &RepositoryLayout,
) -> prikk_error::Result<()> {
    let mut object_store = FileObjectStore::new(layout.clone());
    let old_blob = write_blob(&mut object_store, b"old\n")?;
    let extra_blob = write_blob(&mut object_store, b"extra\n")?;

    let snapshot_manifest = SnapshotManifest {
        files: vec![
            SnapshotEntry {
                path: RepoPath::parse("README.md")?,
                bytes: b"hello\n".to_vec(),
            },
            SnapshotEntry {
                path: RepoPath::parse("old.txt")?,
                bytes: b"old\n".to_vec(),
            },
        ],
    };
    let snapshot_blob = BlobPayload::new(BlobKind::Snapshot, snapshot_manifest.encode()?);
    let snapshot_bytes = snapshot_blob.to_canonical_bytes()?;
    let mut snapshot_envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, snapshot_bytes);
    snapshot_envelope.add_signature(maintainer_signature())?;
    let snapshot_blob_id = object_store.write_object(&snapshot_envelope)?;

    let root_block = signed_block(
        BlockKind::Root,
        Vec::new(),
        Vec::new(),
        Some(snapshot_blob_id),
    );
    let root_block_id = object_store.write_object(&root_block)?;

    // ReplaceBinary is reconciled to the FDD-03 §9.3 node-addressed record but its
    // replay is deferred to the node model (increment 4.4); this harness exercises
    // the still-supported DeleteNode + CreateFile replay. README.md is carried
    // through from the snapshot baseline unchanged.
    let patch_payload = PatchPayload {
        operations: vec![
            Operation {
                op_seq: 1,
                op_id: None,
                preconditions: Vec::new(),
                kind: OperationKind::DeleteNode(DeleteNode {
                    path: "old.txt".to_string(),
                    node_id: NodeId::from_bytes([0x71; 32]),
                    old_node_kind: NodeKind::TextFile,
                    preimage: DeleteNodePreimage::File {
                        old_blob_id: old_blob,
                        old_mode: 0o100644,
                    },
                }),
            },
            Operation {
                op_seq: 2,
                op_id: None,
                preconditions: Vec::new(),
                kind: OperationKind::CreateFile(CreateFile {
                    path: "extra.txt".to_string(),
                    node_id: NodeId::from_bytes([0x72; 32]),
                    blob_id: extra_blob,
                    mode: 0o100644,
                }),
            },
        ],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
    };
    let mut patch =
        ObjectEnvelope::unsigned(ObjectType::Patch, 1, patch_payload.to_canonical_bytes()?);
    patch.add_signature(dummy_signature())?;
    let patch_id = object_store.write_object(&patch)?;

    let patch_block = signed_block(BlockKind::Normal, vec![root_block_id], vec![patch_id], None);
    let patch_block_id = object_store.write_object(&patch_block)?;

    let ref_store = RefStore::new(layout.clone());
    let root_ref_state = signed_ref_state_envelope("heads/main", None, root_block_id, 1);
    let root_ref_state_id = root_ref_state.object_id();
    let root_ref_update =
        signed_ref_update_envelope("heads/main", None, root_ref_state_id, root_block_id, 1);
    ref_store.publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_state: root_ref_state,
        ref_update: root_ref_update,
    })?;

    let patch_ref_state =
        signed_ref_state_envelope("heads/main", Some(root_ref_state_id), patch_block_id, 2);
    let patch_ref_state_id = patch_ref_state.object_id();
    let patch_ref_update = signed_ref_update_envelope(
        "heads/main",
        Some(root_ref_state_id),
        patch_ref_state_id,
        patch_block_id,
        2,
    );
    ref_store.publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: Some(root_ref_state_id),
        ref_state: patch_ref_state,
        ref_update: patch_ref_update,
    })?;
    Ok(())
}

fn write_blob(
    store: &mut FileObjectStore,
    bytes: &[u8],
) -> prikk_error::Result<prikk_object::ObjectId> {
    let payload = BlobPayload::new(BlobKind::Text, bytes.to_vec());
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, payload.to_canonical_bytes()?);
    envelope.add_signature(maintainer_signature())?;
    store.write_object(&envelope)
}

fn signed_block(
    kind: BlockKind,
    parent_block_ids: Vec<prikk_object::ObjectId>,
    patch_ids: Vec<prikk_object::ObjectId>,
    snapshot_blob_ref: Option<prikk_object::ObjectId>,
) -> ObjectEnvelope {
    let payload = BlockPayload {
        parent_block_ids,
        kind,
        patch_ids,
        state_merkle_root: MerkleRoot([0_u8; 32]),
        snapshot_blob_ref,
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::Block, 1, payload_bytes.unwrap_or_default());
    assert!(envelope.add_signature(maintainer_signature()).is_ok());
    envelope
}
