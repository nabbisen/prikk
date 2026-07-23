//! Authoritative lifecycle replay — lineage walker and state-effect interpreter
//! (DC-09 Phase 4.4-2c-2a..2c-2b).
//!
//! Walks the v1 single-parent block lineage from a baseline back to a genesis horizon over the
//! **real** object store, failing closed on every malformed-lineage condition, then applies each
//! block's patch operations in apply order (oldest first) into a [`NodeLifecycleState`].
//!
//! Per the O1 ruling, replay must apply every lifecycle-affecting state effect *exactly* or fail
//! closed. As of 2c-2d **all** lifecycle-affecting operations have exact effects: `CreateFile`,
//! `CreateSymlink`, `DeleteNode`, `RenamePath`, `ChangePerm`, `ReplaceBinary`, and `EditText` (the
//! last derives a new full-text `BlobPayload(Text, …)` content id by materializing the edited
//! text).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use prikk_error::PrikkError;
use prikk_object::{
    BlobKind, BlockPayload, NodeId, NodeKind, ObjectId, ObjectType, text_span_hash,
};

use super::{BlobContentResolver, BlobKindResolver, StoreBackedResolver};
use crate::node_lifecycle::{LiveNode, NodeContent, NodeLifecycleState};
use crate::object_store::ObjectReader;
use crate::patch_replay::decode::{
    DecodedDeletePreimage, DecodedOperationKind, decode_patch_operations,
};
use crate::path::RepoPath;
use crate::text_span::{self, TextSpanResolutionFailure};

/// Replay-local materialized text for edited text nodes, keyed by `node_id`. Transient to a replay
/// pass; never part of the persisted lifecycle index (which stores only `blob_id` + `mode`).
type TextCache = BTreeMap<NodeId, Vec<u8>>;

/// Structured lifecycle-replay error taxonomy (carry-forward P2-3).
///
/// These are the classes a replay / fallback caller branches on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LifecycleReplayError {
    /// A block referenced by the lineage walk is absent. Never treated as genesis (P2-1).
    MissingBlockInLineage { block_id: ObjectId },
    /// A block object exists but cannot be read as a `Block` (wrong type, decode failure).
    UnreadableBlockInLineage { block_id: ObjectId, detail: String },
    /// A block in the v1 window has more than one parent. Merge windows are deferred (DC-13).
    MergeLineageUnsupported {
        block_id: ObjectId,
        parent_count: usize,
    },
    /// The single-parent walk revisited a block — a cycle in a store that should be a DAG.
    LineageCycle { block_id: ObjectId },
    /// The walk reached genesis, but genesis is not the claimed horizon (v1 adequate-horizon).
    HorizonNotInLineage { horizon_id: ObjectId },
    /// A decoded operation could not be applied to the replayed state: the target node is not
    /// live, a path is occupied, restoration-equivalence failed, or a stated old-state field
    /// (mode/path) disagrees with the replayed reality. Distinct from a decode failure.
    InconsistentLifecycleEffect { detail: String },
    /// A patch referenced by a block is missing or cannot be decoded.
    MalformedPatchInLineage { patch_id: ObjectId, detail: String },
    /// A blob required to resolve a lifecycle state effect is absent. (2c-2b onward.)
    MissingBlobForLifecycleEffect { blob_id: ObjectId },
    /// An `EditText` span could not be uniquely localized in the replayed text during sealed-history
    /// replay. This is an integrity failure (the sealed edit applied cleanly when authored), not a
    /// user/merge conflict.
    TextSpanResolutionFailed {
        node_id: NodeId,
        span_id: [u8; 32],
        reason: TextSpanResolutionFailure,
    },
}

