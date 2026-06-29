//! Hard FDD identity vectors — normative, hand-pinned, never generated.
//!
//! These assert literal ObjectId outputs from the ratified FDD-03. They are the
//! regression floor: a later reconciliation phase that alters identity bytes the
//! FDD fixes must break a test here, not silently re-bless a snapshot.

use crate::id::{ObjectId, ObjectType};

/// FDD-03 §4.1 golden vector — the empty-PATCH ObjectId. An empty payload has no
/// fields, so codec/type/payload changes in later phases must leave it unchanged.
/// This is THE anchor for the entire DC-09 reconciliation.
#[test]
fn empty_patch_anchor_matches_fdd_golden_vector() {
    let id = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"");
    assert_eq!(
        id.to_hex(),
        "510ab866a195347da66cada7fcb724a5ed77c4b85cf57345db169324e55d5157",
        "FDD-03 §4.1 empty-PATCH anchor changed — a reconciliation phase altered \
         identity bytes that must stay stable",
    );
}

/// Non-empty single-payload vector (originally inline in `id.rs`; pinned here as
/// the hard home for identity vectors).
#[test]
fn patch_payload_vector_is_stable() {
    let id = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"payload");
    assert_eq!(
        id.to_hex(),
        "5f8711b3f84991d60b65221d66ed5ec260d28cc19c5c4ed3c1fe44d334265fe6",
    );
}

/// FDD-03 §3 object type codes. The type code is part of the ObjectId preimage,
/// so these are identity bytes — pinned as hard normative values and round-tripped.
#[test]
fn object_type_codes_match_fdd_section_3() {
    let expected: &[(ObjectType, u16)] = &[
        (ObjectType::Patch, 0x01),
        (ObjectType::Block, 0x02),
        (ObjectType::RefState, 0x03),
        (ObjectType::RefUpdate, 0x04),
        (ObjectType::Tag, 0x05),
        (ObjectType::Attestation, 0x06),
        (ObjectType::Blob, 0x07),
        (ObjectType::BlockSummaryCache, 0x08),
        (ObjectType::RecoveryNote, 0x09),
        (ObjectType::ProjectGenesis, 0x0A),
    ];
    for &(ty, code) in expected {
        assert_eq!(ty.code(), code, "{ty} has wrong FDD-03 §3 code");
        assert_eq!(
            ObjectType::from_code(code),
            Ok(ty),
            "from_code({code:#04x}) did not round-trip",
        );
    }
}

/// FDD-03 §7.1 value_type codes. The value_type byte is in every field record and
/// thus part of object identity — pinned here as hard normative values.
#[test]
fn value_type_codes_match_fdd_section_7_1() {
    use crate::canonical::WireType;
    assert_eq!(WireType::Bool as u8, 0x01);
    assert_eq!(WireType::U16 as u8, 0x02);
    assert_eq!(WireType::U32 as u8, 0x03);
    assert_eq!(WireType::U64 as u8, 0x04);
    assert_eq!(WireType::EnumU16 as u8, 0x05);
    assert_eq!(WireType::String as u8, 0x10);
    assert_eq!(WireType::Bytes as u8, 0x11);
    assert_eq!(WireType::ObjectId as u8, 0x12);
    assert_eq!(WireType::RepoPath as u8, 0x13);
    assert_eq!(WireType::Record as u8, 0x20);
    assert_eq!(WireType::RecordListItem as u8, 0x21);
}

/// Cross-check: a codec-built payload (bool/u16/u32/utf8/bytes) hashes to a value
/// independently computed from the FDD §7.1 codes. Catches an emitter wired to the
/// wrong value_type even when `value_type_codes_match` passes.
#[test]
fn codec_sample_object_id_is_stable() {
    let id = ObjectId::from_canonical_payload(ObjectType::Patch, 1, &super::codec_sample_payload());
    assert_eq!(
        id.to_hex(),
        "dd9de6141b827ded0e87646afc1344716adaa0215722f776fb8aaa10c1c43749",
    );
}

