//! Doctor tests.

use std::io::Write;

use prikk_object::{
    BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope, ObjectType,
};

use crate::{
    doctor_repository, repair_repository, DoctorRepairOptions, DoctorSeverity, FileObjectStore,
    ObjectWriter, RepositoryLayout, Wal,
};

use super::helpers::{
    maintainer_signature, sample_object_id, signed_empty_block_envelope, signed_patch_envelope, unique_temp_dir,
};

#[test]
fn doctor_reports_healthy_repository() {
    let root = unique_temp_dir("doctor-healthy");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let report = doctor_repository(&layout);
        assert!(report.is_healthy());
        assert!(report.verification.is_some());
        assert_eq!(report.count_by_severity(DoctorSeverity::Error), 0);
        assert_eq!(report.count_by_severity(DoctorSeverity::Info), 1);
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn doctor_reports_trailing_partial_wal_warning() {
    let root = unique_temp_dir("doctor-partial-wal");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let wal_path = layout.default_queue_wal_path();
        assert!(std::fs::write(&wal_path, b"partial").is_ok());
        let report = doctor_repository(&layout);
        assert!(report.is_healthy());
        assert_eq!(report.count_by_severity(DoctorSeverity::Warning), 1);
        assert_eq!(
            report.verification.as_ref().map(|summary| summary.trailing_partial_wal_bytes),
            Some(7)
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn doctor_reports_verification_error() {
    let root = unique_temp_dir("doctor-bad-block");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut store = FileObjectStore::new(layout.clone());
        let missing_patch = sample_object_id("doctor-missing-patch");
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
            let report = doctor_repository(&layout);
            assert!(!report.is_healthy());
            assert_eq!(report.count_by_severity(DoctorSeverity::Error), 1);
            assert!(report.verification.is_none());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn doctor_repair_truncates_only_trailing_partial_wal() {
    let root = unique_temp_dir("doctor-repair-wal-tail");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let wal = Wal::new(layout.default_queue_wal_path());
        assert!(wal.append_patch(&signed_patch_envelope()).is_ok());
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(layout.default_queue_wal_path());
        assert!(file.is_ok());
        if let Ok(file) = file.as_mut() {
            assert!(file.write_all(b"partial").is_ok());
            assert!(file.sync_all().is_ok());
        }

        let before = doctor_repository(&layout);
        assert!(before.is_healthy());
        assert_eq!(
            before.verification.as_ref().map(|summary| summary.trailing_partial_wal_bytes),
            Some(7)
        );

        let repair = repair_repository(&layout, DoctorRepairOptions::truncate_wal_tail());
        assert!(repair.is_ok());
        if let Ok(repair) = repair {
            assert_eq!(repair.wal_repair.truncated_bytes, 7);
            assert_eq!(repair.wal_repair.preserved_records, 1);
            assert!(repair.after.is_healthy());
            assert_eq!(
                repair
                    .after
                    .verification
                    .as_ref()
                    .map(|summary| summary.trailing_partial_wal_bytes),
                Some(0)
            );
        }

        let replay = wal.replay();
        assert!(replay.is_ok());
        if let Ok(replay) = replay {
            assert_eq!(replay.records.len(), 1);
            assert_eq!(replay.trailing_partial_bytes, 0);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn doctor_repair_reconstructs_missing_main_ref_pointer() {
    let root = unique_temp_dir("doctor-repair-main-ref");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let block = signed_empty_block_envelope();
        let target = block.object_id();
        assert!(object_store.write_object(&block).is_ok());
        let store = crate::RefStore::new(layout.clone());
        let ref_state = crate::tests::helpers::signed_ref_state_envelope("heads/main", None, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = crate::tests::helpers::signed_ref_update_envelope(
            "heads/main",
            None,
            ref_state_id,
            target,
            1,
        );
        let publication = crate::RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        assert!(store.publish(&publication).is_ok());
        assert!(std::fs::remove_file(layout.ref_pointer_path("heads/main")).is_ok());

        let before = doctor_repository(&layout);
        assert!(before.is_healthy());
        assert_eq!(before.count_by_severity(DoctorSeverity::Warning), 1);

        let repair = repair_repository(&layout, DoctorRepairOptions::reconstruct_main_ref());
        assert!(repair.is_ok());
        if let Ok(repair) = repair {
            assert_eq!(repair.ref_repair.as_ref().map(|value| value.wrote_pointer), Some(true));
            assert!(repair.after.is_healthy());
            assert_eq!(repair.after.count_by_severity(DoctorSeverity::Warning), 0);
        }
        assert_eq!(store.read_current_ref_state_id("heads/main"), Ok(Some(ref_state_id)));
    }
    let _ = std::fs::remove_dir_all(root);
}