impl fmt::Display for LifecycleReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBlockInLineage { block_id } => write!(
                f,
                "lifecycle replay: block {block_id} is missing and cannot be treated as genesis"
            ),
            Self::UnreadableBlockInLineage { block_id, detail } => {
                write!(
                    f,
                    "lifecycle replay: block {block_id} is unreadable ({detail})"
                )
            }
            Self::MergeLineageUnsupported {
                block_id,
                parent_count,
            } => write!(
                f,
                "lifecycle replay: block {block_id} has {parent_count} parents; \
                 v1 windows require a single-parent lineage (merge deferred to DC-13)"
            ),
            Self::LineageCycle { block_id } => {
                write!(f, "lifecycle replay: cycle detected at block {block_id}")
            }
            Self::HorizonNotInLineage { horizon_id } => write!(
                f,
                "lifecycle replay: walk reached genesis without crossing the claimed horizon \
                 {horizon_id}"
            ),
            Self::InconsistentLifecycleEffect { detail } => write!(
                f,
                "lifecycle replay: operation could not be applied to the replayed state ({detail})"
            ),
            Self::MalformedPatchInLineage { patch_id, detail } => {
                write!(
                    f,
                    "lifecycle replay: patch {patch_id} is malformed ({detail})"
                )
            }
            Self::MissingBlobForLifecycleEffect { blob_id } => {
                write!(
                    f,
                    "lifecycle replay: blob {blob_id} required for a state effect is missing"
                )
            }
            Self::TextSpanResolutionFailed {
                node_id,
                span_id,
                reason,
            } => {
                write!(f, "lifecycle replay: EditText span on node ")?;
                for byte in node_id.as_bytes() {
                    write!(f, "{byte:02x}")?;
                }
                write!(f, " (span_id ")?;
                for byte in span_id {
                    write!(f, "{byte:02x}")?;
                }
                write!(f, ") could not be localized: {reason}")
            }
        }
    }
}

impl From<LifecycleReplayError> for PrikkError {
    fn from(value: LifecycleReplayError) -> Self {
        Self::Integrity(value.to_string())
    }
}

/// Read a `Block` payload, distinguishing absence from unreadability so the walk can fail closed
/// with the correct structured class.
fn read_block(
    reader: &impl ObjectReader,
    block_id: ObjectId,
) -> Result<BlockPayload, LifecycleReplayError> {
    let envelope = reader.read_object(block_id).map_err(|e| {
        LifecycleReplayError::UnreadableBlockInLineage {
            block_id,
            detail: e.to_string(),
        }
    })?;
    let Some(envelope) = envelope else {
        return Err(LifecycleReplayError::MissingBlockInLineage { block_id });
    };
    if envelope.object_type != ObjectType::Block {
        return Err(LifecycleReplayError::UnreadableBlockInLineage {
            block_id,
            detail: format!("object is not a Block ({} found)", envelope.object_type),
        });
    }
    BlockPayload::decode_canonical(&envelope.canonical_payload).map_err(|e| {
        LifecycleReplayError::UnreadableBlockInLineage {
            block_id,
            detail: e.to_string(),
        }
    })
}

/// One walked block: its id and decoded payload, retained so replay does not re-read.
type WalkedBlock = (ObjectId, BlockPayload);

/// Source of lineage blocks for walking. Each implementor reads a block **once** and yields its
/// own `Block` view (the full payload for replay; just the parent ids for provenance), failing
/// closed on missing, unreadable, or non-Block objects. This is the **single** seam through which
/// both authoritative replay and cache provenance read the lineage, so the two cannot drift on
/// which blocks are in the window or in what order — and, since the walk returns what it read, on a
/// file-backed store replay never re-reads a block whose contents could have changed (E4).
pub(crate) trait LineageBlockReader {
    /// What a single lineage-block read yields to the caller.
    type Block;
    /// Read one lineage block exactly once.
    fn read_lineage_block(&self, block_id: ObjectId) -> Result<Self::Block, LifecycleReplayError>;
    /// The parent block ids of a read block (drives the single-parent walk rule).
    fn parents_of(block: &Self::Block) -> &[ObjectId];
}

