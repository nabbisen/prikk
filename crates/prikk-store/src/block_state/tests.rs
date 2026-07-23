use prikk_object::{
    BlobKind, BlobPayload, BlockKind, BlockPayload, CanonicalEncode, CreateFile, MerkleRoot,
    NodeId, ObjectEnvelope, ObjectId, ObjectType, Operation, OperationKind, PatchPayload,
    PatchPurpose,
};

use super::{derive_next_state_root, validate_block_v2_shape, verify_block_v2_state};
use crate::{MemoryObjectStore, ObjectWriter, compute_state_root};

fn payload(kind: BlockKind, parents: Vec<ObjectId>, root: MerkleRoot) -> BlockPayload {
    BlockPayload {
        parent_block_ids: parents,
        kind,
        patch_ids: Vec::new(),
        state_merkle_root: root,
        snapshot_blob_ref: None,
    }
}

#[test]
fn format2_parent_and_kind_matrix_is_closed() -> prikk_error::Result<()> {
    let parent = ObjectId::from_bytes([1; 32]);
    let root = compute_state_root(&[])?;
    assert!(validate_block_v2_shape(&payload(BlockKind::Root, Vec::new(), root)).is_ok());
    assert!(validate_block_v2_shape(&payload(BlockKind::Normal, vec![parent], root)).is_ok());
    assert!(validate_block_v2_shape(&payload(BlockKind::Root, vec![parent], root)).is_err());
    assert!(validate_block_v2_shape(&payload(BlockKind::Normal, Vec::new(), root)).is_err());
    assert!(
        validate_block_v2_shape(&payload(BlockKind::Normal, vec![parent, parent], root)).is_err()
    );
    for kind in [BlockKind::Merge, BlockKind::Repair, BlockKind::Import] {
        assert!(validate_block_v2_shape(&payload(kind, Vec::new(), root)).is_err());
    }
    Ok(())
}

#[test]
fn verification_recomputes_empty_root_and_rejects_mismatch() -> prikk_error::Result<()> {
    let store = MemoryObjectStore::new();
    let block_id = ObjectId::from_bytes([7; 32]);
    let valid = payload(BlockKind::Root, Vec::new(), compute_state_root(&[])?);
    verify_block_v2_state(&store, block_id, &valid)?;

    let invalid = payload(BlockKind::Root, Vec::new(), MerkleRoot([0; 32]));
    assert!(verify_block_v2_state(&store, block_id, &invalid).is_err());
    Ok(())
}

#[test]
fn format2_lineage_rejects_schema1_parent() -> prikk_error::Result<()> {
    let mut store = MemoryObjectStore::new();
    let parent_payload = payload(BlockKind::Root, Vec::new(), MerkleRoot([0; 32]));
    let parent =
        ObjectEnvelope::unsigned(ObjectType::Block, 1, parent_payload.to_canonical_bytes()?);
    let parent_id = store.write_object(&parent)?;
    assert!(derive_next_state_root(&store, Some(parent_id), &[]).is_err());
    Ok(())
}

#[test]
fn format2_lineage_rejects_parent_with_forged_state_root() -> prikk_error::Result<()> {
    let mut store = MemoryObjectStore::new();
    let parent_payload = payload(BlockKind::Root, Vec::new(), MerkleRoot([0; 32]));
    let parent =
        ObjectEnvelope::unsigned(ObjectType::Block, 2, parent_payload.to_canonical_bytes()?);
    let parent_id = store.write_object(&parent)?;

    assert!(derive_next_state_root(&store, Some(parent_id), &[]).is_err());
    Ok(())
}

#[test]
fn equivalent_state_ignores_patch_identity() -> prikk_error::Result<()> {
    let mut store = MemoryObjectStore::new();
    let blob_payload = BlobPayload::new(BlobKind::Text, b"same state\n".to_vec());
    let blob = ObjectEnvelope::unsigned(ObjectType::Blob, 1, blob_payload.to_canonical_bytes()?);
    let blob_id = store.write_object(&blob)?;
    let operation = Operation {
        op_seq: 1,
        op_id: None,
        preconditions: Vec::new(),
        kind: OperationKind::CreateFile(CreateFile {
            path: "same.txt".to_string(),
            node_id: NodeId::from_bytes([0x41; 32]),
            blob_id,
            mode: 0o100644,
        }),
    };
    let first = patch_with_parents(operation.clone(), Vec::new())?;
    let second = patch_with_parents(operation, vec![ObjectId::from_bytes([0x77; 32])])?;
    let first_id = store.write_object(&first)?;
    let second_id = store.write_object(&second)?;
    assert_ne!(first_id, second_id);
    assert_eq!(
        derive_next_state_root(&store, None, &[first_id])?,
        derive_next_state_root(&store, None, &[second_id])?
    );
    Ok(())
}

fn patch_with_parents(
    operation: Operation,
    parent_patch_ids: Vec<ObjectId>,
) -> prikk_error::Result<ObjectEnvelope> {
    let payload = PatchPayload {
        operations: vec![operation],
        parent_patch_ids,
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    Ok(ObjectEnvelope::unsigned(
        ObjectType::Patch,
        1,
        payload.to_canonical_bytes()?,
    ))
}
