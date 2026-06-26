//! Minimal local seal command implementation.
//!
//! This module publishes the currently active WAL as one Block and advances `heads/main` through
//! the existing RefState/RefUpdate primitives. It deliberately does not implement audit plugins,
//! patch application, real worktree state materialization, or conflict algebra.

use std::path::PathBuf;

use prikk_hash::sha256;
use prikk_object::{
    BlockKind, BlockPayload, CanonicalEncode, MerkleRoot, ObjectEnvelope, ObjectType, RefKind,
    RefStatePayload, RefUpdatePayload, Signature, SignatureAlgorithm, SignerRole,
};
use prikk_store::{
    ActiveLock, FileObjectStore, ObjectWriter, RefPublication, RefStore,
    RepositoryLayout, Wal,
};

const DEFAULT_BRANCH_REF: &str = "heads/main";
const DEV_MAINTAINER_KEY_ID: &str = "dev-placeholder-maintainer";

/// Result of sealing the current active WAL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealCommandResult {
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
) -> std::result::Result<SealCommandResult, String> {
    parse_seal_args(args)?;
    let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
    seal_active_no_audit(layout, DEFAULT_BRANCH_REF)
}

fn parse_seal_args(args: Vec<String>) -> std::result::Result<(), String> {
    let mut allow_no_audit = false;
    for arg in args {
        match arg.as_str() {
            "--allow-no-audit" => allow_no_audit = true,
            other => return Err(format!("unknown seal argument: {other}")),
        }
    }
    if !allow_no_audit {
        return Err("seal scaffold requires --allow-no-audit".to_string());
    }
    Ok(())
}

fn seal_active_no_audit(
    layout: RepositoryLayout,
    ref_name: &str,
) -> std::result::Result<SealCommandResult, String> {
    let _active_lock = ActiveLock::acquire(layout.default_active_lock_path())
        .map_err(|err| err.to_string())?;
    let wal = Wal::new(layout.default_queue_wal_path());
    let replay = wal.replay().map_err(|err| err.to_string())?;
    if replay.trailing_partial_bytes != 0 {
        return Err(format!(
            "active WAL has {} trailing partial bytes; run verify/doctor before seal",
            replay.trailing_partial_bytes
        ));
    }
    if replay.records.is_empty() {
        return Err("active WAL has no patch records to seal".to_string());
    }

    let mut object_store = FileObjectStore::new(layout.clone());
    let ref_store = RefStore::new(layout.clone());
    let current = current_ref_state(&object_store, &ref_store, ref_name)?;
    let patch_ids = persist_wal_patches(&mut object_store, &replay.records)?;
    let parent_block_ids = current
        .as_ref()
        .map(|state| vec![state.target_block_id])
        .unwrap_or_default();
    let block_payload = BlockPayload {
        parent_block_ids,
        kind: if current.is_some() { BlockKind::Normal } else { BlockKind::Root },
        patch_ids: patch_ids.clone(),
        state_merkle_root: scaffold_state_root(&patch_ids),
        snapshot_blob_ref: None,
    };
    let block_envelope = signed_envelope(
        ObjectType::Block,
        block_payload.to_canonical_bytes().map_err(|err| err.to_string())?,
        SignerRole::Maintainer,
        DEV_MAINTAINER_KEY_ID,
        b"prikk.dev.block-signature.v1",
    )?;
    let block_id = object_store.write_object(&block_envelope).map_err(|err| err.to_string())?;
    let update_seq = current.as_ref().map(|state| state.update_seq + 1).unwrap_or(1);
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
        ref_state_payload.to_canonical_bytes().map_err(|err| err.to_string())?,
        SignerRole::Maintainer,
        DEV_MAINTAINER_KEY_ID,
        b"prikk.dev.ref-state-signature.v1",
    )?;
    let ref_state_id = ref_state_envelope.object_id();
    let ref_update_payload = RefUpdatePayload {
        ref_name: ref_name.to_string(),
        old_ref_state_id: previous_ref_state_id,
        new_ref_state_id: ref_state_id,
        new_target_object_id: block_id,
        update_seq,
        created_at: 0,
        author_key_id: DEV_MAINTAINER_KEY_ID.to_string(),
    };
    let ref_update_envelope = signed_envelope(
        ObjectType::RefUpdate,
        ref_update_payload.to_canonical_bytes().map_err(|err| err.to_string())?,
        SignerRole::Maintainer,
        DEV_MAINTAINER_KEY_ID,
        b"prikk.dev.ref-update-signature.v1",
    )?;
    let publication = RefPublication {
        ref_name: ref_name.to_string(),
        expected_previous_ref_state_id: previous_ref_state_id,
        ref_state: ref_state_envelope,
        ref_update: ref_update_envelope,
    };
    let published_ref_state_id = ref_store.publish(&publication).map_err(|err| err.to_string())?;
    wal.truncate_empty().map_err(|err| err.to_string())?;
    Ok(SealCommandResult {
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
        let id = object_store.write_object(&record.envelope).map_err(|err| err.to_string())?;
        patch_ids.push(id);
    }
    Ok(patch_ids)
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
    role: SignerRole,
    key_id: &str,
    dev_salt: &[u8],
) -> std::result::Result<ObjectEnvelope, String> {
    let mut envelope = ObjectEnvelope::unsigned(object_type, 1, canonical_payload);
    let object_id = envelope.object_id();
    let mut signature_preimage = Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        object_type,
        object_id,
        role,
        key_id,
    );
    signature_preimage.extend_from_slice(dev_salt);
    envelope
        .add_signature(Signature {
            algorithm: SignatureAlgorithm::Ed25519,
            key_id: key_id.to_string(),
            signature_bytes: sha256(&signature_preimage).to_vec(),
            created_at: 0,
            signer_role: role,
        })
        .map_err(|err| err.to_string())?;
    Ok(envelope)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CurrentRefState {
    ref_state_id: prikk_object::ObjectId,
    target_block_id: prikk_object::ObjectId,
    update_seq: u64,
}