/// Reader-backed lineage source: reads and **retains** the full block payload, preserving the
/// missing/unreadable distinction. Replay applies patches from the payload returned by the walk,
/// so no block is read a second time.
struct ReaderLineage<'a, R: ObjectReader>(&'a R);

impl<R: ObjectReader> LineageBlockReader for ReaderLineage<'_, R> {
    type Block = BlockPayload;

    fn read_lineage_block(&self, block_id: ObjectId) -> Result<BlockPayload, LifecycleReplayError> {
        read_block(self.0, block_id)
    }

    fn parents_of(block: &BlockPayload) -> &[ObjectId] {
        &block.parent_block_ids
    }
}

/// Canonical v1 single-parent lineage walk — the **single source of truth** for the lineage
/// window. From `baseline`, follow single parents to repository genesis (a block with no
/// parents), which MUST equal `horizon`. Returns the chain in apply order (genesis/horizon first …
/// baseline last), each entry paired with the `Block` the walk read for it (so replay does not
/// re-read). Fails closed on a merge (>1 parent), a cycle, or a genesis that is not the claimed
/// horizon. Used by both authoritative replay and cache provenance.
pub(crate) fn walk_single_parent_chain<R: LineageBlockReader>(
    blocks: &R,
    baseline: ObjectId,
    horizon: ObjectId,
) -> Result<Vec<(ObjectId, R::Block)>, LifecycleReplayError> {
    walk_single_parent_chain_inner(blocks, baseline, Some(horizon))
}

fn walk_single_parent_chain_inner<R: LineageBlockReader>(
    blocks: &R,
    baseline: ObjectId,
    expected_horizon: Option<ObjectId>,
) -> Result<Vec<(ObjectId, R::Block)>, LifecycleReplayError> {
    let mut chain: Vec<(ObjectId, R::Block)> = Vec::new();
    let mut visited: BTreeSet<ObjectId> = BTreeSet::new();
    let mut current = baseline;

    loop {
        if !visited.insert(current) {
            return Err(LifecycleReplayError::LineageCycle { block_id: current });
        }
        let block = blocks.read_lineage_block(current)?;
        let next = match R::parents_of(&block) {
            [] => None,
            [parent] => Some(*parent),
            other => {
                return Err(LifecycleReplayError::MergeLineageUnsupported {
                    block_id: current,
                    parent_count: other.len(),
                });
            }
        };
        chain.push((current, block));
        match next {
            None => {
                if expected_horizon.is_some_and(|horizon| current != horizon) {
                    return Err(LifecycleReplayError::HorizonNotInLineage {
                        horizon_id: expected_horizon.unwrap_or(current),
                    });
                }
                break;
            }
            Some(parent) => current = parent,
        }
    }

    chain.reverse();
    Ok(chain)
}

fn walk_lineage_to_genesis(
    reader: &impl ObjectReader,
    baseline: ObjectId,
) -> Result<Vec<WalkedBlock>, LifecycleReplayError> {
    walk_single_parent_chain_inner(&ReaderLineage(reader), baseline, None)
}

/// Walk the v1 single-parent lineage from `baseline` back to the genesis `horizon`, returning the
/// chain in **apply order** with each block's payload — read exactly once by the walk and retained
/// for patch application. Built on the shared [`walk_single_parent_chain`]; performs no second
/// block read (E4).
fn walk_lineage(
    reader: &impl ObjectReader,
    baseline: ObjectId,
    horizon: ObjectId,
) -> Result<Vec<WalkedBlock>, LifecycleReplayError> {
    walk_single_parent_chain(&ReaderLineage(reader), baseline, horizon)
}

