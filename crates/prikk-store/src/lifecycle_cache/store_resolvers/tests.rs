#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use super::StoreBackedResolver;
use crate::lifecycle_cache::{BlobKindResolver, BlockParentResolver};
use crate::memory_store::MemoryObjectStore;
use crate::object_store::ObjectWriter;
use prikk_object::{
    BlobKind, BlobPayload, BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope,
    ObjectId, ObjectType,
};

fn block_envelope(parents: &[ObjectId], kind: BlockKind) -> ObjectEnvelope {
    let payload = BlockPayload {
        parent_block_ids: parents.to_vec(),
        kind,
        patch_ids: Vec::new(),
        state_merkle_root: MerkleRoot([0_u8; 32]),
        snapshot_blob_ref: None,
        mainline_parent_id: None,
        merge_baseline_block_id: None,
    };
    ObjectEnvelope::unsigned(
        ObjectType::Block,
        1,
        payload.to_canonical_bytes().expect("encode block"),
    )
}

fn blob_envelope(kind: BlobKind) -> ObjectEnvelope {
    let payload = BlobPayload::new(kind, b"content".to_vec());
    ObjectEnvelope::unsigned(
        ObjectType::Blob,
        1,
        payload.to_canonical_bytes().expect("encode blob"),
    )
}

fn write(store: &mut MemoryObjectStore, env: &ObjectEnvelope) -> ObjectId {
    store.write_object(env).expect("write object")
}

#[test]
fn genesis_block_resolves_to_zero_parents() {
    let mut store = MemoryObjectStore::new();
    let id = write(&mut store, &block_envelope(&[], BlockKind::Root));

    let resolver = StoreBackedResolver::new(&store);
    let parents = resolver.parent_block_ids(&id).expect("resolve genesis");
    assert!(
        parents.is_empty(),
        "a decoded block with zero parents is genesis"
    );
}

#[test]
fn block_resolves_to_its_parent_ids() {
    let mut store = MemoryObjectStore::new();
    // Two distinct, strictly-sorted parent ids (need not themselves exist: the resolver
    // returns the field verbatim and does not recurse).
    let p1 = ObjectId::from_bytes([1_u8; 32]);
    let p2 = ObjectId::from_bytes([2_u8; 32]);
    let id = write(&mut store, &block_envelope(&[p1, p2], BlockKind::Normal));

    let resolver = StoreBackedResolver::new(&store);
    let parents = resolver.parent_block_ids(&id).expect("resolve parents");
    assert_eq!(parents, vec![p1, p2]);
}

#[test]
fn missing_block_is_an_error_not_genesis() {
    // P2-1: an absent block must error, never be read as a zero-parent genesis.
    let store = MemoryObjectStore::new();
    let absent = ObjectId::from_bytes([9_u8; 32]);

    let resolver = StoreBackedResolver::new(&store);
    let err = resolver
        .parent_block_ids(&absent)
        .expect_err("missing block must error");
    assert!(
        err.to_string().contains("missing")
            && err.to_string().contains("cannot be treated as genesis"),
        "unexpected error: {err}"
    );
}

#[test]
fn wrong_type_for_block_is_an_error() {
    // An object that exists but is not a Block must error (not be mistaken for genesis).
    let mut store = MemoryObjectStore::new();
    let blob_id = write(&mut store, &blob_envelope(BlobKind::Binary));

    let resolver = StoreBackedResolver::new(&store);
    let err = resolver
        .parent_block_ids(&blob_id)
        .expect_err("blob resolved as block must error");
    assert!(err.to_string().contains("not a Block"), "got: {err}");
}

#[test]
fn present_blob_resolves_to_its_kind() {
    let mut store = MemoryObjectStore::new();
    let id = write(&mut store, &blob_envelope(BlobKind::Binary));

    let resolver = StoreBackedResolver::new(&store);
    let kind = resolver.blob_kind(&id).expect("resolve blob kind");
    assert_eq!(kind, Some(BlobKind::Binary));
}

#[test]
fn missing_blob_is_fail_closed_none() {
    let store = MemoryObjectStore::new();
    let absent = ObjectId::from_bytes([7_u8; 32]);

    let resolver = StoreBackedResolver::new(&store);
    let kind = resolver
        .blob_kind(&absent)
        .expect("missing blob is Ok(None)");
    assert_eq!(kind, None, "absent blob is the fail-closed sentinel");
}

#[test]
fn wrong_type_for_blob_is_an_error() {
    // A present object that is not a Blob must error rather than yield a bogus kind.
    let mut store = MemoryObjectStore::new();
    let block_id = write(&mut store, &block_envelope(&[], BlockKind::Root));

    let resolver = StoreBackedResolver::new(&store);
    let err = resolver
        .blob_kind(&block_id)
        .expect_err("block resolved as blob must error");
    assert!(err.to_string().contains("not a Blob"), "got: {err}");
}
