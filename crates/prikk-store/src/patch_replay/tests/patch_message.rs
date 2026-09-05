//! RFC 123 §8 controls: `PatchPayload.message` (tag 6, `WireType::String`) round-trips through
//! encode/decode, an explicitly encoded empty string is refused at decode (not only at encode),
//! and a schema below `PATCH_MESSAGE_SCHEMA` carrying tag 6 is refused outright -- the same
//! schema-gating shape `schema2_parent_ids_retired.rs` beside this file already established for
//! tag 2.

#![allow(clippy::expect_used)]

use prikk_object::{
    CanonicalEncode, CanonicalWriter, CreateFile, NodeId, ObjectId, Operation, OperationKind,
    PATCH_MESSAGE_SCHEMA, PATCH_TEXT_SPAN_V2_SCHEMA, PatchPayload, PatchPurpose,
};

use crate::patch_replay::decode::{decode_patch_message, decode_patch_operations};

fn one_create_file_operation() -> Operation {
    Operation {
        op_seq: 1,
        op_id: None,
        preconditions: Vec::new(),
        kind: OperationKind::CreateFile(CreateFile {
            path: "message.txt".to_string(),
            node_id: NodeId::from_bytes([0xa1; 32]),
            blob_id: ObjectId::from_bytes([0xa2; 32]),
            mode: 0o100_644,
        }),
    }
}

fn payload_with_message(message: Option<&str>) -> PatchPayload {
    PatchPayload {
        operations: vec![one_create_file_operation()],
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
        message: message.map(str::to_string),
    }
}

/// Control 1: round-trip for `Some` (including multibyte UTF-8 and an embedded newline) and for
/// `None`, through the real encoder and the real decoder -- not a hand-built fixture.
#[test]
fn message_round_trips_through_encode_and_decode() {
    let with_message = "fix: 日本語のバグを修正\nsecond line";
    let encoded = payload_with_message(Some(with_message))
        .to_canonical_bytes()
        .expect("Some(message) encodes");
    let decoded = decode_patch_message(&encoded, PATCH_MESSAGE_SCHEMA)
        .expect("a well-formed schema-4 message decodes");
    assert_eq!(decoded, Some(with_message.to_string()));

    let encoded_none = payload_with_message(None)
        .to_canonical_bytes()
        .expect("None encodes");
    let decoded_none = decode_patch_message(&encoded_none, PATCH_MESSAGE_SCHEMA)
        .expect("a payload with no message field decodes");
    assert_eq!(decoded_none, None);
}

/// Control 4 (identity): two patches identical but for their message have different ids -- the
/// whole claim of "message as evidence" in one assertion. Lives here, not in `prikk-object`, only
/// because this file already has the fixtures; the assertion itself needs no store-only API.
#[test]
fn message_text_changes_the_patch_id() {
    let with_a = payload_with_message(Some("message A"))
        .to_canonical_bytes()
        .expect("encodes");
    let with_b = payload_with_message(Some("message B"))
        .to_canonical_bytes()
        .expect("encodes");
    let id_a = ObjectId::from_canonical_payload(
        prikk_object::ObjectType::Patch,
        PATCH_MESSAGE_SCHEMA,
        &with_a,
    );
    let id_b = ObjectId::from_canonical_payload(
        prikk_object::ObjectType::Patch,
        PATCH_MESSAGE_SCHEMA,
        &with_b,
    );
    assert_ne!(
        id_a, id_b,
        "two patches identical but for message text must have different ids"
    );
}

/// Control 2: `Some("")` is refused at decode, not only at `PatchPayload::validate` on the write
/// side. Built with `CanonicalWriter` directly, bypassing `PatchPayload::encode_canonical` (which
/// cannot itself produce these bytes, since it calls `validate()` first) -- the same "construct
/// past the encoder" discipline `schema2_parent_ids_retired.rs` uses for tag 2.
#[test]
fn decode_refuses_an_explicitly_encoded_empty_message() {
    let mut writer = CanonicalWriter::new();
    writer
        .repeated_record_list(1, &[one_create_file_operation()])
        .expect("operations record list encodes");
    writer.field_string(6, "").expect("empty string encodes");
    let bytes = writer.finish();

    let error = decode_patch_message(&bytes, PATCH_MESSAGE_SCHEMA)
        .expect_err("an explicitly encoded empty message must be refused at decode");
    assert!(
        error.to_string().contains("empty"),
        "expected an empty-message refusal, got: {error}"
    );
}

/// Control 3: a schema-3 envelope carrying tag 6 must not decode -- checked through both decode
/// entry points that read tag 6's schema legality independently (`decode_patch_operations`,
/// `decode_patch_message`), since neither trusts the other to have already run.
#[test]
fn schema_3_patch_with_message_field_is_refused() {
    let bytes = payload_with_message(Some("not yet legal"))
        .to_canonical_bytes()
        .expect("encodes regardless of the schema it will be checked against");

    let operations_error = decode_patch_operations(&bytes, PATCH_TEXT_SPAN_V2_SCHEMA)
        .expect_err("schema 3 must refuse a patch carrying tag 6");
    assert!(
        operations_error.to_string().contains("message")
            && operations_error.to_string().contains("requires schema"),
        "expected a schema-gated message refusal, got: {operations_error}"
    );

    let message_error = decode_patch_message(&bytes, PATCH_TEXT_SPAN_V2_SCHEMA)
        .expect_err("schema 3 must refuse tag 6 here too, independently");
    assert!(
        message_error.to_string().contains("requires schema"),
        "expected a schema-gated message refusal, got: {message_error}"
    );
}

/// Negative control for the test above, mirroring `schema_2_patch_without_field_2_decodes_normally`
/// beside this file: the refusal is specifically about tag 6's *presence* below schema 4, not a
/// blanket rejection of schema 3 -- the identical payload with no message at all must decode fine
/// at schema 3.
#[test]
fn schema_3_patch_without_message_field_decodes_normally() {
    let bytes = payload_with_message(None)
        .to_canonical_bytes()
        .expect("encodes");
    let decoded = decode_patch_operations(&bytes, PATCH_TEXT_SPAN_V2_SCHEMA)
        .expect("schema 3 must accept a patch that never carries tag 6 at all");
    assert_eq!(decoded.len(), 1);
}
