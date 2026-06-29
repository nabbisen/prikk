//! Patch operations list framing tests (FDD-03 §9.1 tag 1).
//!
//! `PatchPayload.operations` must be framed as `record_list_item` (0x21). The
//! store decoder requires that wire type; this proves both the positive case and
//! the rejection of the pre-§9.1 `record` (0x20) framing the increment closes.
#![allow(clippy::expect_used)]

use prikk_object::{
    CanonicalEncode, CreateFile, NodeId, ObjectId, Operation, OperationKind, PatchPayload,
};

use crate::patch_replay::decode::decode_supported_patch_operations;

fn valid_patch_payload_bytes() -> Vec<u8> {
    let patch = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::CreateFile(CreateFile {
                path: "a.txt".to_string(),
                node_id: NodeId::from_bytes([0x74; 32]),
                blob_id: ObjectId::from_bytes([0x11; 32]),
                mode: 0o100_644,
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
    };
    patch.to_canonical_bytes().expect("patch encodes")
}

#[test]
fn decoder_accepts_record_list_item_framing() {
    let bytes = valid_patch_payload_bytes();
    // First field is operations (tag 1); its value_type byte is at offset 2.
    assert_eq!(
        bytes.get(2).copied(),
        Some(0x21),
        "expected record_list_item"
    );
    assert!(decode_supported_patch_operations(&bytes).is_ok());
}

#[test]
fn decoder_rejects_old_record_framing() {
    let mut bytes = valid_patch_payload_bytes();
    // Downgrade the operations item type from record_list_item (0x21) to the
    // pre-§9.1 record (0x20) framing; the nested operation bytes are otherwise
    // valid, so this isolates the framing regression. The decoder must reject.
    assert_eq!(bytes.get(2).copied(), Some(0x21));
    if let Some(byte) = bytes.get_mut(2) {
        *byte = 0x20;
    }
    assert!(decode_supported_patch_operations(&bytes).is_err());
}

#[test]
fn decoder_rejects_empty_payload() {
    // FDD-03 §9.1: a persisted/imported patch with no operations is malformed.
    assert!(decode_supported_patch_operations(b"").is_err());
}

#[test]
fn decoder_rejects_payload_without_operations() {
    // A parent-only payload (tag 2, object_id) carries no tag-1 operation item.
    // The read path must reject it even though the bytes are well-formed TLV.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&2_u16.to_be_bytes()); // tag 2 = parent_patch_ids
    bytes.push(0x12); // value_type object_id
    bytes.extend_from_slice(&32_u64.to_be_bytes()); // len = 32
    bytes.extend_from_slice(&[0x11_u8; 32]); // 32-byte object id
    assert!(decode_supported_patch_operations(&bytes).is_err());
}