/// Minimal top-level TLV walker for byte-level structural assertions: returns the
/// `(tag, value_type)` of each top-level field in order. Test-only.
// test helper: payloads under test are well-formed by construction.
#[allow(clippy::indexing_slicing, clippy::expect_used)]
fn top_level_tag_types(payload: &[u8]) -> Vec<(u16, u8)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < payload.len() {
        let tag = u16::from_be_bytes([payload[i], payload[i + 1]]);
        let value_type = payload[i + 2];
        let len =
            u64::from_be_bytes(payload[i + 3..i + 11].try_into().expect("8-byte length")) as usize;
        out.push((tag, value_type));
        i += 11 + len;
    }
    out
}

/// FDD-03 §9.5: RefStatePayload field tags AND value types, asserted at the byte
/// level. Round-trip tests cannot catch a wrong-but-self-consistent layout; this
/// pins the exact on-wire sequence against the ratified table.
#[test]
fn refstate_field_layout_matches_fdd_section_9_5() {
    // bool 0x01, u32 0x03, u64 0x04, enum_u16 0x05, utf8 0x10, bytes 0x11,
    // object_id 0x12, record_list_item 0x21
    let got = top_level_tag_types(&super::refs_populated_payload());
    let want = vec![
        (1, 0x10), // ref_name              utf8
        (2, 0x12), // target_object_id      object_id
        (3, 0x04), // update_seq            u64
        (4, 0x12), // previous_ref_state_id object_id
        (5, 0x12), // required_attestation_ids object_id
        (6, 0x05), // ref_kind              enum_u16
    ];
    assert_eq!(got, want, "RefState layout drifted from FDD-03 §9.5");
}

/// FDD-03 §9.9: AttestationPayload field tags AND value types, byte level.
#[test]
fn attestation_field_layout_matches_fdd_section_9_9() {
    let got = top_level_tag_types(&super::attestation_populated_payload());
    let want = vec![
        (1, 0x12), // target_block_id          object_id
        (2, 0x10), // policy_version           utf8
        (3, 0x11), // plugin_set_hash          bytes
        (4, 0x21), // results                  record_list_item
        (5, 0x05), // status                   enum_u16
        (6, 0x04), // created_at               u64
        (7, 0x01), // is_reproducible_offline  bool
    ];
    assert_eq!(got, want, "Attestation layout drifted from FDD-03 §9.9");
}

/// FDD-03 §9.10: the single PluginResultEntry nested inside the results list has
/// the correct field tags/types. Walks into the tag-4 record_list_item value.
#[allow(clippy::indexing_slicing, clippy::expect_used)]
#[test]
fn plugin_result_entry_layout_matches_fdd_section_9_10() {
    let payload = super::attestation_populated_payload();
    // find tag 4 (results, record_list_item) and walk its nested record bytes
    let mut i = 0;
    let mut nested = None;
    while i < payload.len() {
        let tag = u16::from_be_bytes([payload[i], payload[i + 1]]);
        let len = u64::from_be_bytes(payload[i + 3..i + 11].try_into().expect("len")) as usize;
        if tag == 4 {
            nested = Some(payload[i + 11..i + 11 + len].to_vec());
            break;
        }
        i += 11 + len;
    }
    let nested = nested.expect("results entry present");
    let got = top_level_tag_types(&nested);
    let want = vec![
        (1, 0x10), // plugin_id      utf8
        (2, 0x10), // plugin_version utf8
        (3, 0x05), // status         enum_u16
        (4, 0x11), // report_hash    bytes
        (5, 0x03), // finding_count  u32
    ];
    assert_eq!(
        got, want,
        "PluginResultEntry layout drifted from FDD-03 §9.10"
    );
}

