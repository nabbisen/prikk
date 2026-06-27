//! Minimal patch replay planning for supported file-level operations.
//!
//! PR-024 keeps a deliberately narrow replay boundary. It can reconstruct an in-memory snapshot
//! manifest by walking a single-parent block chain and applying `CreateFile`, `DeleteFile`,
//! `ReplaceBinary`, and full-file exact-span `EditText` operations. Arbitrary text-span discovery,
//! renames, chmod, symlinks, merge algebra, and conflict handling remain later increments.

use std::collections::{BTreeMap, HashSet};

pub(crate) mod decode;

use prikk_error::{PrikkError, Result};
use prikk_object::{
    text_span_hash, BlobPayload, BlockPayload, CanonicalEncode, ObjectEnvelope, ObjectId,
    ObjectType, RefStatePayload,
};

use crate::layout::RepositoryLayout;
use crate::object_store::FileObjectStore;
use crate::path::RepoPath;
use crate::refs::RefStore;
use crate::snapshot::{SnapshotEntry, SnapshotManifest};

use decode::{decode_supported_patch_operations, SupportedPatchOperation};

/// Read-only result of replaying supported patch operations to an in-memory snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchReplayPlan {
    /// Ref used as the checkout target.
    pub ref_name: String,
    /// Target block ID.
    pub target_block_id: ObjectId,
    /// Number of blocks replayed from oldest to newest.
    pub block_count: usize,
    /// Number of patch objects replayed.
    pub patch_count: usize,
    /// Number of supported operations applied.
    pub applied_operation_count: usize,
    /// Number of files in the resulting manifest.
    pub file_count: usize,
    /// Total content bytes in the resulting manifest.
    pub total_content_bytes: u64,
    /// Repository-relative paths in the resulting manifest.
    pub paths: Vec<String>,
}

/// Replay the supported operation subset for a ref without writing the worktree.
pub fn prepare_patch_replay_plan(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<PatchReplayPlan> {
    let snapshot = replay_supported_patch_chain(layout, ref_name)?;
    let paths = snapshot
        .manifest
        .files
        .iter()
        .map(|entry| entry.path.as_str().to_string())
        .collect();
    Ok(PatchReplayPlan {
        ref_name: snapshot.ref_name,
        target_block_id: snapshot.target_block_id,
        block_count: snapshot.block_count,
        patch_count: snapshot.patch_count,
        applied_operation_count: snapshot.applied_operation_count,
        file_count: snapshot.manifest.files.len(),
        total_content_bytes: snapshot.manifest.total_content_bytes(),
        paths,
    })
}

/// In-memory replay result used by patch checkout materialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PatchReplaySnapshot {
    /// Ref used as the replay target.
    pub(crate) ref_name: String,
    /// Target block ID.
    pub(crate) target_block_id: ObjectId,
    /// Number of blocks replayed from oldest to newest.
    pub(crate) block_count: usize,
    /// Number of patch objects replayed.
    pub(crate) patch_count: usize,
    /// Number of supported operations applied.
    pub(crate) applied_operation_count: usize,
    /// Resulting file manifest.
    pub(crate) manifest: SnapshotManifest,
    /// Files explicitly removed by replayed patches and still absent in the final manifest.
    pub(crate) deleted_files: Vec<PatchReplayDeletedFile>,
    /// Latest snapshot baseline used as the rollback-preview target for the supported replay
    /// window.
    pub(crate) baseline_manifest: SnapshotManifest,
}

/// A file explicitly deleted while replaying the supported patch subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PatchReplayDeletedFile {
    /// Validated repository-relative path that was removed.
    pub(crate) path: RepoPath,
    /// Blob ID recorded as the delete precondition.
    pub(crate) old_blob_id: ObjectId,
    /// Bytes that must still be present before an opt-in destructive delete may occur.
    pub(crate) old_bytes: Vec<u8>,
}

