//! Retained active-state comparison used by mutation guards.

use prikk_error::{PrikkError, Result};
use prikk_object::{BlockPayload, ObjectType, RefStatePayload};

use super::{RefPublication, RefStore};
use crate::active::{ActiveRefMetadata, read_active_ref_metadata};
use crate::layout::RepositoryLayout;
use crate::object_store::FileObjectStore;
use crate::trust::{load_maintainer_trust_policy, verify_trusted_publication_envelope};
use crate::wal::Wal;

pub(super) fn has_incomplete_active_cleanup(layout: &RepositoryLayout) -> Result<bool> {
    let replay = Wal::for_layout(layout).replay()?;
    let ActiveRefMetadata::Valid(ref_name) = read_active_ref_metadata(layout)? else {
        return Ok(false);
    };
    if replay.records.is_empty() {
        return Ok(false);
    }
    let store = RefStore::new(layout.clone());
    let Some(state_id) = store.read_current_ref_state_id(&ref_name)? else {
        return Ok(false);
    };
    let objects = FileObjectStore::new(layout.clone());
    let state = objects
        .read_typed(state_id, ObjectType::RefState)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing RefState object: {state_id}")))?;
    let target = RefStatePayload::decode_canonical(&state.canonical_payload, state.schema_version)?
        .target_object_id;
    let block = objects
        .read_typed(target, ObjectType::Block)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing Block object: {target}")))?;
    let payload = BlockPayload::decode_canonical(&block.canonical_payload)?;
    Ok(payload.patch_ids
        == replay
            .records
            .iter()
            .map(|record| record.envelope.object_id())
            .collect::<Vec<_>>())
}

pub(super) fn validate_signer_backed_recovery(
    layout: &RepositoryLayout,
    publication: &RefPublication,
) -> Result<()> {
    match read_active_ref_metadata(layout)? {
        ActiveRefMetadata::Valid(ref_name) if ref_name == publication.ref_name => {}
        _ => {
            return Err(PrikkError::Integrity(
                "signer-backed ref recovery requires matching retained active-ref metadata"
                    .to_string(),
            ));
        }
    }
    let replay = Wal::for_layout(layout).replay()?;
    if replay.records.is_empty() || replay.trailing_partial_bytes != 0 {
        return Err(PrikkError::Integrity(
            "signer-backed ref recovery requires a complete non-empty active WAL".to_string(),
        ));
    }
    let state = RefStatePayload::decode_canonical(
        &publication.ref_state.canonical_payload,
        publication.ref_state.schema_version,
    )?;
    let objects = FileObjectStore::new(layout.clone());
    let block = objects
        .read_typed(state.target_object_id, ObjectType::Block)?
        .ok_or_else(|| {
            PrikkError::Integrity(format!("missing Block object: {}", state.target_object_id))
        })?;
    let payload = BlockPayload::decode_canonical(&block.canonical_payload)?;
    let wal_patch_ids = replay
        .records
        .iter()
        .map(|record| record.envelope.object_id())
        .collect::<Vec<_>>();
    if payload.patch_ids != wal_patch_ids {
        return Err(PrikkError::Integrity(
            "retained active WAL does not prove the proposed publication Block".to_string(),
        ));
    }
    let policy = load_maintainer_trust_policy(layout)?;
    for envelope in [&block, &publication.ref_state, &publication.ref_update] {
        verify_trusted_publication_envelope(&policy, envelope)
            .map_err(|issue| PrikkError::InvalidSignature(issue.message))?;
    }
    Ok(())
}
