use prikk_object::{ObjectEnvelope, ObjectId, ObjectType};

use super::{malformed_envelope, signature};
use crate::file_codec::encode_envelope_file_structural;
use crate::refs::encode_log_record_for_test;
use crate::test_support::{
    signed_empty_block_envelope, signed_patch_envelope, signed_ref_state_envelope,
    signed_ref_update_envelope, unique_temp_dir,
};
use crate::wal::encode_record_for_test;
use crate::{
    RepositoryLayout, SignatureEnvelopeSource, Wal, WalRecord, verify_repository,
    write_active_ref_metadata,
};

fn all_issues_envelope(object_type: ObjectType, payload: &[u8]) -> ObjectEnvelope {
    let duplicate = signature("a", 1);
    let mut malformed_middle = signature("b", 2);
    malformed_middle.signature_bytes.truncate(1);
    let mut repeated = duplicate.clone();
    repeated.created_at = 99;
    ObjectEnvelope {
        object_type,
        schema_version: 1,
        canonical_payload: payload.to_vec(),
        signatures: vec![duplicate, malformed_middle, repeated],
    }
}

fn write_legacy_object(
    layout: &RepositoryLayout,
    envelope: &ObjectEnvelope,
) -> prikk_error::Result<ObjectId> {
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
fn format1_diagnostics_are_byte_preserving_suppressed_and_deterministic() -> prikk_error::Result<()>
{
    let root = unique_temp_dir("dc39-diagnostics");
    let layout = RepositoryLayout::init(root.clone())?;

    let patch_id = write_legacy_object(
        &layout,
        &malformed_envelope(ObjectType::Patch, b"legacy-patch", 63),
    )?;
    let blob_a = malformed_envelope(ObjectType::Blob, b"legacy-a", 65);
    let blob_a_id = write_legacy_object(&layout, &blob_a)?;
    let blob_b = all_issues_envelope(ObjectType::Blob, b"legacy-b");
    let blob_b_id = write_legacy_object(&layout, &blob_b)?;

    let mut wal_envelope = signed_patch_envelope();
    let wal_signature = wal_envelope.signatures.first_mut().ok_or_else(|| {
        prikk_error::PrikkError::Integrity("test WAL envelope is unsigned".to_string())
    })?;
    wal_signature.signature_bytes.truncate(1);
    let wal_record = WalRecord {
        seq: 1,
        envelope: wal_envelope.clone(),
    };
    std::fs::write(
        layout.default_queue_wal_path(),
        encode_record_for_test(&wal_record)?,
    )?;
    write_active_ref_metadata(&layout, "heads/main")?;

    let mut block = signed_empty_block_envelope();
    block.schema_version = 1;
    let block_id = write_legacy_object(&layout, &block)?;
    for ref_name in ["heads/z", "heads/a"] {
        let state = signed_ref_state_envelope(ref_name, None, block_id, 1);
        let state_id = write_legacy_object(&layout, &state)?;
        let update = signed_ref_update_envelope(ref_name, None, state_id, block_id, 1);
        let mut inverted_update = update;
        inverted_update.signatures = vec![signature("z", 9), signature("a", 8)];
        std::fs::write(
            layout.ref_log_path(ref_name),
            encode_log_record_for_test(&inverted_update)?,
        )?;
    }

    let observed_paths = [
        layout.object_path(ObjectType::Patch, patch_id),
        layout.object_path(ObjectType::Blob, blob_a_id),
        layout.object_path(ObjectType::Blob, blob_b_id),
        layout.default_queue_wal_path(),
        layout.ref_log_path("heads/a"),
        layout.ref_log_path("heads/z"),
    ];
    std::fs::write(root.join(".prikk/FORMAT"), b"1\n")?;
    let before = observed_paths
        .iter()
        .map(std::fs::read)
        .collect::<std::io::Result<Vec<_>>>()?;

    let legacy_layout = RepositoryLayout::open(root.clone())?;
    let report = verify_repository(&legacy_layout)?;
    let after = observed_paths
        .iter()
        .map(std::fs::read)
        .collect::<std::io::Result<Vec<_>>>()?;
    assert_eq!(before, after);

    let object_issues: Vec<_> = report
        .signature_envelope_issues
        .iter()
        .filter_map(|issue| match issue.source {
            SignatureEnvelopeSource::Object {
                object_type,
                object_id,
            } => Some((object_type.code(), object_id, issue.code)),
            _ => None,
        })
        .collect();
    assert_eq!(
        object_issues.first(),
        Some(&(
            ObjectType::Patch.code(),
            patch_id,
            "PRIKK-VERIFY-SIGNATURE-MALFORMED"
        ))
    );

    let mut blob_ids = [blob_a_id, blob_b_id];
    blob_ids.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    let observed_blob_ids: Vec<_> = object_issues
        .iter()
        .filter(|(code, _, _)| *code == ObjectType::Blob.code())
        .map(|(_, object_id, _)| *object_id)
        .collect();
    assert_eq!(observed_blob_ids.first(), blob_ids.first());
    assert_eq!(observed_blob_ids.last(), blob_ids.last());

    let all_issue_codes: Vec<_> = report
        .signature_envelope_issues
        .iter()
        .filter(|issue| {
            matches!(
                issue.source,
                SignatureEnvelopeSource::Object { object_id, .. } if object_id == blob_b_id
            )
        })
        .map(|issue| issue.code)
        .collect();
    assert_eq!(
        all_issue_codes,
        [
            "PRIKK-VERIFY-SIGNATURE-MALFORMED",
            "PRIKK-VERIFY-SIGNATURE-DUPLICATE",
            "PRIKK-VERIFY-SIGNATURE-NONCANONICAL-ORDER",
        ]
    );

    let source_kinds: Vec<_> = report
        .signature_envelope_issues
        .iter()
        .map(|issue| match issue.source {
            SignatureEnvelopeSource::Object { .. } => 0,
            SignatureEnvelopeSource::ActiveWal { sequence: 1, .. } => 1,
            SignatureEnvelopeSource::RefLog { ref sequence, .. } if *sequence == 1 => 2,
            _ => 3,
        })
        .collect();
    assert!(
        source_kinds
            .windows(2)
            .all(|pair| matches!(pair, [left, right] if left <= right))
    );
    assert!(source_kinds.contains(&1));
    assert!(source_kinds.contains(&2));
    let ref_names: Vec<_> = report
        .signature_envelope_issues
        .iter()
        .filter_map(|issue| match &issue.source {
            SignatureEnvelopeSource::RefLog { ref_name, .. } => Some(ref_name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(ref_names, ["heads/a", "heads/z"]);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn strict_writers_do_not_admit_legacy_diagnostic_envelopes() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc39-no-read-write-promotion");
    let layout = RepositoryLayout::init(root.clone())?;
    let malformed = malformed_envelope(ObjectType::Patch, b"legacy", 63);
    assert!(malformed.validate().is_ok());
    assert!(malformed.validate_strict().is_err());
    assert!(Wal::for_layout(&layout).append_patch(&malformed).is_err());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
