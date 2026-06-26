//! Payload tests.

use super::{
    BlobPayload, BlockKind, BlockPayload, EditText, MerkleRoot, Operation, OperationKind,
    PatchPayload, RefKind, RefStatePayload, RefUpdatePayload,
};
use crate::{CanonicalEncode, ObjectId, ObjectType};

#[test]
fn patch_operations_must_be_contiguous() {
    let patch = PatchPayload {
        operations: vec![Operation {
            op_seq: 2,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::EditText(EditText {
                path: "a.txt".to_string(),
                anchor_id: "anchor".to_string(),
                old_span_hash: vec![1],
                replacement: "hello".to_string(),
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
    };
    assert!(patch.to_canonical_bytes().is_err());
}

#[test]
fn blob_payload_has_stable_object_id() {
    let payload = BlobPayload { bytes: b"hello".to_vec() };
    let bytes_a = payload.to_canonical_bytes();
    let bytes_b = payload.to_canonical_bytes();
    assert_eq!(bytes_a, bytes_b);
    if let Ok(bytes) = bytes_a {
        let id_a = ObjectId::from_canonical_payload(ObjectType::Blob, 1, &bytes);
        let id_b = ObjectId::from_canonical_payload(ObjectType::Blob, 1, &bytes);
        assert_eq!(id_a, id_b);
    }
}

#[test]
fn ref_state_payload_decodes_its_canonical_bytes() {
    let target = ObjectId::from_canonical_payload(ObjectType::Block, 1, b"block");
    let previous = ObjectId::from_canonical_payload(ObjectType::RefState, 1, b"prev");
    let payload = RefStatePayload {
        ref_name: "heads/main".to_string(),
        kind: RefKind::Branch,
        target_object_id: target,
        update_seq: 2,
        previous_ref_state_id: Some(previous),
        required_attestation_ids: Vec::new(),
    };
    let bytes = payload.to_canonical_bytes();
    assert!(bytes.is_ok());
    if let Ok(bytes) = bytes {
        let decoded = RefStatePayload::decode_canonical(&bytes);
        assert_eq!(decoded, Ok(payload));
    }
}

#[test]
fn block_payload_decodes_its_canonical_bytes() {
    let patch = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"patch");
    let payload = BlockPayload {
        parent_block_ids: Vec::new(),
        kind: BlockKind::Root,
        patch_ids: vec![patch],
        state_merkle_root: MerkleRoot([7_u8; 32]),
        snapshot_blob_ref: None,
    };
    let bytes = payload.to_canonical_bytes();
    assert!(bytes.is_ok());
    if let Ok(bytes) = bytes {
        let decoded = BlockPayload::decode_canonical(&bytes);
        assert_eq!(decoded, Ok(payload));
    }
}

#[test]
fn ref_update_payload_decodes_its_canonical_bytes() {
    let previous = ObjectId::from_canonical_payload(ObjectType::RefState, 1, b"prev");
    let current = ObjectId::from_canonical_payload(ObjectType::RefState, 1, b"current");
    let block = ObjectId::from_canonical_payload(ObjectType::Block, 1, b"block");
    let payload = RefUpdatePayload {
        ref_name: "heads/main".to_string(),
        old_ref_state_id: Some(previous),
        new_ref_state_id: current,
        new_target_object_id: block,
        update_seq: 2,
        created_at: 9,
        author_key_id: "maintainer-key".to_string(),
    };
    let bytes = payload.to_canonical_bytes();
    assert!(bytes.is_ok());
    if let Ok(bytes) = bytes {
        let decoded = RefUpdatePayload::decode_canonical(&bytes);
        assert_eq!(decoded, Ok(payload));
    }
}
