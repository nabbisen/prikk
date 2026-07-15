//! Shared test fixtures and cross-module test harnesses.

use prikk_object::{
    BlockKind, BlockPayload, CanonicalEncode, ChangePerm, CreateFile, EditText, MerkleRoot, NodeId,
    ObjectEnvelope, ObjectId, ObjectType, Operation, OperationKind, PatchPayload, PatchPurpose,
    RefKind, RefStatePayload, RefUpdatePayload, Signature, SignatureAlgorithm, SignerRole,
};

use crate::{
    FileObjectStore, ObjectWriter, RefPublication, RefStore, RepoPath, RepositoryLayout,
    SnapshotEntry, SnapshotManifest,
};
use prikk_object::{BlobKind, BlobPayload, DeleteNode, DeleteNodePreimage, NodeKind};

pub(crate) fn signed_patch_envelope() -> ObjectEnvelope {
    let payload = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::CreateFile(CreateFile {
                path: "a.txt".to_string(),
                node_id: NodeId::from_bytes([0x51; 32]),
                blob_id: sample_object_id("patch-envelope-blob"),
                mode: 0o100_644,
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let bytes = payload_bytes.unwrap_or_default();
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, bytes);
    assert!(envelope.add_signature(rollback_author_signature()).is_ok());
    envelope
}

/// Return a supported rollback-marked Patch envelope for sealed-history classification tests.
pub(crate) fn rollback_patch_envelope() -> ObjectEnvelope {
    let payload = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::CreateFile(CreateFile {
                path: "rollback.txt".to_string(),
                node_id: NodeId::from_bytes([0x73; 32]),
                blob_id: sample_object_id("rollback-created"),
                mode: 0o100644,
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::RollbackDraft,
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let bytes = payload_bytes.unwrap_or_default();
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, bytes);
    assert!(envelope.add_signature(rollback_author_signature()).is_ok());
    envelope
}

pub(crate) fn signed_empty_block_envelope() -> ObjectEnvelope {
    let payload = BlockPayload {
        parent_block_ids: Vec::new(),
        kind: BlockKind::Root,
        patch_ids: Vec::new(),
        state_merkle_root: MerkleRoot([0_u8; 32]),
        snapshot_blob_ref: None,
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let bytes = payload_bytes.unwrap_or_default();
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Block, 1, bytes);
    assert!(envelope.add_signature(maintainer_signature()).is_ok());
    envelope
}

pub(crate) fn signed_ref_state_envelope(
    ref_name: &str,
    previous_ref_state_id: Option<ObjectId>,
    target_object_id: ObjectId,
    update_seq: u64,
) -> ObjectEnvelope {
    let payload = RefStatePayload {
        ref_name: ref_name.to_string(),
        kind: RefKind::Branch,
        target_object_id,
        update_seq,
        previous_ref_state_id,
        required_attestation_ids: Vec::new(),
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let bytes = payload_bytes.unwrap_or_default();
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::RefState, 1, bytes);
    assert!(envelope.add_signature(maintainer_signature()).is_ok());
    envelope
}

pub(crate) fn signed_ref_update_envelope(
    ref_name: &str,
    old_ref_state_id: Option<ObjectId>,
    new_ref_state_id: ObjectId,
    new_target_object_id: ObjectId,
    update_seq: u64,
) -> ObjectEnvelope {
    let payload = RefUpdatePayload {
        ref_name: ref_name.to_string(),
        old_ref_state_id,
        new_ref_state_id,
        new_target_object_id,
        update_seq,
        created_at: 7,
        author_key_id: "maintainer-key".to_string(),
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let bytes = payload_bytes.unwrap_or_default();
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::RefUpdate, 1, bytes);
    assert!(envelope.add_signature(maintainer_signature()).is_ok());
    envelope
}

pub(crate) fn sample_object_id(label: &str) -> ObjectId {
    ObjectId::from_canonical_payload(ObjectType::Blob, 1, label.as_bytes())
}

pub(crate) fn dummy_signature() -> Signature {
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "author-key".to_string(),
        signature_bytes: vec![1, 2, 3, 4],
        created_at: 7,
        signer_role: SignerRole::Author,
    }
}

pub(crate) fn rollback_author_signature() -> Signature {
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "rollback-author-key".to_string(),
        signature_bytes: vec![7; 64],
        created_at: 7,
        signer_role: SignerRole::Author,
    }
}

pub(crate) fn legacy_rollback_marker_signature() -> Signature {
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "dev-placeholder-rollback-author".to_string(),
        signature_bytes: vec![9, 9, 9, 9],
        created_at: 9,
        signer_role: SignerRole::Author,
    }
}

pub(crate) fn maintainer_signature() -> Signature {
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "maintainer-key".to_string(),
        signature_bytes: vec![5, 6, 7, 8],
        created_at: 8,
        signer_role: SignerRole::Maintainer,
    }
}

pub(crate) fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "prikk-pr014-{name}-{}-{}",
        std::process::id(),
        monotonic_suffix()
    ));
    assert!(std::fs::create_dir_all(&path).is_ok());
    path
}

fn monotonic_suffix() -> u128 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    }
}

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
        purpose: PatchPurpose::Normal,
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