/// FDD-03 §9.11: BlobPayload field tags AND value types, byte level.
#[test]
fn blob_field_layout_matches_fdd_section_9_11() {
    let got = top_level_tag_types(&super::blob_populated_payload());
    let want = vec![
        (1, 0x05), // blob_kind     enum_u16
        (2, 0x11), // content       bytes
        (3, 0x04), // declared_size u64
    ];
    assert_eq!(got, want, "Blob layout drifted from FDD-03 §9.11");
}

/// FDD-03 §9.1 tag 1: `PatchPayload.operations` MUST be framed as
/// `record_list_item` (0x21), not `record` (0x20). An empty patch cannot witness
/// this; the populated vector carries one operation. The pinned id is an
/// out-of-band cross-check on the list framing (independent of operation
/// internals, which are reconciled in later 4.2 increments).
#[test]
fn patch_operations_field_uses_record_list_item() {
    let got = top_level_tag_types(&super::patch_operations_populated_payload());
    assert_eq!(
        got,
        vec![(1, 0x21)],
        "PatchPayload.operations must use record_list_item (FDD-03 §9.1 tag 1)"
    );
    let id = ObjectId::from_canonical_payload(
        ObjectType::Patch,
        1,
        &super::patch_operations_populated_payload(),
    );
    assert_eq!(
        id.to_hex(),
        "24031b48ef9b5d1a7bdd31fda720c549a727ba9af774c59c8b5278f6c2bcc854",
        "populated-patch identity drifted"
    );
}

/// FDD-03 §9.3 CreateFile nested-record field tags AND value types, pinned at the
/// byte level: repo_path, node_id bytes, blob_id object_id, mode u32. Round-trip
/// tests cannot catch a wrong-but-self-consistent layout; this fixes the wire.
#[test]
#[allow(clippy::expect_used)]
fn create_file_record_layout_matches_fdd_section_9_3() {
    use crate::CanonicalEncode;
    let op = crate::CreateFile {
        path: "a.txt".to_string(),
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        blob_id: ObjectId::from_bytes([0x11; 32]),
        mode: 0o100_644,
    };
    let got = top_level_tag_types(&op.to_canonical_bytes().expect("CreateFile encodes"));
    assert_eq!(
        got,
        vec![(1, 0x13), (2, 0x11), (3, 0x12), (4, 0x03)],
        "CreateFile §9.3 field layout drifted"
    );
}

/// FDD-03 §9.3 DeleteNode (file preimage) field layout: repo_path, node_id bytes,
/// old_node_kind enum_u16, old_blob_id object_id, old_mode u32 (no old_target).
#[test]
#[allow(clippy::expect_used)]
fn delete_node_file_record_layout_matches_fdd_section_9_3() {
    use crate::CanonicalEncode;
    let op = crate::DeleteNode {
        path: "a.txt".to_string(),
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        old_node_kind: crate::NodeKind::TextFile,
        preimage: crate::DeleteNodePreimage::File {
            old_blob_id: ObjectId::from_bytes([0x11; 32]),
            old_mode: 0o100_644,
        },
    };
    let got = top_level_tag_types(&op.to_canonical_bytes().expect("DeleteNode encodes"));
    assert_eq!(
        got,
        vec![(1, 0x13), (2, 0x11), (3, 0x05), (4, 0x12), (6, 0x03)],
        "DeleteNode file §9.3 field layout drifted"
    );
}

/// FDD-03 §9.3 DeleteNode (symlink preimage) field layout: repo_path, node_id
/// bytes, old_node_kind enum_u16, old_target utf8 (no old_blob_id/old_mode).
#[test]
#[allow(clippy::expect_used)]
fn delete_node_symlink_record_layout_matches_fdd_section_9_3() {
    use crate::CanonicalEncode;
    let op = crate::DeleteNode {
        path: "link".to_string(),
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        old_node_kind: crate::NodeKind::Symlink,
        preimage: crate::DeleteNodePreimage::Symlink {
            old_target: "target.txt".to_string(),
        },
    };
    let got = top_level_tag_types(&op.to_canonical_bytes().expect("DeleteNode symlink encodes"));
    assert_eq!(
        got,
        vec![(1, 0x13), (2, 0x11), (3, 0x05), (5, 0x10)],
        "DeleteNode symlink §9.3 field layout drifted"
    );
}

