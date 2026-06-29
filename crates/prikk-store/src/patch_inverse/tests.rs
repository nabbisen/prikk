//! Patch inverse planning tests.

use prikk_object::{
    BlobKind, BlobPayload, BlockKind, BlockPayload, CanonicalEncode, CreateFile, DeleteNode,
    DeleteNodePreimage, MerkleRoot, NodeId, NodeKind, ObjectEnvelope, ObjectType, Operation,
    OperationKind, PatchPayload,
};

use crate::{
    FileObjectStore, ObjectWriter, RefPublication, RefStore, RepoPath, RepositoryLayout,
    SnapshotEntry, SnapshotManifest, prepare_patch_inverse_plan,
};

use crate::test_support::{
    dummy_signature, maintainer_signature, signed_ref_state_envelope, signed_ref_update_envelope,
    unique_temp_dir,
};

#[test]
fn inverse_plan_reverses_supported_file_operations() {
    let root = unique_temp_dir("patch-inverse-file-ops");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let result = publish_snapshot_then_patch_block(&layout);
        assert!(result.is_ok());
        let plan = prepare_patch_inverse_plan(&layout, "heads/main");
        assert!(plan.is_ok());
        if let Ok(plan) = plan {
            assert_eq!(plan.block_count, 2);
            assert_eq!(plan.patch_count, 1);
            assert_eq!(plan.original_operation_count, 2);
            assert_eq!(plan.inverse_operation_count, 2);
            let labels: Vec<&str> = plan
                .operations
                .iter()
                .map(|operation| operation.kind.as_str())
                .collect();
            assert_eq!(labels, vec!["delete-file", "create-file"]);
            let paths: Vec<&str> = plan
                .operations
                .iter()
                .map(|operation| operation.path.as_str())
                .collect();
            assert_eq!(paths, vec!["extra.txt", "old.txt"]);
            let seqs: Vec<u32> = plan
                .operations
                .iter()
                .map(|operation| operation.op_seq)
                .collect();
            assert_eq!(seqs, vec![1, 2]);
            assert_eq!(plan.inverse_payload.operations.len(), 2);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

fn publish_snapshot_then_patch_block(layout: &RepositoryLayout) -> prikk_error::Result<()> {
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
    let patch_id = write_patch(&mut object_store, patch_payload)?;
    publish_root_then_patch_ref(layout, root_block_id, patch_id)
}

fn write_patch(
    object_store: &mut FileObjectStore,
    payload: PatchPayload,
) -> prikk_error::Result<prikk_object::ObjectId> {
    let mut patch = ObjectEnvelope::unsigned(ObjectType::Patch, 1, payload.to_canonical_bytes()?);
    patch.add_signature(dummy_signature())?;
    object_store.write_object(&patch)
}

fn publish_root_then_patch_ref(
    layout: &RepositoryLayout,
    root_block_id: prikk_object::ObjectId,
    patch_id: prikk_object::ObjectId,
) -> prikk_error::Result<()> {
    let mut object_store = FileObjectStore::new(layout.clone());
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
    ref_store
        .publish(&RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: Some(root_ref_state_id),
            ref_state: patch_ref_state,
            ref_update: patch_ref_update,
        })
        .map(|_object_id| ())
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

/// Publish a snapshot containing a binary file, then a block whose only operation
/// deletes it as a `BINARY_FILE` node referencing a `BlobKind::Binary` blob. This
/// exercises inverse(DeleteNode) over a binary blob through the kind-aware
/// `ensure_blob_matches_node_kind` consume-path check (FDD-03 §9.3).
fn publish_binary_file_delete_block(layout: &RepositoryLayout) -> prikk_error::Result<()> {
    let mut object_store = FileObjectStore::new(layout.clone());
    let bin_bytes: Vec<u8> = vec![0x00, 0x01, 0xff, 0x10];
    let bin_blob = BlobPayload::new(BlobKind::Binary, bin_bytes.clone());
    let mut bin_envelope =
        ObjectEnvelope::unsigned(ObjectType::Blob, 1, bin_blob.to_canonical_bytes()?);
    bin_envelope.add_signature(maintainer_signature())?;
    let bin_blob_id = object_store.write_object(&bin_envelope)?;

    let snapshot_manifest = SnapshotManifest {
        files: vec![SnapshotEntry {
            path: RepoPath::parse("data.bin")?,
            bytes: bin_bytes,
        }],
    };
    let snapshot_blob = BlobPayload::new(BlobKind::Snapshot, snapshot_manifest.encode()?);
    let mut snapshot_envelope =
        ObjectEnvelope::unsigned(ObjectType::Blob, 1, snapshot_blob.to_canonical_bytes()?);
    snapshot_envelope.add_signature(maintainer_signature())?;
    let snapshot_blob_id = object_store.write_object(&snapshot_envelope)?;

    let root_block = signed_block(
        BlockKind::Root,
        Vec::new(),
        Vec::new(),
        Some(snapshot_blob_id),
    );
    let root_block_id = object_store.write_object(&root_block)?;

    let patch_payload = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::DeleteNode(DeleteNode {
                path: "data.bin".to_string(),
                node_id: NodeId::from_bytes([0x81; 32]),
                old_node_kind: NodeKind::BinaryFile,
                preimage: DeleteNodePreimage::File {
                    old_blob_id: bin_blob_id,
                    old_mode: 0o100644,
                },
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
    };
    let patch_id = write_patch(&mut object_store, patch_payload)?;
    publish_root_then_patch_ref(layout, root_block_id, patch_id)
}

#[test]
fn inverse_plan_reverses_binary_file_deletion() {
    let root = unique_temp_dir("patch-inverse-binary-delete");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(publish_binary_file_delete_block(&layout).is_ok());
        let plan = prepare_patch_inverse_plan(&layout, "heads/main");
        assert!(
            plan.is_ok(),
            "binary-file DeleteNode inverse should succeed"
        );
        if let Ok(plan) = plan {
            assert_eq!(plan.original_operation_count, 1);
            assert_eq!(plan.inverse_operation_count, 1);
            // inverse(DeleteNode) reconstructs the file via CreateFile.
            assert_eq!(
                plan.operations.first().map(|op| op.kind.as_str()),
                Some("create-file")
            );
            assert_eq!(
                plan.operations.first().map(|op| op.path.as_str()),
                Some("data.bin")
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}
