//! Worktree status tests.

use prikk_object::{
    BlobKind, BlobPayload, BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope,
    ObjectType,
};

use crate::{
    FileObjectStore, ObjectWriter, RefPublication, RefStore, RepoPath, RepositoryLayout,
    SnapshotEntry, SnapshotManifest, WorktreeChangeKind, materialize_snapshot_checkout,
    worktree_status,
};

use crate::test_support::{
    maintainer_signature, signed_ref_state_envelope, signed_ref_update_envelope, unique_temp_dir,
};

#[test]
fn worktree_status_is_clean_after_snapshot_materialization() {
    let root = unique_temp_dir("worktree-status-clean");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(publish_snapshot_block(&layout, "README.md", b"hello\n").is_ok());
        assert!(materialize_snapshot_checkout(&layout, "heads/main").is_ok());
        let report = worktree_status(&layout, "heads/main");
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert!(report.is_clean());
            assert_eq!(report.tracked_files, 1);
            assert_eq!(report.unchanged_files, 1);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn worktree_status_reports_modified_file() {
    let root = unique_temp_dir("worktree-status-modified");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(publish_snapshot_block(&layout, "README.md", b"hello\n").is_ok());
        assert!(materialize_snapshot_checkout(&layout, "heads/main").is_ok());
        assert!(std::fs::write(root.join("README.md"), b"changed\n").is_ok());
        let report = worktree_status(&layout, "heads/main");
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert!(!report.is_clean());
            assert_eq!(report.count_kind(WorktreeChangeKind::Modified), 1);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn worktree_status_reports_missing_file() {
    let root = unique_temp_dir("worktree-status-missing");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(publish_snapshot_block(&layout, "README.md", b"hello\n").is_ok());
        assert!(materialize_snapshot_checkout(&layout, "heads/main").is_ok());
        assert!(std::fs::remove_file(root.join("README.md")).is_ok());
        let report = worktree_status(&layout, "heads/main");
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert!(!report.is_clean());
            assert_eq!(report.count_kind(WorktreeChangeKind::Missing), 1);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn worktree_status_reports_untracked_file() {
    let root = unique_temp_dir("worktree-status-untracked");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(publish_snapshot_block(&layout, "README.md", b"hello\n").is_ok());
        assert!(materialize_snapshot_checkout(&layout, "heads/main").is_ok());
        assert!(std::fs::write(root.join("extra.txt"), b"extra\n").is_ok());
        let report = worktree_status(&layout, "heads/main");
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert!(!report.is_clean());
            assert_eq!(report.count_kind(WorktreeChangeKind::Untracked), 1);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

fn publish_snapshot_block(
    layout: &RepositoryLayout,
    path: &str,
    bytes: &[u8],
) -> prikk_error::Result<prikk_object::ObjectId> {
    let mut object_store = FileObjectStore::new(layout.clone());
    let path = RepoPath::parse(path)?;
    let manifest = SnapshotManifest {
        files: vec![SnapshotEntry {
            path,
            bytes: bytes.to_vec(),
        }],
    };
    let snapshot_bytes = manifest.encode()?;
    let blob = BlobPayload::new(BlobKind::Snapshot, snapshot_bytes);
    let blob_bytes = blob.to_canonical_bytes()?;
    let mut blob_envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, blob_bytes);
    blob_envelope.add_signature(maintainer_signature())?;
    let blob_id = blob_envelope.object_id();
    object_store.write_object(&blob_envelope)?;

    let block = signed_snapshot_block_envelope(blob_id);
    let block_id = block.object_id();
    object_store.write_object(&block)?;

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
    ref_store.publish(&publication)?;
    Ok(block_id)
}

fn signed_snapshot_block_envelope(snapshot_blob_ref: prikk_object::ObjectId) -> ObjectEnvelope {
    let payload = BlockPayload {
        parent_block_ids: Vec::new(),
        kind: BlockKind::Normal,
        patch_ids: Vec::new(),
        state_merkle_root: MerkleRoot([0_u8; 32]),
        snapshot_blob_ref: Some(snapshot_blob_ref),
        mainline_parent_id: None,
        merge_baseline_block_id: None,
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::Block, 2, payload_bytes.unwrap_or_default());
    assert!(envelope.add_signature(maintainer_signature()).is_ok());
    envelope
}
