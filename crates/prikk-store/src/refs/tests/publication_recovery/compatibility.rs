//! Format-1 compatibility diagnostics that remain read-only.

use prikk_object::{CanonicalEncode, ObjectEnvelope, ObjectType, RefUpdatePayload};

use super::root_publication;
use crate::refs::log;
use crate::test_support::{maintainer_signature, signed_patch_envelope, unique_temp_dir};
use crate::{ActiveSession, RefStore, RepositoryLayout, verify_repository};

#[test]
fn format2_legacy_timestamp_is_not_normalized_and_blocks_mutation() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-legacy-timestamp");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    let store = RefStore::new(layout.clone());
    store.publish(&publication)?;
    std::fs::write(layout.ref_log_path("heads/main"), b"")?;
    let mut update = RefUpdatePayload::decode_canonical(&publication.ref_update.canonical_payload)?;
    update.created_at = 7;
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::RefUpdate, 1, update.to_canonical_bytes()?);
    envelope.add_signature(maintainer_signature())?;
    std::fs::write(
        layout.ref_log_path("heads/main"),
        log::encode_log_record_for_test(&envelope)?,
    )?;

    assert!(verify_repository(&layout).is_err());
    assert!(
        ActiveSession::new(layout.clone())
            .append_patch(&signed_patch_envelope())
            .is_err()
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
