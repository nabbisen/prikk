//! Shared test fixtures.

use prikk_object::{
    BlockKind, BlockPayload, CanonicalEncode, EditText, MerkleRoot, ObjectEnvelope, ObjectId,
    ObjectType, Operation, OperationKind, PatchPayload, RefKind, RefStatePayload, RefUpdatePayload,
    Signature, SignatureAlgorithm, SignerRole,
};

pub(crate) fn signed_patch_envelope() -> ObjectEnvelope {
    let payload = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::EditText(EditText {
                path: "a.txt".to_string(),
                anchor_id: "anchor-1".to_string(),
                old_span_hash: vec![1, 2, 3],
                replacement: "hello".to_string(),
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let bytes = payload_bytes.unwrap_or_default();
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, bytes);
    assert!(envelope.add_signature(dummy_signature()).is_ok());
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
    path.push(format!("prikk-pr014-{name}-{}-{}", std::process::id(), monotonic_suffix()));
    path
}

fn monotonic_suffix() -> u128 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_nanos(),
        Err(_) => 0,
    }
}
