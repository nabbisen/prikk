//! Payload tests.

mod path_validation;
mod proptest_decoders;

use super::{
    AttestationPayload, AttestationStatus, BlobKind, BlobPayload, BlockKind, BlockPayload,
    EditText, MerkleRoot, Operation, OperationKind, PatchPayload, PatchPurpose, PluginResultEntry,
    RECOGNITION_CLAIM_MAX_PATCH_IDS, REF_STATE_CLOSED_SCHEMA, RecognitionClaimPayload, RefKind,
    RefStatePayload, RefUpdatePayload, text_span_hash, validate_text_anchor_id,
};
use crate::{CanonicalEncode, ObjectId, ObjectType, WireType};

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
        closed: false,
    };
    let bytes = payload.to_canonical_bytes();
    assert!(bytes.is_ok());
    if let Ok(bytes) = bytes {
        let decoded = RefStatePayload::decode_canonical(&bytes, 1);
        assert_eq!(decoded, Ok(payload));
    }
}

/// DC-61 identity claim: an ordinary (open) RefState's canonical bytes carry no trace of field 7
/// at all, structurally — not merely "round-trips," which would also pass for an accidentally
/// emitted `false`. Tag 7 as a big-endian `u16` never appears in the byte stream.
#[allow(clippy::expect_used)]
#[test]
fn ref_state_payload_open_encoding_carries_no_field_seven() {
    let target = ObjectId::from_canonical_payload(ObjectType::Block, 1, b"block");
    let payload = RefStatePayload {
        ref_name: "heads/main".to_string(),
        kind: RefKind::Branch,
        target_object_id: target,
        update_seq: 2,
        previous_ref_state_id: None,
        required_attestation_ids: Vec::new(),
        closed: false,
    };
    let bytes = payload.to_canonical_bytes().expect("open payload encodes");
    let tag_seven = 7_u16.to_be_bytes();
    assert!(
        !bytes.windows(2).any(|window| window == tag_seven),
        "open RefState bytes must not contain a field-7 tag anywhere: {bytes:02x?}"
    );
}

/// The closed counterpart: field 7 is present and its wire-encoded value is exactly `true` (byte
/// `0x01`), never emitted as an explicit `false`.
#[allow(clippy::expect_used)]
#[test]
fn ref_state_payload_closed_encoding_carries_field_seven_true() {
    let target = ObjectId::from_canonical_payload(ObjectType::Block, 1, b"block");
    let payload = RefStatePayload {
        ref_name: "heads/main".to_string(),
        kind: RefKind::Branch,
        target_object_id: target,
        update_seq: 2,
        previous_ref_state_id: None,
        required_attestation_ids: Vec::new(),
        closed: true,
    };
    let bytes = payload
        .to_canonical_bytes()
        .expect("closed payload encodes");
    // tag(2) + wire_type(1) + len(8) + value(1) = 12 trailing bytes: 00 07 01 00..00 01 01
    let mut expected_tail = Vec::new();
    expected_tail.extend_from_slice(&7_u16.to_be_bytes());
    expected_tail.push(WireType::Bool as u8);
    expected_tail.extend_from_slice(&1_u64.to_be_bytes());
    expected_tail.push(1); // true
    assert!(
        bytes.ends_with(&expected_tail),
        "closed RefState bytes must end with an explicit tag-7 true field: {bytes:02x?}"
    );

    let schema_2 = RefStatePayload::decode_canonical(&bytes, REF_STATE_CLOSED_SCHEMA);
    assert_eq!(schema_2, Ok(payload));
}

/// Field 7 is schema-gated: legal only at `REF_STATE_CLOSED_SCHEMA` and above. A schema-1 reader
/// encountering it — the format-transition claim DC-61 makes — must reject it outright.
#[allow(clippy::expect_used)]
#[test]
fn ref_state_payload_rejects_closed_field_at_schema_one() {
    let target = ObjectId::from_canonical_payload(ObjectType::Block, 1, b"block");
    let payload = RefStatePayload {
        ref_name: "heads/main".to_string(),
        kind: RefKind::Branch,
        target_object_id: target,
        update_seq: 2,
        previous_ref_state_id: None,
        required_attestation_ids: Vec::new(),
        closed: true,
    };
    let bytes = payload
        .to_canonical_bytes()
        .expect("closed payload encodes");
    assert!(
        RefStatePayload::decode_canonical(&bytes, 1).is_err(),
        "a schema-1 reader must reject a payload carrying field 7"
    );
}

/// Canonical encoding must have exactly one representation of "closed": absent, never an explicit
/// `false`. A hand-crafted payload spelling "not closed" via `tag 7 = false` must be rejected even
/// at schema 2, the same discipline `patch_purpose_explicit_normal_is_rejected` applies to
/// `PatchPurpose`.
#[allow(clippy::expect_used)]
#[test]
fn ref_state_payload_rejects_explicit_false_closed_field() {
    let target = ObjectId::from_canonical_payload(ObjectType::Block, 1, b"block");
    let open = RefStatePayload {
        ref_name: "heads/main".to_string(),
        kind: RefKind::Branch,
        target_object_id: target,
        update_seq: 2,
        previous_ref_state_id: None,
        required_attestation_ids: Vec::new(),
        closed: false,
    };
    let mut bytes = open.to_canonical_bytes().expect("open payload encodes");
    // Hand-append an explicit tag-7 false field after the legitimate open encoding.
    bytes.extend_from_slice(&7_u16.to_be_bytes());
    bytes.push(WireType::Bool as u8);
    bytes.extend_from_slice(&1_u64.to_be_bytes());
    bytes.push(0); // false
    assert!(
        RefStatePayload::decode_canonical(&bytes, REF_STATE_CLOSED_SCHEMA).is_err(),
        "an explicit tag-7 false must be rejected, not accepted as a second spelling of open"
    );
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
        mainline_parent_id: None,
        merge_baseline_block_id: None,
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

#[test]
fn recognition_claim_payload_decodes_its_canonical_bytes() {
    let block = ObjectId::from_canonical_payload(ObjectType::Block, 2, b"block");
    let patch_a = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"a");
    let patch_b = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"b");
    let (first, second) = if patch_a < patch_b {
        (patch_a, patch_b)
    } else {
        (patch_b, patch_a)
    };
    let payload = RecognitionClaimPayload {
        block_id: block,
        patch_ids: vec![first, second],
    };
    let bytes = payload.to_canonical_bytes();
    assert!(bytes.is_ok());
    if let Ok(bytes) = bytes {
        let decoded = RecognitionClaimPayload::decode_canonical(&bytes);
        assert_eq!(decoded, Ok(payload));
    }
}

