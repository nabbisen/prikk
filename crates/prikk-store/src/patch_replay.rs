//! Minimal patch replay planning for supported file-level operations.
//!
//! PR-024 keeps a deliberately narrow replay boundary. It can reconstruct an in-memory snapshot
//! manifest by walking a single-parent block chain and applying `CreateFile`, `DeleteNode`,
//! `EditText`, `ReplaceBinary`, and `ChangePerm` operations (DC-73 wired the last two — see
//! `apply.rs` and `decode.rs::ensure_apply_supported`). Renames and symlinks remain unauthored
//! (`node_authoring.rs` never produces `RenamePath`; symlink authoring is refused outright), so
//! their apply paths stay deferred pending an authoring path, not the node model; merge algebra and
//! conflict handling remain later increments.
//!
//! Split across three files (DC-58): this file keeps the public API and baseline resolution;
//! `read.rs` holds object-store reading helpers (block-chain walking, blob/patch/snapshot
//! loading); `apply.rs` holds the per-operation state-fold logic. `decode.rs` (pre-existing) is
//! unchanged. No behaviour or public path changed by the split.

use std::collections::BTreeMap;

mod apply;
pub(crate) mod decode;
mod read;

use prikk_error::{PrikkError, Result};
use prikk_object::ObjectId;

use crate::layout::RepositoryLayout;
use crate::object_store::FileObjectStore;
use crate::path::RepoPath;
use crate::refs::RefStore;
use crate::snapshot::SnapshotManifest;
use crate::validate_local_branch_ref;

use apply::apply_decoded_operation;
use decode::decode_patch_operations;
use read::{
    current_target_block, files_to_manifest, files_to_replay_manifest, load_snapshot_files,
    read_block, read_patch, single_parent_chain,
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

/// One file entry in a replay-derived manifest, carrying the mode bits `CreateFile`/`ChangePerm`
/// recorded (DC-73). Deliberately **not** `crate::snapshot::SnapshotEntry`: that type is also what
/// `SnapshotManifest::decode` reads from a stored snapshot Blob's wire bytes, which have no mode
/// field of their own — adding one to the shared type would force a default on the decode side for
/// a value the stored bytes never contained. This type exists only in memory, built by replaying
/// operations, and never crosses the object-format boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplayManifestEntry {
    /// Validated repository-relative path.
    pub(crate) path: RepoPath,
    /// File content bytes.
    pub(crate) bytes: Vec<u8>,
    /// Mode bits, as recorded by the most recent `CreateFile`/`ChangePerm` for this node.
    pub(crate) mode: u32,
}

/// Replay-derived manifest, sorted by path. See [`ReplayManifestEntry`] for why this is not
/// `crate::snapshot::SnapshotManifest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplayManifest {
    /// File entries, sorted by path.
    pub(crate) files: Vec<ReplayManifestEntry>,
}

impl ReplayManifest {
    pub(crate) fn total_content_bytes(&self) -> u64 {
        self.files
            .iter()
            .map(|entry| entry.bytes.len() as u64)
            .sum()
    }
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
    /// Resulting file manifest, mode-aware (DC-73).
    pub(crate) manifest: ReplayManifest,
    /// Files explicitly removed by replayed patches and still absent in the final manifest.
    pub(crate) deleted_files: Vec<PatchReplayDeletedFile>,
    /// Latest snapshot baseline used as the rollback-preview target for the supported replay
    /// window. Mode-unaware like the wire-decoded snapshot format it is seeded from — the
    /// rollback-preview consumer compares paths and bytes only.
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
        manifest: files_to_replay_manifest(files, &live_nodes)?,
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
/// ref-log history, or an unreadable/partial ref log, is treated as corruption - not genesis - and
/// fails closed with preserve/restore guidance (design §4, review E2). The active-WAL guard (review E1) is the
/// authoring caller's responsibility and is enforced there.
pub(crate) fn resolve_worktree_baseline(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<WorktreeBaseline> {
    let canonical_ref = validate_local_branch_ref(ref_name)?;
    let ref_store = RefStore::new(layout.clone());
    if ref_store
        .read_current_ref_state_id(&canonical_ref)?
        .is_some()
    {
        let (baseline_block, horizon) = resolve_node_lineage_bounds(layout, &canonical_ref)?;
        return Ok(WorktreeBaseline::Published {
            baseline_block,
            horizon,
        });
    }
    // Pointer absent: genesis only if the log is readable and empty; otherwise corruption.
    // An absent log is decoded as an empty log by `RefStore::replay_log`; unreadable, malformed, or
    // partial logs remain corruption, not genesis.
    let log = ref_store.replay_log(&canonical_ref).map_err(|err| {
        PrikkError::Integrity(format!(
            "ref {canonical_ref} log is unreadable; run `prikk doctor` before committing ({err})"
        ))
    })?;
    if log.trailing_partial_bytes != 0 {
        return Err(PrikkError::Integrity(format!(
            "ref {canonical_ref} pointer is missing and its log has trailing partial bytes; \
             run `prikk doctor` (this is not a genesis repository)"
        )));
    }
    if !log.records.is_empty() {
        return Err(PrikkError::Integrity(format!(
            "ref {canonical_ref} pointer is missing but ref-log history exists; \
             preserve the repository and restore from backup (this is not a genesis repository)"
        )));
    }
    Ok(WorktreeBaseline::Genesis)
}

#[cfg(test)]
mod tests;