pub(crate) fn publish_text_create_then_edit_block(
    layout: &RepositoryLayout,
    old: &[u8],
    new: &[u8],
) -> prikk_error::Result<()> {
    let mut object_store = FileObjectStore::new(layout.clone());
    let node_id = NodeId::from_bytes([0x81; 32]);
    let old_blob = write_blob(&mut object_store, old)?;
    let span = crate::text_span::plan_authored_text_span(old, new, node_id)
        .map_err(|err| prikk_error::PrikkError::Integrity(err.to_string()))?
        .ok_or_else(|| prikk_error::PrikkError::Integrity("test edit is unchanged".to_string()))?;

    let patch_payload = PatchPayload {
        operations: vec![
            Operation {
                op_seq: 1,
                op_id: None,
                preconditions: Vec::new(),
                kind: OperationKind::CreateFile(CreateFile {
                    path: "README.md".to_string(),
                    node_id,
                    blob_id: old_blob,
                    mode: 0o100644,
                }),
            },
            Operation {
                op_seq: 2,
                op_id: None,
                preconditions: Vec::new(),
                kind: OperationKind::EditText(EditText {
                    node_id,
                    span_id: span.span_id,
                    old_span_hash: span.old_span_hash,
                    left_anchor_hash: span.left_anchor_hash,
                    right_anchor_hash: span.right_anchor_hash,
                    replacement_text: span.replacement_text,
                    presentation_hint_line: None,
                    presentation_hint_column: None,
                    old_span_text: span.old_span_text,
                }),
            },
        ],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let mut patch =
        ObjectEnvelope::unsigned(ObjectType::Patch, 1, patch_payload.to_canonical_bytes()?);
    patch.add_signature(dummy_signature())?;
    let patch_id = object_store.write_object(&patch)?;
    let block = signed_block(BlockKind::Root, Vec::new(), vec![patch_id], None);
    let block_id = object_store.write_object(&block)?;

    let ref_store = RefStore::new(layout.clone());
    let ref_state = signed_ref_state_envelope("heads/main", None, block_id, 1);
    let ref_state_id = ref_state.object_id();
    let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, block_id, 1);
    ref_store.publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_state,
        ref_update,
    })?;
    Ok(())
}

pub(crate) fn publish_text_edit_then_unsupported_change_perm_block(
    layout: &RepositoryLayout,
) -> prikk_error::Result<()> {
    let mut object_store = FileObjectStore::new(layout.clone());
    let node_id = NodeId::from_bytes([0x82; 32]);
    let old = b"alpha beta\n";
    let new = b"alpha BETA\n";
    let old_blob = write_blob(&mut object_store, old)?;
    let span = crate::text_span::plan_authored_text_span(old, new, node_id)
        .map_err(|err| prikk_error::PrikkError::Integrity(err.to_string()))?
        .ok_or_else(|| prikk_error::PrikkError::Integrity("test edit is unchanged".to_string()))?;

    let patch_payload = PatchPayload {
        operations: vec![
            Operation {
                op_seq: 1,
                op_id: None,
                preconditions: Vec::new(),
                kind: OperationKind::CreateFile(CreateFile {
                    path: "README.md".to_string(),
                    node_id,
                    blob_id: old_blob,
                    mode: 0o100644,
                }),
            },
            Operation {
                op_seq: 2,
                op_id: None,
                preconditions: Vec::new(),
                kind: OperationKind::EditText(EditText {
                    node_id,
                    span_id: span.span_id,
                    old_span_hash: span.old_span_hash,
                    left_anchor_hash: span.left_anchor_hash,
                    right_anchor_hash: span.right_anchor_hash,
                    replacement_text: span.replacement_text,
                    presentation_hint_line: None,
                    presentation_hint_column: None,
                    old_span_text: span.old_span_text,
                }),
            },
            Operation {
                op_seq: 3,
                op_id: None,
                preconditions: Vec::new(),
                kind: OperationKind::ChangePerm(ChangePerm {
                    node_id,
                    old_mode: 0o100644,
                    new_mode: 0o100755,
                }),
            },
        ],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let mut patch =
        ObjectEnvelope::unsigned(ObjectType::Patch, 1, patch_payload.to_canonical_bytes()?);
    patch.add_signature(dummy_signature())?;
    let patch_id = object_store.write_object(&patch)?;
    let block = signed_block(BlockKind::Root, Vec::new(), vec![patch_id], None);
    let block_id = object_store.write_object(&block)?;

    let ref_store = RefStore::new(layout.clone());
    let ref_state = signed_ref_state_envelope("heads/main", None, block_id, 1);
    let ref_state_id = ref_state.object_id();
    let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, block_id, 1);
    ref_store.publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_state,
        ref_update,
    })?;
    Ok(())
}

pub(crate) fn write_blob(
    store: &mut FileObjectStore,
    bytes: &[u8],
) -> prikk_error::Result<prikk_object::ObjectId> {
    let payload = BlobPayload::new(BlobKind::Text, bytes.to_vec());
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, payload.to_canonical_bytes()?);
    envelope.add_signature(maintainer_signature())?;
    store.write_object(&envelope)
}

pub(crate) fn signed_block(
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