/// Replay the lineage from `baseline` back to `horizon`, applying each operation's lifecycle
/// state effect into a [`NodeLifecycleState`]. Returns the reconstructed state.
///
/// **This is not yet `ReplayDerivedLifecycleState`**; the producer
/// (`super::replay_derived_state`) wraps it through `ReplayDerivedLifecycleState::from_replay`.
/// Every operation kind has an exact effect; blob kinds and text content are resolved through the
/// real store-backed [`StoreBackedResolver`].
pub(crate) fn replay_lineage(
    reader: &impl ObjectReader,
    baseline: ObjectId,
    horizon: ObjectId,
) -> Result<NodeLifecycleState, LifecycleReplayError> {
    let chain = walk_lineage(reader, baseline, horizon)?;
    replay_chain_with_appended_patches(reader, &chain, &[], false)
}

pub(crate) fn replay_with_appended_patches(
    reader: &impl ObjectReader,
    parent: Option<ObjectId>,
    patch_ids: &[ObjectId],
) -> Result<NodeLifecycleState, LifecycleReplayError> {
    let chain = match parent {
        Some(parent) => walk_lineage_to_genesis(reader, parent)?,
        None => Vec::new(),
    };
    replay_chain_with_appended_patches(reader, &chain, patch_ids, true)
}

fn replay_chain_with_appended_patches(
    reader: &impl ObjectReader,
    chain: &[WalkedBlock],
    appended_patch_ids: &[ObjectId],
    require_schema_one: bool,
) -> Result<NodeLifecycleState, LifecycleReplayError> {
    let blob_resolver = if require_schema_one {
        StoreBackedResolver::new_format2(reader)
    } else {
        StoreBackedResolver::new(reader)
    };
    let mut state = NodeLifecycleState::new();
    let mut text_cache = TextCache::new();
    for (_block_id, block) in chain {
        apply_patch_ids(
            reader,
            &block.patch_ids,
            &blob_resolver,
            &mut state,
            &mut text_cache,
            require_schema_one,
        )?;
    }
    apply_patch_ids(
        reader,
        appended_patch_ids,
        &blob_resolver,
        &mut state,
        &mut text_cache,
        require_schema_one,
    )?;
    Ok(state)
}

fn apply_patch_ids<R: BlobKindResolver + BlobContentResolver>(
    reader: &impl ObjectReader,
    patch_ids: &[ObjectId],
    blob_resolver: &R,
    state: &mut NodeLifecycleState,
    text_cache: &mut TextCache,
    require_schema_one: bool,
) -> Result<(), LifecycleReplayError> {
    for patch_id in patch_ids {
        let operations = read_patch_operations(reader, *patch_id, require_schema_one)?;
        for operation in &operations {
            apply_state_effect(state, text_cache, &operation.kind, blob_resolver)?;
        }
    }
    Ok(())
}

/// Read and decode a patch's operations, mapping every failure to `MalformedPatchInLineage`.
fn read_patch_operations(
    reader: &impl ObjectReader,
    patch_id: ObjectId,
    require_schema_one: bool,
) -> Result<Vec<crate::patch_replay::decode::DecodedPatchOperation>, LifecycleReplayError> {
    let envelope = reader.read_object(patch_id).map_err(|e| {
        LifecycleReplayError::MalformedPatchInLineage {
            patch_id,
            detail: e.to_string(),
        }
    })?;
    let Some(envelope) = envelope else {
        return Err(LifecycleReplayError::MalformedPatchInLineage {
            patch_id,
            detail: "patch object is missing".to_string(),
        });
    };
    if envelope.object_type != ObjectType::Patch {
        return Err(LifecycleReplayError::MalformedPatchInLineage {
            patch_id,
            detail: format!("object is not a Patch ({} found)", envelope.object_type),
        });
    }
    if require_schema_one && envelope.schema_version != 1 {
        return Err(LifecycleReplayError::MalformedPatchInLineage {
            patch_id,
            detail: format!(
                "format-2 Patch requires envelope schema 1, got {}",
                envelope.schema_version
            ),
        });
    }
    decode_patch_operations(&envelope.canonical_payload).map_err(|e| {
        LifecycleReplayError::MalformedPatchInLineage {
            patch_id,
            detail: e.to_string(),
        }
    })
}

mod effect;
use effect::apply_state_effect;

#[cfg(test)]
mod tests;
