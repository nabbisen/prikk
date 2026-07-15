//! Minimal local seal command implementation.
//!
//! This module publishes the currently active WAL as one Block and advances `heads/main` through
//! the existing RefState/RefUpdate primitives. It deliberately does not implement audit plugins,
//! patch application, real worktree state materialization, or conflict algebra.

use std::path::PathBuf;

use prikk_hash::sha256;
use prikk_object::{
    BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope, ObjectType, RefKind,
    RefStatePayload, RefUpdatePayload,
};
use prikk_store::{
    ActiveLock, ActiveRefMetadata, FileObjectStore, MaintainerSigner, ObjectWriter, RefPublication,
    RefStore, RepositoryLayout, Wal, maintainer_signature, read_active_ref_metadata,
    remove_active_ref_metadata, validate_local_branch_ref, verify_signer_trusted,
};

const DEFAULT_BRANCH_REF: &str = "heads/main";

/// Result of sealing the current active WAL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealCommandResult {
    /// Ref advanced by seal.
    pub ref_name: String,
    /// Number of patch records sealed.
    pub patch_count: usize,
    /// New block ID.
    pub block_id: prikk_object::ObjectId,
    /// New RefState ID.
    pub ref_state_id: prikk_object::ObjectId,
}

/// Parse and run the local seal scaffold.
pub fn run_seal(
    root: PathBuf,
    args: Vec<String>,
    signer: &impl MaintainerSigner,
) -> std::result::Result<SealCommandResult, String> {
    let ref_name = parse_seal_args(args)?;
    let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
    seal_active_no_audit(layout, &ref_name, signer)
}

fn parse_seal_args(args: Vec<String>) -> std::result::Result<String, String> {
    let mut allow_no_audit = false;
    let mut ref_name = DEFAULT_BRANCH_REF.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--allow-no-audit" => allow_no_audit = true,
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("seal --ref requires a value".to_string());
                };
                ref_name = value;
            }
            other => return Err(format!("unknown seal argument: {other}")),
        }
    }
    if !allow_no_audit {
        return Err("seal scaffold requires --allow-no-audit".to_string());
    }
    validate_local_branch_ref(&ref_name).map_err(|err| err.to_string())
}

