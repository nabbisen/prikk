//! Patch-schema-2 handoff (v2 amendment) §5 items 1-2: `PATCH_PARENT_IDS_RETIRED_SCHEMA` retires
//! tag 2 (`parent_patch_ids`) at schema 2 and above, while schema 1 -- every patch already written
//! -- must keep decoding it unchanged. `PatchPayload` no longer has the field at all, so every
//! fixture here bypasses it and writes tag 2 directly with `CanonicalWriter`, exactly where
//! `encode_canonical` used to emit it, to construct the one shape no production code can author
//! anymore.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use prikk_object::{
    CanonicalWriter, CreateFile, NodeId, ObjectId, Operation, OperationKind,
    PATCH_PARENT_IDS_RETIRED_SCHEMA,
};

use crate::patch_replay::decode::{DecodedOperationKind, decode_patch_operations};

fn one_create_file_operation() -> Operation {
    Operation {
        op_seq: 1,
        op_id: None,
        preconditions: Vec::new(),
        kind: OperationKind::CreateFile(CreateFile {
            path: "schema2.txt".to_string(),
            node_id: NodeId::from_bytes([0x9c; 32]),
            blob_id: ObjectId::from_bytes([0x9d; 32]),
            mode: 0o100_644,
        }),
    }
}

fn patch_bytes_with_raw_tag2(parent_patch_ids: &[ObjectId]) -> Vec<u8> {
    let mut writer = CanonicalWriter::new();
    writer
        .repeated_record_list(1, &[one_create_file_operation()])
        .expect("operations record list encodes");
    writer
        .repeated_object_id(2, parent_patch_ids)
        .expect("parent_patch_ids record encodes");
    writer.finish()
}

/// §5 item 1, "the sharpest of them" per the v2 amendment §4: a schema-1 patch carrying a
/// non-empty tag 2 still decodes -- and to the identical operations as the same patch without
/// it -- because schema 1's tag-2 handling predates this schema existing at all and must stay
/// unchanged for every patch already written.
#[test]
fn schema_1_patch_with_field_2_present_decodes_unchanged() {
    let with_field_2 = patch_bytes_with_raw_tag2(&[ObjectId::from_bytes([0x9e; 32])]);
    let without_field_2 = patch_bytes_with_raw_tag2(&[]);

    let decoded_with = decode_patch_operations(&with_field_2, 1)
        .expect("schema 1 must still decode a non-empty tag 2 -- it is legal there");
    let decoded_without = decode_patch_operations(&without_field_2, 1)
        .expect("schema 1 decodes a patch with no tag 2 at all");

    assert_eq!(
        decoded_with, decoded_without,
        "tag 2's value must have zero effect on the decoded operations at schema 1, exactly as \
         before this schema existed"
    );
    assert_eq!(decoded_with.len(), 1);
    assert!(matches!(
        decoded_with[0].kind,
        DecodedOperationKind::CreateFile { .. }
    ));
}

/// §5 item 2: a schema-2 patch carrying tag 2 is refused outright -- the field is retired, not
/// merely ignored, at `PATCH_PARENT_IDS_RETIRED_SCHEMA` and above.
#[test]
fn schema_2_patch_with_field_2_present_is_refused() {
    let bytes = patch_bytes_with_raw_tag2(&[ObjectId::from_bytes([0x9e; 32])]);
    let error = decode_patch_operations(&bytes, PATCH_PARENT_IDS_RETIRED_SCHEMA)
        .expect_err("schema 2 must refuse a patch carrying tag 2");
    let message = error.to_string();
    assert!(
        message.contains("parent_patch_ids") && message.contains("retired"),
        "expected a retired-tag-2 refusal naming parent_patch_ids, got: {message}"
    );
}

/// Negative control for the two tests above: the refusal is specifically about tag 2's
/// *presence* at schema 2, not a blanket rejection of schema 2 itself -- the identical payload
/// with an empty (i.e. absent-on-the-wire, per `CanonicalWriter::repeated_object_id`'s own
/// empty-slice behaviour) tag 2 must decode at schema 2 exactly as it does at schema 1. Without
/// the schema-conditioned check in `decode_patch_operations`, `schema_2_patch_with_field_2_present_is_refused`
/// above would still need *something* to fail on, and a decoder that instead unconditionally
/// refused schema 2 outright would pass that test too while breaking every legitimate schema-2
/// patch -- this test is what rules that degenerate implementation out.
#[test]
fn schema_2_patch_without_field_2_decodes_normally() {
    let schema1_bytes = patch_bytes_with_raw_tag2(&[]);
    let schema2_bytes = patch_bytes_with_raw_tag2(&[]);
    assert_eq!(
        schema1_bytes, schema2_bytes,
        "bytes are schema-independent by construction here"
    );

    let decoded = decode_patch_operations(&schema2_bytes, PATCH_PARENT_IDS_RETIRED_SCHEMA)
        .expect("schema 2 must accept a patch that never carries tag 2 at all");
    assert_eq!(decoded.len(), 1);
    assert!(matches!(
        decoded[0].kind,
        DecodedOperationKind::CreateFile { .. }
    ));
}
