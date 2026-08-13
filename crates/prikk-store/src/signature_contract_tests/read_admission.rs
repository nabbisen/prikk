use prikk_object::{ObjectEnvelope, ObjectType};

use super::admission::strict_rejection_variants;
use crate::file_codec::encode_envelope_file_structural;
use crate::refs::encode_log_record_for_test;
use crate::test_support::{
    signed_patch_blob_envelope, signed_patch_envelope, signed_ref_update_envelope, unique_temp_dir,
};
use crate::wal::encode_record_for_test;
use crate::{
    FileObjectStore, ObjectReader, RefStore, RepositoryLayout, Wal, WalRecord,
    derive_next_state_root, verify_repository,
};

fn write_structural_object(
    layout: &RepositoryLayout,
    envelope: &ObjectEnvelope,
) -> prikk_error::Result<prikk_object::ObjectId> {
    let object_id = envelope.object_id();
    let path = layout.object_path(envelope.object_type, object_id);
    let parent = path
        .parent()
        .ok_or_else(|| prikk_error::PrikkError::Io("test object path has no parent".to_string()))?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(path, encode_envelope_file_structural(envelope)?)?;
    Ok(object_id)
}

#[test]
fn format2_object_reads_reject_every_strict_envelope_failure() -> prikk_error::Result<()> {
    let valid = signed_patch_blob_envelope();
    for (index, invalid) in strict_rejection_variants(ObjectType::Blob, &valid.canonical_payload)
        .into_iter()
        .enumerate()
    {
        let root = unique_temp_dir(&format!("dc40-strict-object-read-{index}"));
        let layout = RepositoryLayout::init(root.clone())?;
        let object_id = write_structural_object(&layout, &invalid)?;
        let objects = FileObjectStore::new(layout.clone());

        assert!(objects.read_object(object_id).is_err());
        assert!(verify_repository(&layout)?.has_stage_failure());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn format2_wal_reads_reject_every_strict_envelope_failure() -> prikk_error::Result<()> {
    let valid = signed_patch_envelope();
    for (index, invalid) in strict_rejection_variants(ObjectType::Patch, &valid.canonical_payload)
        .into_iter()
        .enumerate()
    {
        let root = unique_temp_dir(&format!("dc40-strict-wal-read-{index}"));
        let layout = RepositoryLayout::init(root.clone())?;
        let record = WalRecord {
            seq: 1,
            envelope: invalid,
        };
        std::fs::write(
            layout.default_queue_wal_path(),
            encode_record_for_test(&record)?,
        )?;

        assert!(Wal::for_layout(&layout).replay().is_err());
        assert!(verify_repository(&layout)?.has_stage_failure());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn format2_ref_log_reads_reject_every_strict_envelope_failure() -> prikk_error::Result<()> {
    let valid = signed_ref_update_envelope(
        "heads/main",
        None,
        crate::test_support::sample_object_id("state"),
        crate::test_support::sample_object_id("target"),
        1,
    );
    for (index, invalid) in
        strict_rejection_variants(ObjectType::RefUpdate, &valid.canonical_payload)
            .into_iter()
            .enumerate()
    {
        let root = unique_temp_dir(&format!("dc40-strict-ref-log-read-{index}"));
        let layout = RepositoryLayout::init(root.clone())?;
        std::fs::write(
            layout.ref_log_path("heads/main"),
            encode_log_record_for_test(&invalid)?,
        )?;

        assert!(
            RefStore::new(layout.clone())
                .replay_log("heads/main")
                .is_err()
        );
        assert!(verify_repository(&layout)?.has_stage_failure());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn format2_authoritative_replay_rejects_every_strict_patch_failure() -> prikk_error::Result<()> {
    let valid = signed_patch_envelope();
    for (index, invalid) in strict_rejection_variants(ObjectType::Patch, &valid.canonical_payload)
        .into_iter()
        .enumerate()
    {
        let root = unique_temp_dir(&format!("dc40-strict-replay-read-{index}"));
        let layout = RepositoryLayout::init(root.clone())?;
        let patch_id = write_structural_object(&layout, &invalid)?;
        let objects = FileObjectStore::new(layout.clone());

        assert!(derive_next_state_root(&objects, None, &[patch_id]).is_err());
        assert!(verify_repository(&layout)?.has_stage_failure());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}