fn seal_active_no_audit(
    layout: RepositoryLayout,
    ref_name: &str,
    signer: &impl MaintainerSigner,
) -> std::result::Result<SealCommandResult, String> {
    let ref_name = validate_local_branch_ref(ref_name).map_err(|err| err.to_string())?;
    let _active_lock = ActiveLock::acquire(&layout).map_err(|err| err.to_string())?;
    let wal = Wal::for_layout(&layout);
    let replay = wal.replay().map_err(|err| err.to_string())?;
    if replay.trailing_partial_bytes != 0 {
        return Err(format!(
            "active WAL has {} trailing partial bytes; run verify/doctor before seal",
            replay.trailing_partial_bytes
        ));
    }
    if replay.records.is_empty() {
        match read_active_ref_metadata(&layout).map_err(|err| err.to_string())? {
            ActiveRefMetadata::Missing => {}
            ActiveRefMetadata::Valid(_) | ActiveRefMetadata::Invalid(_) => {
                remove_active_ref_metadata(&layout).map_err(|err| err.to_string())?;
            }
        }
        return Err("active WAL has no patch records to seal".to_string());
    }
    match read_active_ref_metadata(&layout).map_err(|err| err.to_string())? {
        ActiveRefMetadata::Valid(actual) if actual == ref_name => {}
        ActiveRefMetadata::Valid(actual) => {
            return Err(format!(
                "active WAL is owned by {actual}; requested seal ref is {ref_name}"
            ));
        }
        ActiveRefMetadata::Missing => {
            return Err("active WAL has records but active ref metadata is missing".to_string());
        }
        ActiveRefMetadata::Invalid(reason) => {
            return Err(format!(
                "active WAL has records but active ref metadata is malformed: {reason}"
            ));
        }
    }

    let mut object_store = FileObjectStore::new(layout.clone());
    let ref_store = RefStore::new(layout.clone());
    let current = current_ref_state(&object_store, &ref_store, &ref_name)?;
    let wal_patch_ids = collect_wal_patch_ids(&replay.records)?;
    if let Some(current) = current.as_ref() {
        if current_tip_matches_wal_patches(&object_store, current, &wal_patch_ids)? {
            wal.truncate_empty().map_err(|err| err.to_string())?;
            remove_active_ref_metadata(&layout).map_err(|err| err.to_string())?;
            return Ok(SealCommandResult {
                ref_name,
                patch_count: wal_patch_ids.len(),
                block_id: current.target_block_id,
                ref_state_id: current.ref_state_id,
            });
        }
    }
    verify_signer_trusted(&layout, signer).map_err(|err| err.to_string())?;
    let patch_ids = persist_wal_patches(&mut object_store, &replay.records)?;
    let parent_block_ids = current
        .as_ref()
        .map(|state| vec![state.target_block_id])
        .unwrap_or_default();
    let block_payload = BlockPayload {
        parent_block_ids,
        kind: if current.is_some() {
            BlockKind::Normal
        } else {
            BlockKind::Root
        },
        patch_ids: patch_ids.clone(),
        state_merkle_root: scaffold_state_root(&patch_ids),
        snapshot_blob_ref: None,
    };
    let block_envelope = signed_envelope(
        ObjectType::Block,
        block_payload
            .to_canonical_bytes()
            .map_err(|err| err.to_string())?,
        signer,
    )?;
    let block_id = object_store
        .write_object(&block_envelope)
        .map_err(|err| err.to_string())?;
    let update_seq = current
        .as_ref()
        .map(|state| state.update_seq + 1)
        .unwrap_or(1);
    let previous_ref_state_id = current.as_ref().map(|state| state.ref_state_id);
    let ref_state_payload = RefStatePayload {
        ref_name: ref_name.to_string(),
        kind: RefKind::Branch,
        target_object_id: block_id,
        update_seq,
        previous_ref_state_id,
        required_attestation_ids: Vec::new(),
    };
    let ref_state_envelope = signed_envelope(
        ObjectType::RefState,
        ref_state_payload
            .to_canonical_bytes()
            .map_err(|err| err.to_string())?,
        signer,
    )?;
    let ref_state_id = ref_state_envelope.object_id();
    let ref_update_payload = RefUpdatePayload {
        ref_name: ref_name.to_string(),
        old_ref_state_id: previous_ref_state_id,
        new_ref_state_id: ref_state_id,
        new_target_object_id: block_id,
        update_seq,
        created_at: 0,
        author_key_id: signer.key_id().to_string(),
    };
    let ref_update_envelope = signed_envelope(
        ObjectType::RefUpdate,
        ref_update_payload
            .to_canonical_bytes()
            .map_err(|err| err.to_string())?,
        signer,
    )?;
    let publication = RefPublication {
        ref_name: ref_name.clone(),
        expected_previous_ref_state_id: previous_ref_state_id,
        ref_state: ref_state_envelope,
        ref_update: ref_update_envelope,
    };
    let published_ref_state_id = ref_store
        .publish(&publication)
        .map_err(|err| err.to_string())?;
    wal.truncate_empty().map_err(|err| err.to_string())?;
    remove_active_ref_metadata(&layout).map_err(|err| err.to_string())?;
    Ok(SealCommandResult {
        ref_name,
        patch_count: patch_ids.len(),
        block_id,
        ref_state_id: published_ref_state_id,
    })
}

fn persist_wal_patches(
    object_store: &mut FileObjectStore,
    records: &[prikk_store::WalRecord],
) -> std::result::Result<Vec<prikk_object::ObjectId>, String> {
    let mut patch_ids = Vec::with_capacity(records.len());
    for record in records {
        if record.envelope.object_type != ObjectType::Patch {
            return Err(format!(
                "active WAL record {} is {}, expected patch",
                record.seq, record.envelope.object_type
            ));
        }
        let id = object_store
            .write_object(&record.envelope)
            .map_err(|err| err.to_string())?;
        patch_ids.push(id);
    }
    Ok(patch_ids)
}