/// Hand-write one canonical TLV field: tag (u16 BE) ‖ wire type (u8) ‖ length (u64 BE) ‖ value.
fn write_field(out: &mut Vec<u8>, tag: u16, wire_type: WireType, value: &[u8]) {
    out.extend_from_slice(&tag.to_be_bytes());
    out.push(wire_type as u8);
    out.extend_from_slice(&(value.len() as u64).to_be_bytes());
    out.extend_from_slice(value);
}

#[test]
fn recognition_claim_payload_rejects_unsorted_patch_ids_at_encode_and_decode() {
    let block = ObjectId::from_canonical_payload(ObjectType::Block, 2, b"block");
    let patch_a = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"a");
    let patch_b = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"b");
    let (first, second) = if patch_a < patch_b {
        (patch_a, patch_b)
    } else {
        (patch_b, patch_a)
    };

    // Encoder: deliberately descending -- must be refused, not silently sorted.
    let payload = RecognitionClaimPayload {
        block_id: block,
        patch_ids: vec![second, first],
    };
    assert!(payload.to_canonical_bytes().is_err());

    // Decoder: hand it unsorted bytes directly, built by hand rather than through the encoder
    // (which would itself refuse) -- per §7 row 5, the decoder must refuse independently, not
    // merely inherit the encoder's own check.
    let mut bytes = Vec::new();
    write_field(&mut bytes, 1, WireType::ObjectId, block.as_bytes());
    write_field(&mut bytes, 2, WireType::ObjectId, second.as_bytes());
    write_field(&mut bytes, 2, WireType::ObjectId, first.as_bytes());
    assert!(RecognitionClaimPayload::decode_canonical(&bytes).is_err());
}

#[test]
fn recognition_claim_payload_rejects_duplicate_patch_ids_at_decode() {
    let block = ObjectId::from_canonical_payload(ObjectType::Block, 2, b"block");
    let patch = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"patch");
    let mut bytes = Vec::new();
    write_field(&mut bytes, 1, WireType::ObjectId, block.as_bytes());
    write_field(&mut bytes, 2, WireType::ObjectId, patch.as_bytes());
    write_field(&mut bytes, 2, WireType::ObjectId, patch.as_bytes());
    assert!(RecognitionClaimPayload::decode_canonical(&bytes).is_err());
}

#[test]
fn recognition_claim_payload_rejects_empty_patch_ids() {
    let block = ObjectId::from_canonical_payload(ObjectType::Block, 2, b"block");
    let payload = RecognitionClaimPayload {
        block_id: block,
        patch_ids: Vec::new(),
    };
    assert!(payload.to_canonical_bytes().is_err());
}

#[allow(clippy::expect_used)]
#[test]
fn recognition_claim_payload_rejects_unknown_field_tag() {
    let block = ObjectId::from_canonical_payload(ObjectType::Block, 2, b"block");
    let patch = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"patch");
    let payload = RecognitionClaimPayload {
        block_id: block,
        patch_ids: vec![patch],
    };
    let mut bytes = payload.to_canonical_bytes().expect("payload must encode");
    // Append one well-formed but unrecognized field (tag 99, empty string) after the real fields.
    bytes.extend_from_slice(&99_u16.to_be_bytes());
    bytes.push(WireType::String as u8);
    bytes.extend_from_slice(&0_u64.to_be_bytes());
    assert!(RecognitionClaimPayload::decode_canonical(&bytes).is_err());
}

#[allow(clippy::expect_used)]
#[test]
fn recognition_claim_payload_rejects_patch_ids_over_the_declared_limit() {
    let block = ObjectId::from_canonical_payload(ObjectType::Block, 2, b"block");
    // RECOGNITION_CLAIM_MAX_PATCH_IDS + 1 strictly ascending patch ids -- exceeds the bound by
    // exactly one, so decode must refuse at the boundary, per §7 row 6.
    let mut patch_ids = Vec::with_capacity(RECOGNITION_CLAIM_MAX_PATCH_IDS + 1);
    for index in 0..=RECOGNITION_CLAIM_MAX_PATCH_IDS {
        let mut seed = [0_u8; 32];
        seed[..8].copy_from_slice(&(index as u64).to_be_bytes());
        patch_ids.push(ObjectId::from_bytes(seed));
    }
    patch_ids.sort();
    let payload = RecognitionClaimPayload {
        block_id: block,
        patch_ids,
    };
    let bytes = payload
        .to_canonical_bytes()
        .expect("over-limit payload must still encode -- the bound is enforced on decode");
    let result = RecognitionClaimPayload::decode_canonical(&bytes);
    assert!(result.is_err(), "decode must refuse over-limit patch_ids");
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
