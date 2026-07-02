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
//! text). No operation maps to `UnsupportedLifecycleEffect` any longer. The reconstructed state is
//! **not** yet wrapped as `ReplayDerivedLifecycleState` and is consumed by no caller — 2c-2e
//! exposes it and wires `ComparedLifecycleCache`. The structured error taxonomy (carry-forward
//! P2-3) lives here, ahead of any caller branching on it.

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
/// These are the classes a replay / compare / fallback caller branches on. They land here, with
/// the lineage walker, ahead of any exposure of replay-derived state, exactly as required. A few
/// classes (`MissingBlobForLifecycleEffect`, `LifecycleCompareMismatch`) are not yet produced by
/// this skeleton; they belong to the state-effect (2c-2b/2c-2c) and compare (2c-2e) increments and
/// are defined now so the taxonomy is stable before any caller branches on it.
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
    /// A decoded operation has no implemented lifecycle state effect yet — replay fails closed
    /// rather than producing an approximate state (O1).
    UnsupportedLifecycleEffect { operation: &'static str },
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
    /// A cache disagreed with authoritative replay. (2c-2e.)
    LifecycleCompareMismatch { detail: String },
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
            Self::UnsupportedLifecycleEffect { operation } => write!(
                f,
                "lifecycle replay: operation {operation} has no implemented state effect; \
                 replay fails closed (O1)"
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
            Self::LifecycleCompareMismatch { detail } => {
                write!(
                    f,
                    "lifecycle replay: cache disagrees with authoritative replay ({detail})"
                )
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
                if current != horizon {
                    return Err(LifecycleReplayError::HorizonNotInLineage {
                        horizon_id: horizon,
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
    let blob_resolver = StoreBackedResolver::new(reader);
    let mut state = NodeLifecycleState::new();
    let mut text_cache = TextCache::new();
    for (_block_id, block) in &chain {
        for patch_id in &block.patch_ids {
            let operations = read_patch_operations(reader, *patch_id)?;
            for operation in &operations {
                apply_state_effect(&mut state, &mut text_cache, &operation.kind, &blob_resolver)?;
            }
        }
    }
    Ok(state)
}

/// Read and decode a patch's operations, mapping every failure to `MalformedPatchInLineage`.
fn read_patch_operations(
    reader: &impl ObjectReader,
    patch_id: ObjectId,
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
    decode_patch_operations(&envelope.canonical_payload).map_err(|e| {
        LifecycleReplayError::MalformedPatchInLineage {
            patch_id,
            detail: e.to_string(),
        }
    })
}

/// Apply one decoded operation's lifecycle state effect. `CreateFile`, `CreateSymlink`,
/// `DeleteNode`, `RenamePath`, `ChangePerm`, `ReplaceBinary`, and `EditText` are all exact as of
/// 2c-2d. All node-lifecycle apply failures map to `InconsistentLifecycleEffect`. `EditText`
/// additionally uses the materialized-text cache and the blob-content resolver.
fn apply_state_effect<R: BlobKindResolver + BlobContentResolver>(
    state: &mut NodeLifecycleState,
    text_cache: &mut TextCache,
    kind: &DecodedOperationKind,
    blob_resolver: &R,
) -> Result<(), LifecycleReplayError> {
    match kind {
        DecodedOperationKind::CreateFile {
            path,
            node_id,
            blob_id,
            mode,
        } => {
            let repo_path = parse_repo_path(path)?;
            let blob_kind = blob_resolver
                .blob_kind(blob_id)
                .map_err(inconsistent)?
                .ok_or(LifecycleReplayError::MissingBlobForLifecycleEffect { blob_id: *blob_id })?;
            let node_kind = NodeKind::from_file_blob_kind(blob_kind).map_err(inconsistent)?;
            let node = LiveNode {
                path: repo_path,
                kind: node_kind,
                content: NodeContent::File {
                    blob_id: *blob_id,
                    mode: *mode,
                },
            };
            state.create_node(*node_id, node).map_err(inconsistent)
        }
        DecodedOperationKind::CreateSymlink {
            path,
            node_id,
            target,
        } => {
            let repo_path = parse_repo_path(path)?;
            let node = LiveNode {
                path: repo_path,
                kind: NodeKind::Symlink,
                content: NodeContent::Symlink {
                    target: target.clone(),
                },
            };
            state.create_node(*node_id, node).map_err(inconsistent)
        }
        DecodedOperationKind::DeleteNode {
            path,
            node_id,
            preimage,
        } => {
            // Exact replay (P1-1): the persisted record's old-state assertion (path, kind,
            // content) must match the replayed live node before the tombstone is recorded.
            let expected = expected_deleted_node(path, preimage)?;
            state
                .delete_node_checked(*node_id, &expected)
                .map(|_| ())
                .map_err(inconsistent)?;
            // The node's materialized text (if any) is no longer current.
            text_cache.remove(node_id);
            Ok(())
        }
        DecodedOperationKind::RenamePath {
            node_id,
            old_path,
            new_path,
        } => {
            // Exact replay (P1-2): the persisted record's old_path must match the live path.
            let expected_old = parse_repo_path(old_path)?;
            let new = parse_repo_path(new_path)?;
            state
                .rename_node_checked(*node_id, &expected_old, new)
                .map_err(inconsistent)
        }
        DecodedOperationKind::ChangePerm {
            node_id,
            old_mode,
            new_mode,
        } => state
            .change_file_mode(*node_id, *old_mode, *new_mode)
            .map_err(inconsistent),
        DecodedOperationKind::ReplaceBinary {
            node_id,
            old_blob_id,
            new_blob_id,
        } => {
            // Both blobs must be persisted and binary; the live node must currently reference
            // old_blob_id (checked in the substrate). Exact content swap, mode preserved.
            require_binary_blob(blob_resolver, *old_blob_id)?;
            require_binary_blob(blob_resolver, *new_blob_id)?;
            state
                .replace_file_blob(*node_id, *old_blob_id, *new_blob_id)
                .map_err(inconsistent)
        }
        DecodedOperationKind::EditText {
            node_id,
            span_id,
            old_span_hash,
            left_anchor_hash,
            right_anchor_hash,
            replacement_text,
            old_span_text,
        } => apply_edit_text(
            state,
            text_cache,
            blob_resolver,
            *node_id,
            span_id,
            old_span_hash,
            left_anchor_hash,
            right_anchor_hash,
            replacement_text,
            old_span_text,
        ),
    }
}

/// Apply an `EditText` to the lifecycle index (2c-2d, forward only). Materializes the node's
/// current text (lazily, from the blob-content resolver), localizes the span per the FDD-01 §5.1
/// 64-byte anchor-filtered rule, splices in `replacement_text`, derives the new
/// `BlobPayload(Text, new_text)` object id, and records it as the node's content id. Mode,
/// `node_id`, and path are unchanged.
#[allow(clippy::too_many_arguments)]
fn apply_edit_text<R: BlobContentResolver>(
    state: &mut NodeLifecycleState,
    text_cache: &mut TextCache,
    blob_resolver: &R,
    node_id: NodeId,
    span_id: &[u8; 32],
    old_span_hash: &[u8; 32],
    left_anchor_hash: &[u8; 32],
    right_anchor_hash: &[u8; 32],
    replacement_text: &[u8],
    old_span_text: &[u8],
) -> Result<(), LifecycleReplayError> {
    // Defense-in-depth: the canonical validator binds this at decode; re-assert here.
    if text_span_hash(old_span_text) != *old_span_hash {
        return Err(LifecycleReplayError::InconsistentLifecycleEffect {
            detail: "EditText old_span_hash != SHA-256(old_span_text)".to_string(),
        });
    }

    // Liveness + text-file eligibility; capture the current content blob id.
    let live = state.live_node(&node_id).ok_or_else(|| {
        LifecycleReplayError::InconsistentLifecycleEffect {
            detail: "EditText target node_id is not live".to_string(),
        }
    })?;
    if live.kind != NodeKind::TextFile {
        return Err(LifecycleReplayError::InconsistentLifecycleEffect {
            detail: "EditText target is not a text-file node".to_string(),
        });
    }
    let current_blob_id = match &live.content {
        NodeContent::File { blob_id, .. } => *blob_id,
        NodeContent::Symlink { .. } => {
            return Err(LifecycleReplayError::InconsistentLifecycleEffect {
                detail: "EditText target has symlink content".to_string(),
            });
        }
    };

    // Materialize current text: cache hit, else read the current content blob (must be Text).
    let current_text = match text_cache.get(&node_id) {
        Some(text) => text.clone(),
        None => {
            let (blob_kind, content) = blob_resolver
                .blob_content(&current_blob_id)
                .map_err(inconsistent)?
                .ok_or(LifecycleReplayError::MissingBlobForLifecycleEffect {
                    blob_id: current_blob_id,
                })?;
            if blob_kind != BlobKind::Text {
                return Err(LifecycleReplayError::InconsistentLifecycleEffect {
                    detail: "EditText current content blob is not Text".to_string(),
                });
            }
            content
        }
    };

    // Localize the span (FDD-01 §5.1, anchor-filtered) via the shared text-span module.
    let (start, end) = text_span::locate_text_span(
        &current_text,
        old_span_text,
        left_anchor_hash,
        right_anchor_hash,
        span_id,
        node_id,
        old_span_hash,
    )
    .map_err(|reason| LifecycleReplayError::TextSpanResolutionFailed {
        node_id,
        span_id: *span_id,
        reason,
    })?;

    // Splice and derive the new content identity through the shared module, so authoring and
    // replay produce the same bytes and the same `BlobPayload(Text, new_text)` id.
    let new_text = text_span::splice_text(&current_text, start, end, replacement_text)
        .map_err(inconsistent)?;
    let new_blob_id = text_span::text_blob_id(&new_text).map_err(inconsistent)?;
    state
        .set_text_blob(node_id, new_blob_id)
        .map_err(inconsistent)?;
    text_cache.insert(node_id, new_text);
    Ok(())
}

/// Require a blob to be present and `BlobKind::Binary` for a `ReplaceBinary` effect; a missing
/// blob is the fail-closed `MissingBlobForLifecycleEffect`, a non-binary blob is inconsistent.
fn require_binary_blob(
    resolver: &impl BlobKindResolver,
    blob_id: ObjectId,
) -> Result<(), LifecycleReplayError> {
    let kind = resolver
        .blob_kind(&blob_id)
        .map_err(inconsistent)?
        .ok_or(LifecycleReplayError::MissingBlobForLifecycleEffect { blob_id })?;
    if kind != BlobKind::Binary {
        return Err(LifecycleReplayError::InconsistentLifecycleEffect {
            detail: format!("ReplaceBinary blob {blob_id} is not binary ({kind:?})"),
        });
    }
    Ok(())
}

/// Build the live node a `DeleteNode` record asserts it is deleting, from the persisted path and
/// discriminated deletion preimage, for exact-replay verification (P1-1).
fn expected_deleted_node(
    path: &str,
    preimage: &DecodedDeletePreimage,
) -> Result<LiveNode, LifecycleReplayError> {
    let repo_path = parse_repo_path(path)?;
    let (kind, content) = match preimage {
        DecodedDeletePreimage::File {
            old_node_kind,
            old_blob_id,
            old_mode,
        } => (
            *old_node_kind,
            NodeContent::File {
                blob_id: *old_blob_id,
                mode: *old_mode,
            },
        ),
        DecodedDeletePreimage::Symlink { old_target } => (
            NodeKind::Symlink,
            NodeContent::Symlink {
                target: old_target.clone(),
            },
        ),
    };
    Ok(LiveNode {
        path: repo_path,
        kind,
        content,
    })
}

/// Parse a decoded operation's raw path into a validated repo-relative path.
fn parse_repo_path(path: &str) -> Result<RepoPath, LifecycleReplayError> {
    RepoPath::parse(path).map_err(inconsistent)
}

/// Map a node-lifecycle apply error into the structured `InconsistentLifecycleEffect` class.
fn inconsistent<E: fmt::Display>(error: E) -> LifecycleReplayError {
    LifecycleReplayError::InconsistentLifecycleEffect {
        detail: error.to_string(),
    }
}

#[cfg(test)]
mod tests;
