//! Minimal patch replay planning for supported file-level operations.
//!
//! PR-024 keeps a deliberately narrow replay boundary. It can reconstruct an in-memory snapshot
//! manifest by walking a single-parent block chain and applying `CreateFile` and `DeleteNode`
//! operations. `EditText` and `ReplaceBinary` are reconciled to the FDD-03 §9.3 node-addressed
//! records but their application is deferred to the node model (increment 4.4; `EditText` also
//! needs FDD-01 §7.2.1 span anchoring); renames, chmod, symlinks, merge algebra, and conflict
//! handling remain later increments.

use std::collections::{BTreeMap, HashSet};

pub(crate) mod decode;

use prikk_error::{PrikkError, Result};
use prikk_object::{
    BlobKind, BlobPayload, BlockPayload, NodeId, NodeKind, ObjectEnvelope, ObjectId, ObjectType,
    RefStatePayload, text_span_hash,
};

use crate::checkout::DEFAULT_CHECKOUT_REF;
use crate::layout::RepositoryLayout;
use crate::object_store::FileObjectStore;
use crate::path::RepoPath;
use crate::refs::RefStore;
use crate::snapshot::{SnapshotEntry, SnapshotManifest};
use crate::text_span;

use decode::{
    DecodedDeletePreimage, DecodedOperationKind, DecodedPatchOperation, decode_patch_operations,
    ensure_apply_supported,
};

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
    let mut live_nodes = BTreeMap::new();
    let mut deleted_files = BTreeMap::new();
    let mut patch_count = 0_usize;
    let mut applied_operation_count = 0_usize;
    let mut baseline_files = BTreeMap::new();

    for block_id in &block_ids {
        let block = read_block(&object_store, *block_id)?;
        if let Some(snapshot_blob_ref) = block.snapshot_blob_ref {
            files = load_snapshot_files(&object_store, snapshot_blob_ref)?;
            live_nodes.clear();
            baseline_files = files.clone();
            deleted_files.clear();
        }
        for patch_id in block.patch_ids {
            let patch = read_patch(&object_store, patch_id)?;
            let operations = decode_patch_operations(&patch.canonical_payload)?;
            for operation in operations {
                apply_decoded_operation(
                    &object_store,
                    &mut files,
                    &mut live_nodes,
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

/// Resolve the node-addressed lineage bounds for a ref: the current target block (baseline) and the
/// lineage genesis (horizon). Worktree authoring (4.4a-2) supplies these to `replay_derived_state`
/// so the baseline node lifecycle state is reconstructed from authoritative node-addressed history,
/// never from a snapshot manifest. Fails closed on a multi-parent lineage (v1 single-parent only).
pub(crate) fn resolve_node_lineage_bounds(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<(ObjectId, ObjectId)> {
    let object_store = FileObjectStore::new(layout.clone());
    let baseline = current_target_block(layout, &object_store, ref_name)?;
    let chain = single_parent_chain(&object_store, baseline)?;
    let horizon = *chain
        .first()
        .ok_or_else(|| PrikkError::Integrity(format!("ref {ref_name} lineage is empty")))?;
    Ok((baseline, horizon))
}

/// Baseline context for worktree authoring: either a published node-addressed lineage, or a genesis
/// (first-commit) context with no baseline at all.
pub(crate) enum WorktreeBaseline {
    /// The ref is published; author against replay-derived node lifecycle state.
    Published {
        /// Current target block (baseline).
        baseline_block: ObjectId,
        /// Lineage genesis (horizon).
        horizon: ObjectId,
    },
    /// The ref has never been published; author against an empty baseline (all `CreateFile`).
    Genesis,
}

/// Decide whether worktree authoring runs against a published lineage or a genesis (first-commit)
/// context (DC-09 4.4b). Genesis is selected **only** when the target ref has never been published:
/// the ref pointer is absent **and** the ref log is readable and empty. A missing pointer with any
/// ref-log history, or an unreadable/partial ref log, is treated as corruption — not genesis — and
/// fails closed pointing at `doctor` (design §4, review E2). The active-WAL guard (review E1) is the
/// authoring caller's responsibility and is enforced there.
pub(crate) fn resolve_worktree_baseline(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<WorktreeBaseline> {
    let ref_store = RefStore::new(layout.clone());
    if ref_store.read_current_ref_state_id(ref_name)?.is_some() {
        let (baseline_block, horizon) = resolve_node_lineage_bounds(layout, ref_name)?;
        return Ok(WorktreeBaseline::Published {
            baseline_block,
            horizon,
        });
    }
    // Pointer absent: genesis only if the log is readable and empty; otherwise corruption.
    // Genesis is scoped to the default ref this increment (review Q2); a non-default unpublished ref
    // is not a genesis case — branch creation / non-default first-commit is a separate design.
    if ref_name != DEFAULT_CHECKOUT_REF {
        return Err(PrikkError::InvalidName(format!(
            "first commit is supported only for {DEFAULT_CHECKOUT_REF}; ref {ref_name} is not \
             published and branch creation is not implemented yet"
        )));
    }
    let log = ref_store.replay_log(ref_name).map_err(|err| {
        PrikkError::Integrity(format!(
            "ref {ref_name} log is unreadable; run `prikk doctor` before committing ({err})"
        ))
    })?;
    if log.trailing_partial_bytes != 0 {
        return Err(PrikkError::Integrity(format!(
            "ref {ref_name} pointer is missing and its log has trailing partial bytes; \
             run `prikk doctor` (this is not a genesis repository)"
        )));
    }
    if !log.records.is_empty() {
        return Err(PrikkError::Integrity(format!(
            "ref {ref_name} pointer is missing but ref-log history exists; \
             run `prikk doctor --repair-main-ref` (this is not a genesis repository)"
        )));
    }
    Ok(WorktreeBaseline::Genesis)
}

fn current_target_block(
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
    let snapshot_content = crate::blob_access::decode_snapshot_blob(&envelope.canonical_payload)?;
    let manifest = SnapshotManifest::decode(&snapshot_content)?;
    let mut files = BTreeMap::new();
    for entry in manifest.files {
        files.insert(entry.path.as_str().to_string(), entry.bytes);
    }
    Ok(files)
}

fn files_to_manifest(files: BTreeMap<String, Vec<u8>>) -> Result<SnapshotManifest> {
    let mut entries = Vec::with_capacity(files.len());
    for (path, bytes) in files {
        entries.push(SnapshotEntry {
            path: RepoPath::parse(&path)?,
            bytes,
        });
    }
    Ok(SnapshotManifest { files: entries })
}

fn apply_decoded_operation(
    object_store: &FileObjectStore,
    files: &mut BTreeMap<String, Vec<u8>>,
    live_nodes: &mut BTreeMap<NodeId, ReplayLiveNode>,
    deleted_files: &mut BTreeMap<String, PatchReplayDeletedFile>,
    operation: DecodedPatchOperation,
) -> Result<()> {
    // Erratum P1: decode success does not imply applicability. The apply-supported
    // subset is gated here as the single source of truth; the match below only needs
    // to handle the kinds the gate admits.
    ensure_apply_supported(&operation)?;
    match operation.kind {
        DecodedOperationKind::CreateFile {
            path,
            node_id,
            blob_id,
            mode: _,
        } => {
            if files.contains_key(&path) {
                return Err(PrikkError::Integrity(format!(
                    "CreateFile would overwrite existing path {path}"
                )));
            }
            if live_nodes.contains_key(&node_id) {
                return Err(PrikkError::Integrity(
                    "CreateFile would introduce an already-live node_id".to_string(),
                ));
            }
            let (kind, bytes) = read_blob_bytes_with_kind(object_store, blob_id)?;
            deleted_files.remove(&path);
            files.insert(path.clone(), bytes);
            live_nodes.insert(node_id, ReplayLiveNode { path, kind });
        }
        DecodedOperationKind::DeleteNode {
            path,
            node_id,
            preimage:
                DecodedDeletePreimage::File {
                    old_node_kind,
                    old_blob_id,
                    old_mode: _,
                },
        } => {
            let old_bytes = files.get(&path).ok_or_else(|| {
                PrikkError::Integrity(format!("DeleteNode path is absent: {path}"))
            })?;
            crate::blob_access::ensure_blob_matches_node_kind(
                old_bytes,
                old_blob_id,
                old_node_kind,
            )?;
            let repo_path = RepoPath::parse(&path)?;
            let deleted = PatchReplayDeletedFile {
                path: repo_path,
                old_blob_id,
                old_bytes: old_bytes.clone(),
            };
            files.remove(&path);
            if let Some(live) = live_nodes.remove(&node_id) {
                if live.path != path {
                    return Err(PrikkError::Integrity(format!(
                        "DeleteNode path {path} does not match live node path {}",
                        live.path
                    )));
                }
                if live.kind != old_node_kind {
                    return Err(PrikkError::Integrity(
                        "DeleteNode old_node_kind does not match live node kind".to_string(),
                    ));
                }
            }
            deleted_files.insert(path, deleted);
        }
        DecodedOperationKind::EditText {
            node_id,
            span_id,
            old_span_hash,
            left_anchor_hash,
            right_anchor_hash,
            replacement_text,
            old_span_text,
        } => {
            apply_edit_text(
                files,
                live_nodes,
                node_id,
                &span_id,
                &old_span_hash,
                &left_anchor_hash,
                &right_anchor_hash,
                &replacement_text,
                &old_span_text,
            )?;
        }
        _ => unreachable!(
            "ensure_apply_supported admits only CreateFile, file-DeleteNode, and EditText for replay"
        ),
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplayLiveNode {
    path: String,
    kind: NodeKind,
}

#[allow(clippy::too_many_arguments)]
fn apply_edit_text(
    files: &mut BTreeMap<String, Vec<u8>>,
    live_nodes: &BTreeMap<NodeId, ReplayLiveNode>,
    node_id: NodeId,
    span_id: &[u8; 32],
    old_span_hash: &[u8; 32],
    left_anchor_hash: &[u8; 32],
    right_anchor_hash: &[u8; 32],
    replacement_text: &[u8],
    old_span_text: &[u8],
) -> Result<()> {
    if text_span_hash(old_span_text) != *old_span_hash {
        return Err(PrikkError::Integrity(format!(
            "EditText hash verification failed before localization for node {} span {}",
            hex32(node_id.as_bytes()),
            hex32(span_id)
        )));
    }
    let live = live_nodes.get(&node_id).ok_or_else(|| {
        PrikkError::Integrity(format!(
            "EditText failed before blob load: node {} is not live (span {})",
            hex32(node_id.as_bytes()),
            hex32(span_id)
        ))
    })?;
    if live.kind != NodeKind::TextFile {
        return Err(PrikkError::Integrity(format!(
            "EditText failed before blob load: node {} is {:?}, not TextFile (span {})",
            hex32(node_id.as_bytes()),
            live.kind,
            hex32(span_id)
        )));
    }
    let current_text = files.get(&live.path).ok_or_else(|| {
        PrikkError::Integrity(format!(
            "EditText failed before blob load: live node {} path {} is absent (span {})",
            hex32(node_id.as_bytes()),
            live.path,
            hex32(span_id)
        ))
    })?;
    if core::str::from_utf8(current_text).is_err() {
        return Err(PrikkError::Integrity(format!(
            "EditText failed during UTF-8 validation for node {} span {}",
            hex32(node_id.as_bytes()),
            hex32(span_id)
        )));
    }
    let (start, end) = text_span::locate_text_span(
        current_text,
        old_span_text,
        left_anchor_hash,
        right_anchor_hash,
        span_id,
        node_id,
        old_span_hash,
    )
    .map_err(|reason| {
        PrikkError::Integrity(format!(
            "EditText failed during localization for node {} span {}: {reason}",
            hex32(node_id.as_bytes()),
            hex32(span_id)
        ))
    })?;
    let new_text =
        text_span::splice_text(current_text, start, end, replacement_text).map_err(|err| {
            PrikkError::Integrity(format!(
                "EditText failed during splice for node {} span {}: {err}",
                hex32(node_id.as_bytes()),
                hex32(span_id)
            ))
        })?;
    files.insert(live.path.clone(), new_text);
    Ok(())
}

fn read_blob_bytes_with_kind(
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

fn hex32(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests;