/// FDD-03 §9.3: a DeleteNode whose `old_node_kind` disagrees with its preimage
/// discriminator must be rejected by both `validate()` and the canonical encoder.
#[test]
fn delete_node_rejects_kind_preimage_mismatch() {
    use crate::CanonicalEncode;
    let text_with_symlink = crate::DeleteNode {
        path: "a".to_string(),
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        old_node_kind: crate::NodeKind::TextFile,
        preimage: crate::DeleteNodePreimage::Symlink {
            old_target: "t".to_string(),
        },
    };
    assert!(text_with_symlink.validate().is_err());
    assert!(text_with_symlink.to_canonical_bytes().is_err());

    let symlink_with_file = crate::DeleteNode {
        path: "a".to_string(),
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        old_node_kind: crate::NodeKind::Symlink,
        preimage: crate::DeleteNodePreimage::File {
            old_blob_id: ObjectId::from_bytes([0x11; 32]),
            old_mode: 0o100_644,
        },
    };
    assert!(symlink_with_file.validate().is_err());
    assert!(symlink_with_file.to_canonical_bytes().is_err());
}

/// FDD-03 §9.3 node-id validator: CreateFile and DeleteNode with an all-zero
/// node_id must fail both `validate()` and the canonical encoder, so the reserved
/// value can never reach persistent identity bytes.
#[test]
fn create_file_rejects_all_zero_node_id() {
    use crate::CanonicalEncode;
    let op = crate::CreateFile {
        path: "a.txt".to_string(),
        node_id: crate::NodeId::from_bytes([0x00; 32]),
        blob_id: ObjectId::from_bytes([0x11; 32]),
        mode: 0o100_644,
    };
    assert!(op.validate().is_err());
    assert!(op.to_canonical_bytes().is_err());
}

#[test]
fn delete_node_rejects_all_zero_node_id() {
    use crate::CanonicalEncode;
    let op = crate::DeleteNode {
        path: "a.txt".to_string(),
        node_id: crate::NodeId::from_bytes([0x00; 32]),
        old_node_kind: crate::NodeKind::TextFile,
        preimage: crate::DeleteNodePreimage::File {
            old_blob_id: ObjectId::from_bytes([0x11; 32]),
            old_mode: 0o100_644,
        },
    };
    assert!(op.validate().is_err());
    assert!(op.to_canonical_bytes().is_err());
}

/// FDD-03 §9.3 EditText field layout (no presentation hints), pinned at the byte
/// level: node_id, span_id, old_span_hash, left/right anchor hashes, replacement_text,
/// old_span_text — all `bytes` (0x11). Round-trip tests cannot catch a
/// wrong-but-self-consistent layout; this fixes the wire.
#[test]
#[allow(clippy::expect_used)]
fn edit_text_record_layout_matches_fdd_section_9_3() {
    use crate::CanonicalEncode;
    let op = crate::EditText {
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        span_id: [0x10; 32],
        old_span_hash: crate::text_span_hash(b"old"),
        left_anchor_hash: [0x11; 32],
        right_anchor_hash: [0x12; 32],
        replacement_text: b"new".to_vec(),
        presentation_hint_line: None,
        presentation_hint_column: None,
        old_span_text: b"old".to_vec(),
    };
    let got = top_level_tag_types(&op.to_canonical_bytes().expect("EditText encodes"));
    assert_eq!(
        got,
        vec![
            (1, 0x11),
            (2, 0x11),
            (3, 0x11),
            (4, 0x11),
            (5, 0x11),
            (6, 0x11),
            (9, 0x11)
        ],
        "EditText §9.3 field layout drifted"
    );
}

