//! Repository verification tests.

use prikk_object::{
    BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope, ObjectType,
};

use crate::{
    ActiveWalMetadataStatus, FileObjectStore, ObjectWriter, RepositoryLayout, Wal,
    verify_repository, write_active_ref_metadata,
};

use crate::test_support::{
    dummy_signature, maintainer_signature, sample_object_id, signed_patch_envelope, unique_temp_dir,
};

#[test]
fn verify_repository_detects_block_with_missing_patch() {
    let root = unique_temp_dir("block-missing-patch");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut store = FileObjectStore::new(layout.clone());
        let missing_patch = sample_object_id("missing-patch");
        let payload = BlockPayload {
            parent_block_ids: Vec::new(),
            kind: BlockKind::Root,
            patch_ids: vec![missing_patch],
            state_merkle_root: MerkleRoot([0_u8; 32]),
            snapshot_blob_ref: None,
        };
        let payload_bytes = payload.to_canonical_bytes();
        assert!(payload_bytes.is_ok());
        if let Ok(payload_bytes) = payload_bytes {
            let mut block = ObjectEnvelope::unsigned(ObjectType::Block, 1, payload_bytes);
            assert!(block.add_signature(maintainer_signature()).is_ok());
            assert!(store.write_object(&block).is_ok());
            assert!(verify_repository(&layout).is_err());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn verify_repository_counts_objects_and_wal_records() {
    let root = unique_temp_dir("verify");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut store = FileObjectStore::new(layout.clone());
        let mut blob = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"payload".to_vec());
        assert!(blob.add_signature(dummy_signature()).is_ok());
        assert!(store.write_object(&blob).is_ok());

        let wal = Wal::new(layout.default_queue_wal_path());
        assert!(write_active_ref_metadata(&layout, "heads/main").is_ok());
        assert!(wal.append_patch(&signed_patch_envelope()).is_ok());

        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.checked_objects, 1);
            assert_eq!(report.checked_blocks, 0);
            assert_eq!(report.checked_wal_records, 1);
            assert_eq!(report.persisted_wal_patches, 0);
            assert_eq!(report.checked_refs, 0);
            assert_eq!(report.checked_ref_log_records, 0);
            assert_eq!(report.trailing_partial_wal_bytes, 0);
            assert_eq!(
                report.active_wal_metadata_status,
                ActiveWalMetadataStatus::ValidForNonEmptyWal {
                    ref_name: "heads/main".to_string()
                }
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn verify_repository_reports_missing_active_metadata_for_non_empty_wal() {
    let root = unique_temp_dir("verify-active-metadata-missing");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let wal = Wal::new(layout.default_queue_wal_path());
        assert!(wal.append_patch(&signed_patch_envelope()).is_ok());

        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(
                report.active_wal_metadata_status,
                ActiveWalMetadataStatus::MissingForNonEmptyWal
            );
            assert!(report.has_active_wal_metadata_integrity_issue());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn verify_repository_reports_malformed_empty_active_metadata_as_warning_state() {
    let root = unique_temp_dir("verify-active-metadata-debris");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(std::fs::write(layout.default_active_ref_name_path(), b"tags/v1").is_ok());

        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert!(matches!(
                report.active_wal_metadata_status,
                ActiveWalMetadataStatus::InvalidForEmptyWal { .. }
            ));
            assert!(report.has_active_wal_metadata_warning());
            assert!(!report.has_active_wal_metadata_integrity_issue());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn verify_repository_detects_object_file_in_wrong_prefix() {
    let root = unique_temp_dir("verify-wrong-prefix");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut store = FileObjectStore::new(layout.clone());
        let envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"payload".to_vec());
        let id = store.write_object(&envelope);
        assert!(id.is_ok());
        if let Ok(id) = id {
            let correct = layout.object_path(ObjectType::Blob, id);
            let wrong_dir = layout.object_type_dir(ObjectType::Blob).join("ff");
            assert!(std::fs::create_dir_all(&wrong_dir).is_ok());
            let wrong = wrong_dir.join(format!("{}.pobj", id.to_hex()));
            assert!(std::fs::rename(correct, wrong).is_ok());
            assert!(verify_repository(&layout).is_err());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}