/// Replay the supported operation subset into a validated in-memory manifest.
pub(crate) fn replay_supported_patch_chain(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<PatchReplaySnapshot> {
    let object_store = FileObjectStore::new(layout.clone());
    let target_block_id = current_target_block(layout, &object_store, ref_name)?;
    let block_ids = single_parent_chain(&object_store, target_block_id)?;
    let mut files = BTreeMap::new();
    let mut deleted_files = BTreeMap::new();
    let mut patch_count = 0_usize;
    let mut applied_operation_count = 0_usize;
    let mut baseline_files = BTreeMap::new();

    for block_id in &block_ids {
        let block = read_block(&object_store, *block_id)?;
        if let Some(snapshot_blob_ref) = block.snapshot_blob_ref {
            files = load_snapshot_files(&object_store, snapshot_blob_ref)?;
            baseline_files = files.clone();
            deleted_files.clear();
        }
        for patch_id in block.patch_ids {
            let patch = read_patch(&object_store, patch_id)?;
            let operations = decode_supported_patch_operations(&patch.canonical_payload)?;
            for operation in operations {
                apply_supported_operation(
                    &object_store,
                    &mut files,
                    &mut deleted_files,
                    operation,
                )?;
                applied_operation_count += 1;
            }
            patch_count += 1;
        }
    }

    Ok(PatchReplaySnapshot {
        ref_name: ref_name.to_string(),
        target_block_id,
        block_count: block_ids.len(),
        patch_count,
        applied_operation_count,
        manifest: files_to_manifest(files)?,
        deleted_files: deleted_files.into_values().collect(),
        baseline_manifest: files_to_manifest(baseline_files)?,
    })
}

fn current_target_block(
    layout: &RepositoryLayout,
    object_store: &FileObjectStore,
    ref_name: &str,
) -> Result<ObjectId> {
    let ref_store = RefStore::new(layout.clone());
    let ref_state_id = ref_store.read_current_ref_state_id(ref_name)?.ok_or_else(|| {
        PrikkError::Integrity(format!("ref {ref_name} is not published"))
    })?;
    let envelope = object_store
        .read_typed(ref_state_id, ObjectType::RefState)?
        .ok_or_else(|| {
            PrikkError::Integrity(format!(
                "ref {ref_name} points to missing RefState {ref_state_id}"
            ))
        })?;
    let ref_state = RefStatePayload::decode_canonical(&envelope.canonical_payload)?;
    if ref_state.ref_name != ref_name {
        return Err(PrikkError::Integrity(format!(
            "RefState name mismatch: expected {ref_name}, got {}",
            ref_state.ref_name
        )));
    }
    Ok(ref_state.target_object_id)
}

fn single_parent_chain(object_store: &FileObjectStore, target: ObjectId) -> Result<Vec<ObjectId>> {
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

fn read_block(object_store: &FileObjectStore, block_id: ObjectId) -> Result<BlockPayload> {
    let envelope = object_store
        .read_typed(block_id, ObjectType::Block)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing Block {block_id}")))?;
    BlockPayload::decode_canonical(&envelope.canonical_payload)
}

fn read_patch(object_store: &FileObjectStore, patch_id: ObjectId) -> Result<ObjectEnvelope> {
    object_store
        .read_typed(patch_id, ObjectType::Patch)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing Patch {patch_id}")))
}

fn load_snapshot_files(
    object_store: &FileObjectStore,
    snapshot_blob_ref: ObjectId,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let envelope = object_store
        .read_typed(snapshot_blob_ref, ObjectType::Blob)?
        .ok_or_else(|| {
            PrikkError::Integrity(format!("missing snapshot Blob {snapshot_blob_ref}"))
        })?;
    let blob = BlobPayload::decode_canonical(&envelope.canonical_payload)?;
    let manifest = SnapshotManifest::decode(&blob.bytes)?;
    let mut files = BTreeMap::new();
    for entry in manifest.files {
        files.insert(entry.path.as_str().to_string(), entry.bytes);
    }
    Ok(files)
}

fn files_to_manifest(files: BTreeMap<String, Vec<u8>>) -> Result<SnapshotManifest> {
    let mut entries = Vec::with_capacity(files.len());
    for (path, bytes) in files {
        entries.push(SnapshotEntry { path: RepoPath::parse(&path)?, bytes });
    }
    Ok(SnapshotManifest { files: entries })
}

fn apply_supported_operation(
    object_store: &FileObjectStore,
    files: &mut BTreeMap<String, Vec<u8>>,
    deleted_files: &mut BTreeMap<String, PatchReplayDeletedFile>,
    operation: SupportedPatchOperation,
) -> Result<()> {
    match operation {
        SupportedPatchOperation::CreateFile { path, blob_id } => {
            if files.contains_key(&path) {
                return Err(PrikkError::Integrity(format!(
                    "CreateFile would overwrite existing path {path}"
                )));
            }
            let bytes = read_blob_bytes(object_store, blob_id)?;
            deleted_files.remove(&path);
            files.insert(path, bytes);
        }
        SupportedPatchOperation::DeleteFile { path, old_blob_id } => {
            let old_bytes = files.get(&path).ok_or_else(|| {
                PrikkError::Integrity(format!("DeleteFile path is absent: {path}"))
            })?;
            ensure_blob_matches(old_bytes, old_blob_id)?;
            let repo_path = RepoPath::parse(&path)?;
            let deleted = PatchReplayDeletedFile {
                path: repo_path,
                old_blob_id,
                old_bytes: old_bytes.clone(),
            };
            files.remove(&path);
            deleted_files.insert(path, deleted);
        }
        SupportedPatchOperation::ReplaceBinary { path, old_blob_id, new_blob_id } => {
            let old_bytes = files.get(&path).ok_or_else(|| {
                PrikkError::Integrity(format!("ReplaceBinary path is absent: {path}"))
            })?;
            ensure_blob_matches(old_bytes, old_blob_id)?;
            let new_bytes = read_blob_bytes(object_store, new_blob_id)?;
            files.insert(path, new_bytes);
        }
        SupportedPatchOperation::EditText { path, anchor_id, old_span_hash, replacement } => {
            apply_full_file_text_edit(files, path, anchor_id, old_span_hash, replacement)?;
        }
    }
    Ok(())
}

fn apply_full_file_text_edit(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: String,
    anchor_id: String,
    old_span_hash: [u8; 32],
    replacement: String,
) -> Result<()> {
    if anchor_id != "full-file" {
        return Err(PrikkError::UnsupportedObjectType(format!(
            "unsupported EditText anchor {anchor_id}; only full-file is supported"
        )));
    }
    let old_bytes = files
        .get(&path)
        .ok_or_else(|| PrikkError::Integrity(format!("EditText path is absent: {path}")))?;
    if std::str::from_utf8(old_bytes).is_err() {
        return Err(PrikkError::Integrity(format!(
            "EditText target is not valid UTF-8 text: {path}"
        )));
    }
    let actual_hash = text_span_hash(old_bytes);
    if actual_hash != old_span_hash {
        return Err(PrikkError::Integrity(format!(
            "EditText old_span_hash mismatch for {path}"
        )));
    }
    files.insert(path, replacement.into_bytes());
    Ok(())
}

fn read_blob_bytes(object_store: &FileObjectStore, blob_id: ObjectId) -> Result<Vec<u8>> {
    let envelope = object_store
        .read_typed(blob_id, ObjectType::Blob)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing Blob {blob_id}")))?;
    let blob = BlobPayload::decode_canonical(&envelope.canonical_payload)?;
    Ok(blob.bytes)
}

fn ensure_blob_matches(bytes: &[u8], expected: ObjectId) -> Result<()> {
    let payload = BlobPayload { bytes: bytes.to_vec() };
    let id = ObjectId::from_canonical_payload(
        ObjectType::Blob,
        1,
        &payload.to_canonical_bytes()?,
    );
    if id == expected {
        return Ok(());
    }
    Err(PrikkError::Integrity(format!(
        "operation old_blob_id mismatch: expected {expected}, got {id}"
    )))
}

