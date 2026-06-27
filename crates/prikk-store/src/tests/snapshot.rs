//! Snapshot path-safety and checkout-plan tests.

use prikk_object::{
    BlobPayload, BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope, ObjectType,
};

use crate::{
    prepare_snapshot_checkout_plan, FileObjectStore, ObjectWriter, RefPublication, RefStore,
    RepoPath, RepositoryLayout, SnapshotEntry, SnapshotManifest,
};

use super::helpers::{
    maintainer_signature, signed_ref_state_envelope, signed_ref_update_envelope, unique_temp_dir,
};

#[test]
fn repo_path_rejects_traversal_and_reserved_names() {
    assert!(RepoPath::parse("src/main.rs").is_ok());
    assert!(RepoPath::parse("../escape").is_err());
    assert!(RepoPath::parse("src/../escape").is_err());
    assert!(RepoPath::parse("/absolute").is_err());
    assert!(RepoPath::parse("CON.txt").is_err());
    assert!(RepoPath::parse("src\\main.rs").is_err());
    assert!(RepoPath::parse("日本語.txt").is_err());
}

#[test]
fn snapshot_manifest_rejects_case_collisions() {
    let upper = RepoPath::parse("README.md");
    let lower = RepoPath::parse("readme.md");
    assert!(upper.is_ok());
    assert!(lower.is_ok());
    if let (Ok(upper), Ok(lower)) = (upper, lower) {
        let manifest = SnapshotManifest {
            files: vec![
                SnapshotEntry { path: upper, bytes: b"a".to_vec() },
                SnapshotEntry { path: lower, bytes: b"b".to_vec() },
            ],
        };
        assert!(manifest.encode().is_err());
    }
}

#[test]
fn snapshot_checkout_plan_validates_snapshot_manifest() {
    let root = unique_temp_dir("snapshot-plan");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let path = match RepoPath::parse("src/main.rs") {
            Ok(path) => path,
            Err(err) => panic!("test path should validate: {err}"),
        };
        let manifest = SnapshotManifest {
            files: vec![SnapshotEntry { path, bytes: b"fn main() {}\n".to_vec() }],
        };
        let snapshot_bytes = manifest.encode();
        assert!(snapshot_bytes.is_ok());
        let blob = BlobPayload { bytes: snapshot_bytes.unwrap_or_default() };
        let blob_bytes = blob.to_canonical_bytes();
        assert!(blob_bytes.is_ok());
        let mut blob_envelope = ObjectEnvelope::unsigned(
            ObjectType::Blob,
            1,
            blob_bytes.unwrap_or_default(),
        );
        assert!(blob_envelope.add_signature(maintainer_signature()).is_ok());
        let blob_id = blob_envelope.object_id();
        assert!(object_store.write_object(&blob_envelope).is_ok());

        let block = signed_snapshot_block_envelope(blob_id);
        let block_id = block.object_id();
        assert!(object_store.write_object(&block).is_ok());

        let ref_store = RefStore::new(layout.clone());
        let ref_state = signed_ref_state_envelope("heads/main", None, block_id, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, block_id, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        assert!(ref_store.publish(&publication).is_ok());

        let plan = prepare_snapshot_checkout_plan(&layout, "heads/main");
        assert!(plan.is_ok());
        if let Ok(plan) = plan {
            assert_eq!(plan.file_count, 1);
            assert_eq!(plan.total_content_bytes, 13);
            assert_eq!(plan.paths, vec!["src/main.rs".to_string()]);
            assert_eq!(plan.snapshot_blob_id, blob_id);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

fn signed_snapshot_block_envelope(snapshot_blob_ref: prikk_object::ObjectId) -> ObjectEnvelope {
    let payload = BlockPayload {
        parent_block_ids: Vec::new(),
        kind: BlockKind::Normal,
        patch_ids: Vec::new(),
        state_merkle_root: MerkleRoot([0_u8; 32]),
        snapshot_blob_ref: Some(snapshot_blob_ref),
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let mut envelope = ObjectEnvelope::unsigned(
        ObjectType::Block,
        1,
        payload_bytes.unwrap_or_default(),
    );
    assert!(envelope.add_signature(maintainer_signature()).is_ok());
    envelope
}