/// FDD-03 §9.3 EditText with the optional presentation hints present: tags 7/8 are
/// `u32` (0x03) and sit between replacement_text (6) and old_span_text (9).
#[test]
#[allow(clippy::expect_used)]
fn edit_text_record_layout_with_hints() {
    use crate::CanonicalEncode;
    let op = crate::EditText {
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        span_id: [0x10; 32],
        old_span_hash: crate::text_span_hash(b"old"),
        left_anchor_hash: [0x11; 32],
        right_anchor_hash: [0x12; 32],
        replacement_text: b"new".to_vec(),
        presentation_hint_line: Some(12),
        presentation_hint_column: Some(4),
        old_span_text: b"old".to_vec(),
    };
    let got = top_level_tag_types(&op.to_canonical_bytes().expect("EditText encodes"));
    assert_eq!(
        got,
        vec![
            (1, 0x11),
            (2, 0x11),
            (3, 0x11),
            (4, 0x11),
            (5, 0x11),
            (6, 0x11),
            (7, 0x03),
            (8, 0x03),
            (9, 0x11)
        ],
        "EditText §9.3 hinted field layout drifted"
    );
}

/// FDD-03 §9.3 EditText validators: reject all-zero node_id, an old_span_hash that
/// is not SHA-256(old_span_text), and non-UTF-8 span text — on both `validate()`
/// and the canonical encoder.
#[test]
fn edit_text_validators_reject_malformed_records() {
    use crate::CanonicalEncode;
    let base = crate::EditText {
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        span_id: [0x10; 32],
        old_span_hash: crate::text_span_hash(b"old"),
        left_anchor_hash: [0x11; 32],
        right_anchor_hash: [0x12; 32],
        replacement_text: b"new".to_vec(),
        presentation_hint_line: None,
        presentation_hint_column: None,
        old_span_text: b"old".to_vec(),
    };
    // sanity: the base record is valid
    assert!(base.validate().is_ok());

    let zero_node = crate::EditText {
        node_id: crate::NodeId::from_bytes([0x00; 32]),
        ..base.clone()
    };
    assert!(zero_node.validate().is_err());
    assert!(zero_node.to_canonical_bytes().is_err());

    let bad_hash = crate::EditText {
        old_span_hash: [0x00; 32],
        ..base.clone()
    };
    assert!(bad_hash.validate().is_err());
    assert!(bad_hash.to_canonical_bytes().is_err());

    let bad_old_utf8 = crate::EditText {
        old_span_text: vec![0xff, 0xfe],
        old_span_hash: crate::text_span_hash(&[0xff, 0xfe]),
        ..base.clone()
    };
    assert!(bad_old_utf8.validate().is_err());
    assert!(bad_old_utf8.to_canonical_bytes().is_err());

    let bad_new_utf8 = crate::EditText {
        replacement_text: vec![0xff, 0xfe],
        ..base.clone()
    };
    assert!(bad_new_utf8.validate().is_err());
    assert!(bad_new_utf8.to_canonical_bytes().is_err());
}

/// FDD-03 §9.3 ReplaceBinary field layout (node-addressed): node_id bytes,
/// old_blob_id object_id, new_blob_id object_id. No path, no mode.
#[test]
#[allow(clippy::expect_used)]
fn replace_binary_record_layout_matches_fdd_section_9_3() {
    use crate::CanonicalEncode;
    let op = crate::ReplaceBinary {
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        old_blob_id: ObjectId::from_bytes([0x11; 32]),
        new_blob_id: ObjectId::from_bytes([0x33; 32]),
    };
    let got = top_level_tag_types(&op.to_canonical_bytes().expect("ReplaceBinary encodes"));
    assert_eq!(
        got,
        vec![(1, 0x11), (2, 0x12), (3, 0x12)],
        "ReplaceBinary §9.3 field layout drifted"
    );
}

