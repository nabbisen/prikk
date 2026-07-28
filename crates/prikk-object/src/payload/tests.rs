//! Payload tests.

mod proptest_decoders;

use super::{
    AttestationPayload, AttestationStatus, BlobKind, BlobPayload, BlockKind, BlockPayload,
    EditText, MerkleRoot, Operation, OperationKind, PatchPayload, PatchPurpose, PluginResultEntry,
    RefKind, RefStatePayload, RefUpdatePayload, text_span_hash, validate_text_anchor_id,
};
use crate::{CanonicalEncode, ObjectId, ObjectType};

#[test]
fn text_anchor_ids_are_validated() {
    assert!(validate_text_anchor_id("anchor-1").is_ok());
    assert!(validate_text_anchor_id("").is_err());
    assert!(validate_text_anchor_id("with space").is_err());
}

#[test]
fn text_span_hash_is_stable() {
    let a = text_span_hash(b"hello");
    let b = text_span_hash(b"hello");
    let c = text_span_hash(b"world");
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn edit_text_rejects_hash_binding_violation() {
    // FDD-03 §9.3: old_span_hash must equal SHA-256(old_span_text).
    let op = EditText {
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        span_id: [0x10; 32],
        old_span_hash: [0x00; 32], // wrong: not SHA-256(old_span_text)
        left_anchor_hash: [0x11; 32],
        right_anchor_hash: [0x12; 32],
        replacement_text: b"hello".to_vec(),
        presentation_hint_line: None,
        presentation_hint_column: None,
        old_span_text: b"old".to_vec(),
    };
    assert!(op.validate().is_err());
    assert!(op.to_canonical_bytes().is_err());
}

#[test]
fn patch_operations_must_be_contiguous() {
    let patch = PatchPayload {
        operations: vec![Operation {
            op_seq: 2,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::EditText(EditText {
                node_id: crate::NodeId::from_bytes([0x22; 32]),
                span_id: [0x10; 32],
                old_span_hash: text_span_hash(b"old"),
                left_anchor_hash: [0x11; 32],
                right_anchor_hash: [0x12; 32],
                replacement_text: b"hello".to_vec(),
                presentation_hint_line: None,
                presentation_hint_column: None,
                old_span_text: b"old".to_vec(),
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    assert!(patch.to_canonical_bytes().is_err());
}

#[test]
fn blob_payload_has_stable_object_id() {
    let payload = BlobPayload::new(BlobKind::Text, b"hello".to_vec());
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

fn plugin_result(plugin_id: &str, plugin_version: &str, report_byte: u8) -> PluginResultEntry {
    PluginResultEntry {
        plugin_id: plugin_id.to_string(),
        plugin_version: plugin_version.to_string(),
        status: AttestationStatus::Pass,
        report_hash: vec![report_byte; 32],
        finding_count: 0,
    }
}

fn attestation_with(results: Vec<PluginResultEntry>) -> AttestationPayload {
    AttestationPayload {
        target_block_id: ObjectId::from_bytes([0x11; 32]),
        policy_version: "v1".to_string(),
        plugin_set_hash: vec![0x22; 32],
        results,
        status: AttestationStatus::Pass,
        created_at: 1,
        is_reproducible_offline: true,
    }
}

#[test]
fn attestation_results_accept_ascending_plugin_id() {
    let att = attestation_with(vec![
        plugin_result("audit-a", "0.1", 1),
        plugin_result("audit-b", "0.1", 1),
    ]);
    assert!(att.to_canonical_bytes().is_ok());
}

#[test]
fn attestation_results_reject_reverse_order() {
    let att = attestation_with(vec![
        plugin_result("audit-b", "0.1", 1),
        plugin_result("audit-a", "0.1", 1),
    ]);
    assert!(att.to_canonical_bytes().is_err());
}

#[test]
fn attestation_results_reject_duplicate_plugin_id_differing_version() {
    let att = attestation_with(vec![
        plugin_result("audit", "0.1", 1),
        plugin_result("audit", "0.2", 1),
    ]);
    assert!(
        att.to_canonical_bytes().is_err(),
        "duplicate plugin_id must be rejected even when plugin_version differs"
    );
}

#[test]
fn attestation_results_reject_duplicate_plugin_id_differing_report_hash() {
    let att = attestation_with(vec![
        plugin_result("audit", "0.1", 1),
        plugin_result("audit", "0.1", 2),
    ]);
    assert!(
        att.to_canonical_bytes().is_err(),
        "duplicate plugin_id must be rejected even when report_hash differs"
    );
}

#[test]
fn blob_kind_from_code_rejects_invalid_and_unknown() {
    use super::BlobKind;
    assert!(BlobKind::from_code(0x0000).is_err());
    assert!(BlobKind::from_code(0x00ff).is_err());
    assert_eq!(BlobKind::from_code(0x0001), Ok(BlobKind::Text));
    assert_eq!(BlobKind::from_code(0x0002), Ok(BlobKind::Binary));
    assert_eq!(BlobKind::from_code(0x0003), Ok(BlobKind::Snapshot));
}

#[test]
fn blob_encode_rejects_declared_size_mismatch() {
    let bad = BlobPayload {
        blob_kind: BlobKind::Text,
        content: b"abc".to_vec(),
        declared_size: 99,
    };
    assert!(
        bad.to_canonical_bytes().is_err(),
        "declared_size must equal content length"
    );
}

#[allow(clippy::expect_used)]
#[test]
fn blob_round_trips_via_new() {
    let blob = BlobPayload::new(BlobKind::Binary, vec![0x01, 0x02, 0x03]);
    let bytes = blob.to_canonical_bytes().expect("encode");
    let decoded = BlobPayload::decode_canonical(&bytes).expect("decode");
    assert_eq!(decoded, blob);
    assert_eq!(decoded.declared_size, 3);
}

#[test]
fn blob_decode_rejects_declared_size_mismatch() {
    // hand-craft: blob_kind=Text(1), content="ab"(2 bytes), declared_size=5
    let mut p = Vec::new();
    p.extend_from_slice(&1u16.to_be_bytes());
    p.push(0x05); // enum_u16
    p.extend_from_slice(&2u64.to_be_bytes());
    p.extend_from_slice(&1u16.to_be_bytes()); // Text
    p.extend_from_slice(&2u16.to_be_bytes());
    p.push(0x11); // bytes
    p.extend_from_slice(&2u64.to_be_bytes());
    p.extend_from_slice(b"ab");
    p.extend_from_slice(&3u16.to_be_bytes());
    p.push(0x04); // u64
    p.extend_from_slice(&8u64.to_be_bytes());
    p.extend_from_slice(&5u64.to_be_bytes()); // declared_size=5 != 2
    assert!(BlobPayload::decode_canonical(&p).is_err());
}

#[test]
fn node_kind_from_code_rejects_invalid_and_unknown() {
    use super::NodeKind;
    assert!(NodeKind::from_code(0x0000).is_err());
    assert!(NodeKind::from_code(0x00ff).is_err());
    assert_eq!(NodeKind::from_code(0x0001), Ok(NodeKind::TextFile));
    assert_eq!(NodeKind::from_code(0x0002), Ok(NodeKind::BinaryFile));
    assert_eq!(NodeKind::from_code(0x0003), Ok(NodeKind::Symlink));
}

#[test]
fn node_kind_derives_from_file_blob_kind() {
    use super::{BlobKind, NodeKind};
    assert_eq!(
        NodeKind::from_file_blob_kind(BlobKind::Text),
        Ok(NodeKind::TextFile)
    );
    assert_eq!(
        NodeKind::from_file_blob_kind(BlobKind::Binary),
        Ok(NodeKind::BinaryFile)
    );
    assert!(
        NodeKind::from_file_blob_kind(BlobKind::Snapshot).is_err(),
        "a file node must not derive from a SNAPSHOT blob"
    );
}

#[test]
fn node_id_round_trips_bytes() {
    use super::NodeId;
    let raw = [0x5a_u8; 32];
    let id = NodeId::from_bytes(raw);
    assert_eq!(id.as_bytes(), &raw);
}

#[test]
fn node_id_try_from_bytes_rejects_all_zero() {
    use super::NodeId;
    assert!(NodeId::try_from_bytes([0_u8; 32]).is_err());
    let ok = NodeId::try_from_bytes([0x01_u8; 32]);
    assert!(ok.is_ok());
}

#[test]
fn patch_payload_rejects_empty_operations() {
    use super::PatchPayload;
    use crate::CanonicalEncode;
    let patch = PatchPayload {
        operations: Vec::new(),
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    // §9.1: operations is required with at least one operation.
    assert!(patch.validate().is_err());
    assert!(patch.to_canonical_bytes().is_err());
}

#[test]
fn patch_purpose_absent_decodes_as_normal() {
    let patch = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::EditText(EditText {
                node_id: crate::NodeId::from_bytes([0x22; 32]),
                span_id: [0x10; 32],
                old_span_hash: text_span_hash(b"old"),
                left_anchor_hash: [0x11; 32],
                right_anchor_hash: [0x12; 32],
                replacement_text: b"hello".to_vec(),
                presentation_hint_line: None,
                presentation_hint_column: None,
                old_span_text: b"old".to_vec(),
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let bytes = patch.to_canonical_bytes();
    assert!(bytes.is_ok());
    if let Ok(bytes) = bytes {
        assert_eq!(
            PatchPurpose::decode_from_patch_payload(&bytes),
            Ok(PatchPurpose::Normal)
        );
    }
}

#[test]
fn patch_purpose_explicit_normal_is_rejected() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&5_u16.to_be_bytes());
    bytes.push(crate::WireType::EnumU16 as u8);
    bytes.extend_from_slice(&2_u64.to_be_bytes());
    bytes.extend_from_slice(&PatchPurpose::Normal.code().to_be_bytes());
    assert!(PatchPurpose::decode_from_patch_payload(&bytes).is_err());
}
