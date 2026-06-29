//! Blob role validation tests (FDD-03 §9.3.0).
#![allow(clippy::expect_used)]

use prikk_object::{BlobKind, BlobPayload, CanonicalEncode};

use crate::blob_access::{
    decode_file_content_blob, decode_snapshot_blob, ensure_blob_kind_is_binary,
};

fn encode(kind: BlobKind, content: &[u8]) -> Vec<u8> {
    BlobPayload::new(kind, content.to_vec())
        .to_canonical_bytes()
        .expect("blob encodes")
}

#[test]
fn snapshot_loader_accepts_snapshot_blob() {
    let bytes = encode(BlobKind::Snapshot, b"manifest");
    assert_eq!(
        decode_snapshot_blob(&bytes).expect("snapshot ok"),
        b"manifest"
    );
}

#[test]
fn snapshot_loader_rejects_text_blob() {
    let bytes = encode(BlobKind::Text, b"x");
    assert!(
        decode_snapshot_blob(&bytes).is_err(),
        "snapshot ref to a TEXT blob must be rejected"
    );
}

#[test]
fn snapshot_loader_rejects_binary_blob() {
    let bytes = encode(BlobKind::Binary, b"x");
    assert!(
        decode_snapshot_blob(&bytes).is_err(),
        "snapshot ref to a BINARY blob must be rejected"
    );
}

#[test]
fn file_content_reader_rejects_snapshot_blob() {
    let bytes = encode(BlobKind::Snapshot, b"x");
    assert!(
        decode_file_content_blob(&bytes).is_err(),
        "file content ref to a SNAPSHOT blob must be rejected"
    );
}

#[test]
fn file_content_reader_accepts_text_and_binary() {
    let text = encode(BlobKind::Text, b"hello");
    let binary = encode(BlobKind::Binary, b"\x00\x01\x02");
    assert_eq!(decode_file_content_blob(&text).expect("text ok"), b"hello");
    assert_eq!(
        decode_file_content_blob(&binary).expect("binary ok"),
        b"\x00\x01\x02"
    );
}

/// Compute the Blob ObjectId for `content` under `kind`, as the store would.
fn blob_id(kind: BlobKind, content: &[u8]) -> prikk_object::ObjectId {
    prikk_object::ObjectId::from_canonical_payload(
        prikk_object::ObjectType::Blob,
        1,
        &encode(kind, content),
    )
}

#[test]
fn node_kind_blob_match_accepts_text_file() {
    use crate::blob_access::ensure_blob_matches_node_kind;
    let id = blob_id(BlobKind::Text, b"hello\n");
    assert!(
        ensure_blob_matches_node_kind(b"hello\n", id, prikk_object::NodeKind::TextFile).is_ok()
    );
}

#[test]
fn node_kind_blob_match_accepts_binary_file() {
    use crate::blob_access::ensure_blob_matches_node_kind;
    let content: &[u8] = &[0x00, 0x01, 0xff];
    let id = blob_id(BlobKind::Binary, content);
    assert!(ensure_blob_matches_node_kind(content, id, prikk_object::NodeKind::BinaryFile).is_ok());
}

#[test]
fn node_kind_blob_match_rejects_text_kind_over_binary_blob() {
    use crate::blob_access::ensure_blob_matches_node_kind;
    let content: &[u8] = &[0x00, 0x01, 0xff];
    let binary = blob_id(BlobKind::Binary, content);
    // old_node_kind says TextFile but old_blob_id is a binary blob.
    assert!(
        ensure_blob_matches_node_kind(content, binary, prikk_object::NodeKind::TextFile).is_err()
    );
}

#[test]
fn node_kind_blob_match_rejects_binary_kind_over_text_blob() {
    use crate::blob_access::ensure_blob_matches_node_kind;
    let text = blob_id(BlobKind::Text, b"hello\n");
    assert!(
        ensure_blob_matches_node_kind(b"hello\n", text, prikk_object::NodeKind::BinaryFile)
            .is_err()
    );
}

#[test]
fn node_kind_blob_match_rejects_snapshot_blob_id() {
    use crate::blob_access::ensure_blob_matches_node_kind;
    // old_blob_id identifies a snapshot blob; a file-content recompute cannot match.
    let snapshot = blob_id(BlobKind::Snapshot, b"hello\n");
    assert!(
        ensure_blob_matches_node_kind(b"hello\n", snapshot, prikk_object::NodeKind::TextFile)
            .is_err()
    );
}

#[test]
fn node_kind_blob_match_rejects_symlink_kind() {
    use crate::blob_access::ensure_blob_matches_node_kind;
    let id = blob_id(BlobKind::Text, b"hello\n");
    assert!(
        ensure_blob_matches_node_kind(b"hello\n", id, prikk_object::NodeKind::Symlink).is_err()
    );
}
#[test]
fn binary_kind_check_accepts_binary_blob() {
    // FDD-03 §9.3 ReplaceBinary operates on binary nodes only.
    let bytes = encode(BlobKind::Binary, b"\x00\x01\x02");
    assert!(ensure_blob_kind_is_binary(&bytes).is_ok());
}

#[test]
fn binary_kind_check_rejects_text_blob() {
    // text->binary transition on the ReplaceBinary path must fail closed.
    let bytes = encode(BlobKind::Text, b"hello");
    assert!(
        ensure_blob_kind_is_binary(&bytes).is_err(),
        "ReplaceBinary must reject a TEXT blob"
    );
}

#[test]
fn binary_kind_check_rejects_snapshot_blob() {
    let bytes = encode(BlobKind::Snapshot, b"manifest");
    assert!(
        ensure_blob_kind_is_binary(&bytes).is_err(),
        "ReplaceBinary must reject a SNAPSHOT blob"
    );
}