/// FDD-03 §9.3: the reserved all-zero node_id must be rejected by ReplaceBinary's
/// validate() and the canonical encoder, so it can never reach identity bytes.
#[test]
fn replace_binary_rejects_all_zero_node_id() {
    use crate::CanonicalEncode;
    let op = crate::ReplaceBinary {
        node_id: crate::NodeId::from_bytes([0x00; 32]),
        old_blob_id: ObjectId::from_bytes([0x11; 32]),
        new_blob_id: ObjectId::from_bytes([0x33; 32]),
    };
    assert!(op.validate().is_err());
    assert!(op.to_canonical_bytes().is_err());
}

/// FDD-03 §9.3 RenamePath field layout: node_id bytes, old_path/new_path repo_path.
#[test]
#[allow(clippy::expect_used)]
fn rename_path_record_layout_matches_fdd_section_9_3() {
    use crate::CanonicalEncode;
    let op = crate::RenamePath {
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        old_path: "a.txt".to_string(),
        new_path: "b.txt".to_string(),
    };
    let got = top_level_tag_types(&op.to_canonical_bytes().expect("RenamePath encodes"));
    assert_eq!(
        got,
        vec![(1, 0x11), (2, 0x13), (3, 0x13)],
        "RenamePath §9.3 field layout drifted"
    );
}

#[test]
fn rename_path_rejects_all_zero_node_id() {
    use crate::CanonicalEncode;
    let op = crate::RenamePath {
        node_id: crate::NodeId::from_bytes([0x00; 32]),
        old_path: "a.txt".to_string(),
        new_path: "b.txt".to_string(),
    };
    assert!(op.validate().is_err());
    assert!(op.to_canonical_bytes().is_err());
}

/// FDD-03 §9.3 ChangePerm field layout: node_id bytes, old_mode/new_mode u32.
#[test]
#[allow(clippy::expect_used)]
fn change_perm_record_layout_matches_fdd_section_9_3() {
    use crate::CanonicalEncode;
    let op = crate::ChangePerm {
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        old_mode: 0o100_644,
        new_mode: 0o100_755,
    };
    let got = top_level_tag_types(&op.to_canonical_bytes().expect("ChangePerm encodes"));
    assert_eq!(
        got,
        vec![(1, 0x11), (2, 0x03), (3, 0x03)],
        "ChangePerm §9.3 field layout drifted"
    );
}

#[test]
fn change_perm_rejects_all_zero_node_id() {
    use crate::CanonicalEncode;
    let op = crate::ChangePerm {
        node_id: crate::NodeId::from_bytes([0x00; 32]),
        old_mode: 0o100_644,
        new_mode: 0o100_755,
    };
    assert!(op.validate().is_err());
    assert!(op.to_canonical_bytes().is_err());
}

/// FDD-03 §9.3 CreateSymlink field layout: path repo_path (1), node_id bytes (2),
/// target utf8_string (3).
#[test]
#[allow(clippy::expect_used)]
fn create_symlink_record_layout_matches_fdd_section_9_3() {
    use crate::CanonicalEncode;
    let op = crate::CreateSymlink {
        path: "link".to_string(),
        node_id: crate::NodeId::from_bytes([0x22; 32]),
        target: "target.txt".to_string(),
    };
    let got = top_level_tag_types(&op.to_canonical_bytes().expect("CreateSymlink encodes"));
    assert_eq!(
        got,
        vec![(1, 0x13), (2, 0x11), (3, 0x10)],
        "CreateSymlink §9.3 field layout drifted"
    );
}

#[test]
fn create_symlink_rejects_all_zero_node_id() {
    use crate::CanonicalEncode;
    let op = crate::CreateSymlink {
        path: "link".to_string(),
        node_id: crate::NodeId::from_bytes([0x00; 32]),
        target: "target.txt".to_string(),
    };
    assert!(op.validate().is_err());
    assert!(op.to_canonical_bytes().is_err());
}
