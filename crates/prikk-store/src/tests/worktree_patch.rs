//! Worktree patch draft tests.

use prikk_object::{
    BlobPayload, BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope, ObjectType,
};

use crate::{
    FileObjectStore, ObjectWriter, RefPublication, RefStore, RepoPath, RepositoryLayout,
    SnapshotEntry, SnapshotManifest, Wal, WorktreePatchCommitOptions, WorktreePatchOperationKind,
    commit_worktree_changes, commit_worktree_changes_with_options, materialize_snapshot_checkout,
};

use super::helpers::{
    maintainer_signature, signed_ref_state_envelope, signed_ref_update_envelope, unique_temp_dir,
};

#[test]
fn worktree_patch_commit_records_modified_file() {
    let root = unique_temp_dir("worktree-patch-modified");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(publish_snapshot_block(&layout, "README.md", b"hello\n").is_ok());
        assert!(materialize_snapshot_checkout(&layout, "heads/main").is_ok());
        assert!(std::fs::write(root.join("README.md"), b"changed\n").is_ok());
        let report = commit_worktree_changes(&layout, "heads/main", "change readme");
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.operation_count, 1);
            assert_eq!(report.referenced_blob_count, 2);
            assert_eq!(report.changes.len(), 1);
            assert_eq!(
                report.changes[0].operation,
                WorktreePatchOperationKind::ReplaceBinary
            );
            let replay = Wal::new(layout.default_queue_wal_path()).replay();
            assert!(replay.is_ok());
            if let Ok(replay) = replay {
                assert_eq!(replay.records.len(), 1);
                assert_eq!(replay.records[0].envelope.object_id(), report.patch_id);
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn worktree_patch_commit_can_emit_full_file_text_edit() {
    let root = unique_temp_dir("worktree-patch-text-edit");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(publish_snapshot_block(&layout, "README.md", b"hello\n").is_ok());
        assert!(materialize_snapshot_checkout(&layout, "heads/main").is_ok());
        assert!(std::fs::write(root.join("README.md"), b"changed\n").is_ok());
        let report = commit_worktree_changes_with_options(
            &layout,
            "heads/main",
            "change text",
            WorktreePatchCommitOptions::prefer_text_edits(),
        );
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.operation_count, 1);
            assert_eq!(report.referenced_blob_count, 0);
            assert_eq!(report.text_edit_count, 1);
            assert_eq!(report.changes.len(), 1);
            assert_eq!(
                report.changes[0].operation,
                WorktreePatchOperationKind::EditText
            );
            let replay = Wal::new(layout.default_queue_wal_path()).replay();
            assert!(replay.is_ok());
            if let Ok(replay) = replay {
                assert_eq!(replay.records.len(), 1);
                assert_eq!(replay.records[0].envelope.object_id(), report.patch_id);
            }
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn worktree_patch_text_mode_falls_back_for_binary_modified_file() {
    let root = unique_temp_dir("worktree-patch-text-binary-fallback");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(publish_snapshot_block(&layout, "data.bin", &[0xff, 0x00]).is_ok());
        assert!(materialize_snapshot_checkout(&layout, "heads/main").is_ok());
        assert!(std::fs::write(root.join("data.bin"), &[0xfe, 0x01]).is_ok());
        let report = commit_worktree_changes_with_options(
            &layout,
            "heads/main",
            "change binary",
            WorktreePatchCommitOptions::prefer_text_edits(),
        );
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.operation_count, 1);
            assert_eq!(report.referenced_blob_count, 2);
            assert_eq!(report.text_edit_count, 0);
            assert_eq!(
                report.changes[0].operation,
                WorktreePatchOperationKind::ReplaceBinary
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn worktree_patch_commit_records_untracked_file() {
    let root = unique_temp_dir("worktree-patch-untracked");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(publish_snapshot_block(&layout, "README.md", b"hello\n").is_ok());
        assert!(materialize_snapshot_checkout(&layout, "heads/main").is_ok());
        assert!(std::fs::write(root.join("extra.txt"), b"extra\n").is_ok());
        let report = commit_worktree_changes(&layout, "heads/main", "add extra");
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.operation_count, 1);
            assert_eq!(report.referenced_blob_count, 1);
            assert_eq!(
                report.changes[0].operation,
                WorktreePatchOperationKind::CreateFile
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn worktree_patch_commit_rejects_clean_worktree() {
    let root = unique_temp_dir("worktree-patch-clean");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(publish_snapshot_block(&layout, "README.md", b"hello\n").is_ok());
        assert!(materialize_snapshot_checkout(&layout, "heads/main").is_ok());
        let report = commit_worktree_changes(&layout, "heads/main", "nothing");
        assert!(report.is_err());
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
    let blob = BlobPayload {
        bytes: snapshot_bytes,
    };
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
    };
    let payload_bytes = payload.to_canonical_bytes();
    assert!(payload_bytes.is_ok());
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::Block, 1, payload_bytes.unwrap_or_default());
    assert!(envelope.add_signature(maintainer_signature()).is_ok());
    envelope
}
