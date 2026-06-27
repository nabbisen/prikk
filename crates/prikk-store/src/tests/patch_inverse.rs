//! Patch inverse planning tests.

use prikk_object::{
    text_span_hash, BlobPayload, BlockKind, BlockPayload, CanonicalEncode, CreateFile,
    DeleteFile, EditText, MerkleRoot, ObjectEnvelope, ObjectType, Operation, OperationKind,
    PatchPayload, ReplaceBinary,
};

use crate::{
    prepare_patch_inverse_plan, FileObjectStore, ObjectWriter, PatchInverseOperationKind,
    RefPublication, RefStore, RepoPath, RepositoryLayout, SnapshotEntry, SnapshotManifest,
};

use super::helpers::{
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
            assert_eq!(plan.original_operation_count, 3);
            assert_eq!(plan.inverse_operation_count, 3);
            let labels: Vec<&str> = plan
                .operations
                .iter()
                .map(|operation| operation.kind.as_str())
                .collect();
            assert_eq!(labels, vec!["delete-file", "create-file", "replace-binary"]);
            let paths: Vec<&str> = plan
                .operations
                .iter()
                .map(|operation| operation.path.as_str())
                .collect();
            assert_eq!(paths, vec!["extra.txt", "old.txt", "README.md"]);
            let seqs: Vec<u32> = plan.operations.iter().map(|operation| operation.op_seq).collect();
            assert_eq!(seqs, vec![1, 2, 3]);
            assert_eq!(plan.inverse_payload.operations.len(), 3);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn inverse_plan_builds_full_file_text_inverse() {
    let root = unique_temp_dir("patch-inverse-edit-text");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let result = publish_text_edit_block(&layout);
        assert!(result.is_ok());
        let plan = prepare_patch_inverse_plan(&layout, "heads/main");
        assert!(plan.is_ok());
        if let Ok(plan) = plan {
            assert_eq!(plan.original_operation_count, 1);
            assert_eq!(plan.inverse_operation_count, 1);
            let mut operations = plan.inverse_payload.operations.iter();
            if let Some(operation) = operations.next() {
                assert_eq!(operation.op_seq, 1);
                match &operation.kind {
                    OperationKind::EditText(edit) => {
                        assert_eq!(edit.path, "note.txt");
                        assert_eq!(edit.anchor_id, "full-file");
                        assert_eq!(edit.old_span_hash, text_span_hash(b"changed text"));
                        assert_eq!(edit.replacement, "hello text\n");
                    }
                    _ => panic!("expected inverse EditText"),
                }
            } else {
                panic!("expected one inverse operation");
            }
            assert!(operations.next().is_none());
            let kinds: Vec<PatchInverseOperationKind> =
                plan.operations.iter().map(|operation| operation.kind).collect();
            assert_eq!(kinds, vec![PatchInverseOperationKind::EditText]);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

fn publish_snapshot_then_patch_block(layout: &RepositoryLayout) -> prikk_error::Result<()> {
    let mut object_store = FileObjectStore::new(layout.clone());
    let readme_v1 = write_blob(&mut object_store, b"hello\n")?;
    let old_blob = write_blob(&mut object_store, b"old\n")?;
    let readme_v2 = write_blob(&mut object_store, b"changed\n")?;
    let extra_blob = write_blob(&mut object_store, b"extra\n")?;

    let snapshot_manifest = SnapshotManifest {
        files: vec![
            SnapshotEntry { path: RepoPath::parse("README.md")?, bytes: b"hello\n".to_vec() },
            SnapshotEntry { path: RepoPath::parse("old.txt")?, bytes: b"old\n".to_vec() },
        ],
    };
    let snapshot_blob = BlobPayload { bytes: snapshot_manifest.encode()? };
    let snapshot_bytes = snapshot_blob.to_canonical_bytes()?;
    let mut snapshot_envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, snapshot_bytes);
    snapshot_envelope.add_signature(maintainer_signature())?;
    let snapshot_blob_id = object_store.write_object(&snapshot_envelope)?;

    let root_block = signed_block(BlockKind::Root, Vec::new(), Vec::new(), Some(snapshot_blob_id));
    let root_block_id = object_store.write_object(&root_block)?;

    let patch_payload = PatchPayload {
        operations: vec![
            Operation {
                op_seq: 1,
                op_id: None,
                preconditions: Vec::new(),
                kind: OperationKind::ReplaceBinary(ReplaceBinary {
                    path: "README.md".to_string(),
                    old_blob_id: readme_v1,
                    new_blob_id: readme_v2,
                }),
            },
            Operation {
                op_seq: 2,
                op_id: None,
                preconditions: Vec::new(),
                kind: OperationKind::DeleteFile(DeleteFile {
                    path: "old.txt".to_string(),
                    old_blob_id: old_blob,
                }),
            },
            Operation {
                op_seq: 3,
                op_id: None,
                preconditions: Vec::new(),
                kind: OperationKind::CreateFile(CreateFile {
                    path: "extra.txt".to_string(),
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

fn publish_text_edit_block(layout: &RepositoryLayout) -> prikk_error::Result<()> {
    let mut object_store = FileObjectStore::new(layout.clone());
    let original = b"hello text\n";
    let snapshot_manifest = SnapshotManifest {
        files: vec![SnapshotEntry {
            path: RepoPath::parse("note.txt")?,
            bytes: original.to_vec(),
        }],
    };
    let snapshot_blob = BlobPayload { bytes: snapshot_manifest.encode()? };
    let snapshot_bytes = snapshot_blob.to_canonical_bytes()?;
    let mut snapshot_envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, snapshot_bytes);
    snapshot_envelope.add_signature(maintainer_signature())?;
    let snapshot_blob_id = object_store.write_object(&snapshot_envelope)?;

    let root_block = signed_block(BlockKind::Root, Vec::new(), Vec::new(), Some(snapshot_blob_id));
    let root_block_id = object_store.write_object(&root_block)?;

    let patch_payload = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::EditText(EditText {
                path: "note.txt".to_string(),
                anchor_id: "full-file".to_string(),
                old_span_hash: text_span_hash(original),
                replacement: "changed text".to_string(),
            }),
        }],
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
    let root_ref_update = signed_ref_update_envelope(
        "heads/main",
        None,
        root_ref_state_id,
        root_block_id,
        1,
    );
    ref_store.publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_state: root_ref_state,
        ref_update: root_ref_update,
    })?;

    let patch_ref_state = signed_ref_state_envelope(
        "heads/main",
        Some(root_ref_state_id),
        patch_block_id,
        2,
    );
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
    }).map(|_object_id| ())
}

fn write_blob(
    store: &mut FileObjectStore,
    bytes: &[u8],
) -> prikk_error::Result<prikk_object::ObjectId> {
    let payload = BlobPayload { bytes: bytes.to_vec() };
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
    let mut envelope = ObjectEnvelope::unsigned(
        ObjectType::Block,
        1,
        payload_bytes.unwrap_or_default(),
    );
    assert!(envelope.add_signature(maintainer_signature()).is_ok());
    envelope
}
