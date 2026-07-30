//! Object-store reading helpers for patch replay: block-chain walking and blob/patch/snapshot
//! loading. Split out of `patch_replay.rs` (DC-58) — no behaviour change, all items moved verbatim.

use std::collections::{BTreeMap, HashSet};

use prikk_error::{PrikkError, Result};
use prikk_object::{
    BlobKind, BlobPayload, BlockPayload, NodeKind, ObjectEnvelope, ObjectId, ObjectType,
    RefStatePayload,
};

use crate::layout::RepositoryLayout;
use crate::object_store::FileObjectStore;
use crate::path::RepoPath;
use crate::refs::RefStore;
use crate::snapshot::{SnapshotEntry, SnapshotManifest};

pub(super) fn current_target_block(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    ref_name: &str,
) -> Result<ObjectId> {
    let ref_store = RefStore::new(layout.clone());
    let ref_state_id = ref_store
        .read_current_ref_state_id(ref_name)?
        .ok_or_else(|| PrikkError::Integrity(format!("ref {ref_name} is not published")))?;
    let envelope = object_store
        .read_typed(ref_state_id, ObjectType::RefState)?
        .ok_or_else(|| {
            PrikkError::Integrity(format!(
                "ref {ref_name} points to missing RefState {ref_state_id}"
            ))
        })?;
    let ref_state =
        RefStatePayload::decode_canonical(&envelope.canonical_payload, envelope.schema_version)?;
    if ref_state.ref_name != ref_name {
        return Err(PrikkError::Integrity(format!(
            "RefState name mismatch: expected {ref_name}, got {}",
            ref_state.ref_name
        )));
    }
    Ok(ref_state.target_object_id)
}

pub(super) fn single_parent_chain(
    object_store: &FileObjectStore,
    target: ObjectId,
) -> Result<Vec<ObjectId>> {
    let mut newest_first = Vec::new();
    let mut seen = HashSet::new();
    let mut current = Some(target);
    while let Some(block_id) = current {
        if !seen.insert(block_id) {
            return Err(PrikkError::Integrity(format!(
                "block parent chain contains a cycle at {block_id}"
            )));
        }
        let block = read_block(object_store, block_id)?;
        if block.parent_block_ids.len() > 1 {
            return Err(PrikkError::UnsupportedObjectType(format!(
                "patch replay supports only single-parent chains; block {block_id} has {} parents",
                block.parent_block_ids.len()
            )));
        }
        newest_first.push(block_id);
        current = block.parent_block_ids.first().copied();
    }
    newest_first.reverse();
    Ok(newest_first)
}

pub(super) fn read_block(
    object_store: &FileObjectStore,
    block_id: ObjectId,
) -> Result<BlockPayload> {
    let envelope = object_store
        .read_typed(block_id, ObjectType::Block)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing Block {block_id}")))?;
    BlockPayload::decode_canonical(&envelope.canonical_payload)
}

pub(super) fn read_patch(
    object_store: &FileObjectStore,
    patch_id: ObjectId,
) -> Result<ObjectEnvelope> {
    object_store
        .read_typed(patch_id, ObjectType::Patch)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing Patch {patch_id}")))
}

pub(super) fn load_snapshot_files(
    object_store: &FileObjectStore,
    snapshot_blob_ref: ObjectId,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let envelope = object_store
        .read_typed(snapshot_blob_ref, ObjectType::Blob)?
        .ok_or_else(|| {
            PrikkError::Integrity(format!("missing snapshot Blob {snapshot_blob_ref}"))
        })?;
    let snapshot_content = crate::blob_access::decode_snapshot_blob(&envelope.canonical_payload)?;
    let manifest = SnapshotManifest::decode(&snapshot_content)?;
    let mut files = BTreeMap::new();
    for entry in manifest.files {
        files.insert(entry.path.as_str().to_string(), entry.bytes);
    }
    Ok(files)
}

pub(super) fn files_to_manifest(files: BTreeMap<String, Vec<u8>>) -> Result<SnapshotManifest> {
    let mut entries = Vec::with_capacity(files.len());
    for (path, bytes) in files {
        entries.push(SnapshotEntry {
            path: RepoPath::parse(&path)?,
            bytes,
        });
    }
    Ok(SnapshotManifest { files: entries })
}

pub(super) fn read_blob_bytes_with_kind(
    object_store: &FileObjectStore,
    blob_id: ObjectId,
) -> Result<(NodeKind, Vec<u8>)> {
    let envelope = object_store
        .read_typed(blob_id, ObjectType::Blob)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing Blob {blob_id}")))?;
    let blob = BlobPayload::decode_canonical(&envelope.canonical_payload)?;
    if blob.blob_kind == BlobKind::Snapshot {
        return Err(PrikkError::Integrity(
            "file content reference points to a SNAPSHOT blob".to_string(),
        ));
    }
    let kind = NodeKind::from_file_blob_kind(blob.blob_kind)?;
    Ok((kind, blob.content))
}