fn collect_wal_patch_ids(
    records: &[prikk_store::WalRecord],
) -> std::result::Result<Vec<prikk_object::ObjectId>, String> {
    let mut patch_ids = Vec::with_capacity(records.len());
    for record in records {
        if record.envelope.object_type != ObjectType::Patch {
            return Err(format!(
                "active WAL record {} is {}, expected patch",
                record.seq, record.envelope.object_type
            ));
        }
        patch_ids.push(record.envelope.object_id());
    }
    Ok(patch_ids)
}

fn current_tip_matches_wal_patches(
    object_store: &FileObjectStore,
    current: &CurrentRefState,
    wal_patch_ids: &[prikk_object::ObjectId],
) -> std::result::Result<bool, String> {
    let envelope = object_store
        .read_typed(current.target_block_id, ObjectType::Block)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| {
            format!(
                "current ref targets missing block {}",
                current.target_block_id
            )
        })?;
    let block = BlockPayload::decode_canonical(&envelope.canonical_payload)
        .map_err(|err| err.to_string())?;
    Ok(block.patch_ids == wal_patch_ids)
}

fn current_ref_state(
    object_store: &FileObjectStore,
    ref_store: &RefStore,
    ref_name: &str,
) -> std::result::Result<Option<CurrentRefState>, String> {
    let Some(ref_state_id) = ref_store
        .read_current_ref_state_id(ref_name)
        .map_err(|err| err.to_string())?
    else {
        let log = ref_store
            .replay_log(ref_name)
            .map_err(|err| err.to_string())?;
        if log.trailing_partial_bytes != 0 {
            return Err(format!(
                "ref {ref_name} pointer is missing and its log has trailing partial bytes; \
                 run `prikk doctor` before seal"
            ));
        }
        if !log.records.is_empty() {
            return Err(format!(
                "ref {ref_name} pointer is missing but ref-log history exists; \
                 run `prikk doctor` before seal"
            ));
        }
        return Ok(None);
    };
    let envelope = object_store
        .read_typed(ref_state_id, ObjectType::RefState)
        .map_err(|err| err.to_string())?
        .ok_or_else(|| {
            format!("current ref {ref_name} points to missing RefState {ref_state_id}")
        })?;
    let payload = RefStatePayload::decode_canonical(&envelope.canonical_payload)
        .map_err(|err| err.to_string())?;
    if payload.ref_name != ref_name {
        return Err(format!(
            "current RefState name mismatch: expected {ref_name}, got {}",
            payload.ref_name
        ));
    }
    let target_exists = object_store
        .read_typed(payload.target_object_id, ObjectType::Block)
        .map_err(|err| err.to_string())?
        .is_some();
    if !target_exists {
        return Err(format!(
            "current RefState {ref_state_id} targets missing block {}",
            payload.target_object_id
        ));
    }
    Ok(Some(CurrentRefState {
        ref_state_id,
        target_block_id: payload.target_object_id,
        update_seq: payload.update_seq,
    }))
}

fn scaffold_state_root(patch_ids: &[prikk_object::ObjectId]) -> MerkleRoot {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(b"prikk.dev.scaffold-state-root.v1");
    for id in patch_ids {
        preimage.extend_from_slice(id.as_bytes());
    }
    MerkleRoot(sha256(&preimage))
}

fn signed_envelope(
    object_type: ObjectType,
    canonical_payload: Vec<u8>,
    signer: &impl MaintainerSigner,
) -> std::result::Result<ObjectEnvelope, String> {
    let mut envelope = ObjectEnvelope::unsigned(object_type, 1, canonical_payload);
    let object_id = envelope.object_id();
    envelope
        .add_signature(
            maintainer_signature(signer, object_type, object_id).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?;
    Ok(envelope)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CurrentRefState {
    ref_state_id: prikk_object::ObjectId,
    target_block_id: prikk_object::ObjectId,
    update_seq: u64,
}
