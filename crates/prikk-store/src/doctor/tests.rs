//! Doctor tests.

use std::io::Write;

use prikk_object::{
    BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope, ObjectType, RefKind,
    RefStatePayload, RefUpdatePayload,
};

use crate::{
    DoctorRepairOptions, DoctorSeverity, Ed25519MaintainerSigner, FileObjectStore,
    MaintainerSigner, ObjectWriter, RepositoryLayout, Wal, add_trusted_maintainer,
    doctor_repository, maintainer_signature as real_maintainer_signature, repair_repository,
    write_active_ref_metadata,
};

use crate::test_support::{
    maintainer_signature as legacy_maintainer_signature, sample_object_id, signed_patch_envelope,
    unique_temp_dir,
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
            report
                .verification
                .as_ref()
                .map(|summary| summary.trailing_partial_wal_bytes),
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
            assert!(block.add_signature(legacy_maintainer_signature()).is_ok());
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
        let wal = Wal::for_layout(&layout);
        assert!(write_active_ref_metadata(&layout, "heads/main").is_ok());
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
            before
                .verification
                .as_ref()
                .map(|summary| summary.trailing_partial_wal_bytes),
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
fn doctor_reports_non_empty_active_wal_missing_metadata_as_error() {
    let root = unique_temp_dir("doctor-active-metadata-missing");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let wal = Wal::for_layout(&layout);
        assert!(wal.append_patch(&signed_patch_envelope()).is_ok());

        let report = doctor_repository(&layout);
        assert!(!report.is_healthy());
        assert_eq!(report.count_by_severity(DoctorSeverity::Error), 1);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == "PRIKK-DOCTOR-ACTIVE-REF-METADATA-MISSING" })
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn doctor_reports_empty_active_metadata_debris_as_warning() {
    let root = unique_temp_dir("doctor-active-metadata-debris");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(std::fs::write(layout.default_active_ref_name_path(), b"tags/v1").is_ok());

        let report = doctor_repository(&layout);
        assert!(report.is_healthy());
        assert_eq!(report.count_by_severity(DoctorSeverity::Warning), 1);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == "PRIKK-DOCTOR-ACTIVE-REF-METADATA-MALFORMED-DEBRIS" })
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn doctor_refuses_missing_main_ref_pointer_reconstruction() {
    let root = unique_temp_dir("doctor-repair-main-ref");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let maintainer_seed = [0x44_u8; 32];
        let maintainer =
            match Ed25519MaintainerSigner::from_seed("doctor-maintainer", &maintainer_seed) {
                Ok(signer) => signer,
                Err(error) => panic!("test maintainer signer should be constructible: {error}"),
            };
        let public_key = maintainer
            .public_key_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert!(add_trusted_maintainer(&layout, "doctor-maintainer", &public_key).is_ok());
        let mut object_store = FileObjectStore::new(layout.clone());
        let block_payload = BlockPayload {
            parent_block_ids: Vec::new(),
            kind: BlockKind::Root,
            patch_ids: Vec::new(),
            state_merkle_root: MerkleRoot([0_u8; 32]),
            snapshot_blob_ref: None,
        };
        let block = signed_publication_envelope(
            ObjectType::Block,
            block_payload.to_canonical_bytes().unwrap_or_default(),
            &maintainer,
        );
        let target = block.object_id();
        assert!(object_store.write_object(&block).is_ok());
        let store = crate::RefStore::new(layout.clone());
        let ref_state_payload = RefStatePayload {
            ref_name: "heads/main".to_string(),
            kind: RefKind::Branch,
            target_object_id: target,
            update_seq: 1,
            previous_ref_state_id: None,
            required_attestation_ids: Vec::new(),
        };
        let ref_state = signed_publication_envelope(
            ObjectType::RefState,
            ref_state_payload.to_canonical_bytes().unwrap_or_default(),
            &maintainer,
        );
        let ref_state_id = ref_state.object_id();
        let ref_update_payload = RefUpdatePayload {
            ref_name: "heads/main".to_string(),
            old_ref_state_id: None,
            new_ref_state_id: ref_state_id,
            new_target_object_id: target,
            update_seq: 1,
            created_at: 0,
            author_key_id: "doctor-maintainer".to_string(),
        };
        let ref_update = signed_publication_envelope(
            ObjectType::RefUpdate,
            ref_update_payload.to_canonical_bytes().unwrap_or_default(),
            &maintainer,
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
        assert!(!before.is_healthy());
        assert_eq!(before.count_by_severity(DoctorSeverity::Error), 1);

        let repair = repair_repository(&layout, DoctorRepairOptions::reconstruct_main_ref());
        assert!(repair.is_err());
        assert_eq!(store.read_current_ref_state_id("heads/main"), Ok(None));
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn doctor_repair_requires_active_lock_before_wal_mutation() -> prikk_error::Result<()> {
    let root = unique_temp_dir("doctor-active-lock");
    let layout = RepositoryLayout::init(root.clone())?;
    std::fs::write(layout.default_queue_wal_path(), b"partial")?;
    let before = std::fs::read(layout.default_queue_wal_path())?;
    let active_lock = crate::ActiveLock::acquire(&layout)?;

    let error = repair_repository(&layout, DoctorRepairOptions::truncate_wal_tail())
        .err()
        .ok_or_else(|| prikk_error::PrikkError::Integrity("repair unexpectedly ran".to_string()))?;
    assert!(matches!(error, prikk_error::PrikkError::LockConflict(_)));
    assert_eq!(std::fs::read(layout.default_queue_wal_path())?, before);

    drop(active_lock);
    let report = repair_repository(&layout, DoctorRepairOptions::truncate_wal_tail())?;
    assert_eq!(report.wal_repair.truncated_bytes, before.len());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn doctor_rechecks_publication_guard_after_acquiring_active_lock() -> prikk_error::Result<()> {
    let root = unique_temp_dir("doctor-under-lock-guard");
    let layout = RepositoryLayout::init(root.clone())?;
    std::fs::write(layout.default_queue_wal_path(), b"partial")?;
    let before = std::fs::read(layout.default_queue_wal_path())?;
    let active_lock = crate::ActiveLock::acquire(&layout)?;
    let candidate = layout.ref_tmp_path("heads/main");
    let parent = candidate
        .parent()
        .ok_or_else(|| prikk_error::PrikkError::Integrity("candidate has no parent".to_string()))?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(&candidate, b"candidate")?;

    assert!(repair_repository(&layout, DoctorRepairOptions::truncate_wal_tail()).is_err());
    drop(active_lock);
    let error = repair_repository(&layout, DoctorRepairOptions::truncate_wal_tail())
        .err()
        .ok_or_else(|| prikk_error::PrikkError::Integrity("repair unexpectedly ran".to_string()))?;
    assert!(
        error
            .to_string()
            .contains("repository mutation is blocked by incomplete ref publication")
    );
    assert_eq!(std::fs::read(layout.default_queue_wal_path())?, before);
    assert_eq!(std::fs::read(&candidate)?, b"candidate");
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

fn signed_publication_envelope(
    object_type: ObjectType,
    canonical_payload: Vec<u8>,
    signer: &impl MaintainerSigner,
) -> ObjectEnvelope {
    let mut envelope = ObjectEnvelope::unsigned(object_type, 1, canonical_payload);
    let object_id = envelope.object_id();
    let signature = real_maintainer_signature(signer, object_type, object_id);
    assert!(signature.is_ok());
    if let Ok(signature) = signature {
        assert!(envelope.add_signature(signature).is_ok());
    }
    envelope
}
