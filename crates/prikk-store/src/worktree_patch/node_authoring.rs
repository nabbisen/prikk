//! Node-addressed worktree authoring (DC-09 Phase 4.4a-2a / 4.4a-2aR / 4.4a-2b).
//!
//! Turns worktree changes into node-addressed §9.3 operations against a baseline reconstructed from
//! authoritative replay. Existing paths resolve to their persisted `node_id` from the replay-derived
//! lifecycle state; fresh nodes are minted through [`NodeIdGenerator`] in canonical create order;
//! text edits compute all span identity through the shared [`crate::text_span`] module. Identity-bearing
//! policies: existing-node kind is authoritative (no text↔binary transition), operation order is
//! canonical, and fresh `node_id` assignment is deterministic with respect to canonical create order.
//! File modes are normalized through the single [`normalize_file_mode`] rule and drive both
//! `CreateFile.mode` (4.4a-2aR) and existing-node `ChangePerm` detection (4.4a-2b).
//!
//! Out of scope (unchanged): rename inference (moves author as delete+create) and symlink authoring
//! (fails closed until FDD-04 §5.4a).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

mod worktree_files;

use prikk_error::{PrikkError, Result};
use prikk_object::{
    BlobKind, BlobPayload, CanonicalEncode, ChangePerm, CreateFile, DeleteNode, DeleteNodePreimage,
    EditText, NodeId, NodeKind, ObjectEnvelope, ObjectId, ObjectType, Operation, OperationKind,
    PatchPayload, PatchPurpose, ReplaceBinary,
};

use crate::active::{prepare_empty_active_ref_for_append, require_active_ref_for_non_empty_wal};
use crate::author_signing::AuthorSigner;
use crate::commit_index::{self, CommitIndex, CommitIndexEntry};
use crate::fsutil::{RootFileStat, read_file_if_exists};
use crate::layout::RepositoryLayout;
use crate::lifecycle_cache::incremental::resolve_baseline_state;
use crate::lock::ActiveLock;
use crate::node_id_gen::{NodeIdEntropySource, NodeIdGenerator};
use crate::node_lifecycle::{LiveNode, NodeContent, NodeLifecycleState};
use crate::object_store::{FileObjectStore, ObjectReader, ObjectWriter};
use crate::patch_replay::{WorktreeBaseline, resolve_worktree_baseline};
use crate::path::RepoPath;
use crate::text_span;
use crate::wal::Wal;
use crate::worktree_patch::{
    WorktreePatchCommitOptions, WorktreePatchCommitReport, WorktreePatchOperationKind,
    WorktreePatchOperationSummary, next_op_seq,
};
use crate::{
    ActiveRefMetadata, read_active_ref_metadata, remove_active_ref_metadata,
    validate_local_branch_ref,
};

use worktree_files::{WorktreeFileMeta, enumerate_worktree_files};

/// Canonical mode recorded for a created regular file with no executable bit, and the default on
/// platforms without an executable-bit source (4.4a-2aR, ratified rule).
const REGULAR_FILE_MODE: u32 = 0o100_644;
/// Canonical mode for a created regular file with an executable bit (4.4a-2aR, ratified rule).
const EXECUTABLE_FILE_MODE: u32 = 0o100_755;

/// Structured authoring failure. Kept structured internally (review E2/E3/E4) and flattened into
/// [`PrikkError`] only at the public command boundary.
#[derive(Debug)]
pub(crate) enum AuthorError {
    /// A changed existing path does not resolve to a live node id in the replay-derived baseline
    /// (e.g. a snapshot-only baseline, which carries no node identity). Fails closed; never minted.
    NodeIdentityUnavailable(String),
    /// New worktree bytes would change an existing node's kind (text↔binary). Out of scope.
    UnsupportedKindTransition(String),
    /// Symlink authoring is out of scope until FDD-04 §5.4a static target validation.
    UnsupportedSymlinkAuthoring(String),
    /// Fresh node-id minting failed (propagated without flattening).
    Mint(crate::node_id_gen::NodeIdMintError),
    /// An underlying store/encoding error.
    Store(PrikkError),
}

impl fmt::Display for AuthorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeIdentityUnavailable(detail) => {
                write!(f, "worktree authoring: node identity unavailable: {detail}")
            }
            Self::UnsupportedKindTransition(detail) => {
                write!(
                    f,
                    "worktree authoring: unsupported kind transition: {detail}"
                )
            }
            Self::UnsupportedSymlinkAuthoring(detail) => {
                write!(
                    f,
                    "worktree authoring: unsupported symlink authoring: {detail}"
                )
            }
            Self::Mint(e) => write!(f, "worktree authoring: {e}"),
            Self::Store(e) => write!(f, "worktree authoring: {e}"),
        }
    }
}

impl From<AuthorError> for PrikkError {
    fn from(e: AuthorError) -> Self {
        match e {
            AuthorError::Store(inner) => inner,
            AuthorError::Mint(inner) => inner.into(),
            other => PrikkError::Integrity(other.to_string()),
        }
    }
}

impl From<PrikkError> for AuthorError {
    fn from(e: PrikkError) -> Self {
        AuthorError::Store(e)
    }
}

/// A baseline live file node, resolved from the replay-derived state.
struct BaselineFile {
    node_id: NodeId,
    kind: NodeKind,
    blob_id: ObjectId,
    mode: u32,
}

/// A planned, fully-resolved operation prior to canonical ordering / `op_seq` assignment.
struct PlannedOp {
    kind: OperationKind,
    /// Sort key: canonical repo path for the op's primary node.
    path: String,
    /// Sort key tiebreak: the node id the op addresses.
    node_id: NodeId,
    /// Report label.
    summary_kind: WorktreePatchOperationKind,
    /// Blob references this op writes or reuses (for the report's `referenced_blob_count`).
    blob_refs: usize,
}

/// Rank operations by kind for deterministic ordering (review v2 §4):
/// `DeleteNode` < `CreateFile` < `ChangePerm` < `ReplaceBinary` < `EditText`.
fn kind_rank(kind: &OperationKind) -> u8 {
    match kind {
        OperationKind::DeleteNode(_) => 0,
        OperationKind::CreateFile(_) => 1,
        OperationKind::ChangePerm(_) => 2,
        OperationKind::ReplaceBinary(_) => 3,
        OperationKind::EditText(_) => 4,
        OperationKind::CreateSymlink(_) | OperationKind::RenamePath(_) => 5,
    }
}

/// Author a node-addressed patch from worktree changes and append it to the active WAL.
///
/// The `generator` is injected (review E2): production passes `NodeIdGenerator::production()`; tests
/// pass a deterministic generator. State comes only from `replay_derived_state` (review E3); the
/// snapshot manifest is never consulted as identity authority.
pub(crate) fn author_worktree_patch<S: NodeIdEntropySource, A: AuthorSigner>(
    layout: &RepositoryLayout,
    ref_name: &str,
    message: &str,
    _options: WorktreePatchCommitOptions,
    generator: &mut NodeIdGenerator<S>,
    signer: &A,
) -> Result<WorktreePatchCommitReport> {
    if message.trim().is_empty() {
        return Err(PrikkError::InvalidName(
            "commit message must not be empty".to_string(),
        ));
    }
    author_inner(layout, ref_name, message, generator, signer).map_err(PrikkError::from)
}

fn author_inner<S: NodeIdEntropySource, A: AuthorSigner>(
    layout: &RepositoryLayout,
    ref_name: &str,
    _message: &str,
    generator: &mut NodeIdGenerator<S>,
    signer: &A,
) -> std::result::Result<WorktreePatchCommitReport, AuthorError> {
    let canonical_ref = validate_local_branch_ref(ref_name).map_err(AuthorError::Store)?;
    // 4.4bR2: hold the active-session lock across the entire critical section — the active-WAL
    // emptiness/genesis guard, patch authoring, and the final WAL append — so guard and append are
    // one atomic step. `ActiveLock::acquire` is fail-fast (exclusive create), so a concurrent commit
    // either loses the lock here (LockConflict) or, if it runs after this releases, sees the appended
    // record and fails the "seal first" guard. Released on return (RAII). The append below uses the
    // raw WAL under this held lock (not `ActiveSession::append_patch`, which would re-acquire).
    let _active_lock = ActiveLock::acquire(layout).map_err(AuthorError::Store)?;
    crate::refs::ensure_no_incomplete_publication(layout).map_err(AuthorError::Store)?;

    let wal = Wal::for_layout(layout);
    let active_replay = wal.replay().map_err(AuthorError::Store)?;
    if active_replay.trailing_partial_bytes != 0 {
        return Err(AuthorError::Store(PrikkError::InvalidName(format!(
            "active WAL has {} trailing partial bytes; run `prikk doctor --repair-wal-tail` \
             before committing",
            active_replay.trailing_partial_bytes
        ))));
    }
    if active_replay.records.is_empty() {
        match read_active_ref_metadata(layout).map_err(AuthorError::Store)? {
            ActiveRefMetadata::Missing => {}
            ActiveRefMetadata::Valid(_) | ActiveRefMetadata::Invalid(_) => {
                remove_active_ref_metadata(layout).map_err(AuthorError::Store)?;
            }
        }
    } else {
        require_active_ref_for_non_empty_wal(layout, &canonical_ref).map_err(AuthorError::Store)?;
        return Err(AuthorError::Store(PrikkError::LockConflict(format!(
            "active WAL already contains patches for {canonical_ref}; run `prikk seal --ref \
             {canonical_ref}` before committing again"
        ))));
    }

    let object_store = FileObjectStore::new(layout.clone());

    // Baseline node lifecycle state from authoritative replay only (E3), or an empty genesis
    // baseline (4.4b) when the target ref has never been published. DC-64: `resolve_baseline_state`
    // applies an incremental step from a cached predecessor when eligible, falling back to an
    // unmodified full replay otherwise — see
    // `rfcs/handoffs/DC-64-baseline-reconstruction-cost/incremental-baseline-cache-design-v1.md`.
    let baseline = resolve_worktree_baseline(layout, ref_name)?;
    let baseline_state: NodeLifecycleState = match &baseline {
        WorktreeBaseline::Published {
            baseline_block,
            horizon,
        } => resolve_baseline_state(layout, &object_store, *baseline_block, *horizon)?
            .state()
            .clone(),
        WorktreeBaseline::Genesis => NodeLifecycleState::new(),
    };

    // Baseline file view: path -> (node_id, kind, blob_id, mode). Symlink nodes are tracked so a
    // change touching one can fail closed.
    let mut baseline_files: BTreeMap<String, BaselineFile> = BTreeMap::new();
    let mut baseline_symlinks: BTreeMap<String, NodeId> = BTreeMap::new();
    for (node_id, node) in baseline_state.live_nodes() {
        match &node.content {
            NodeContent::File { blob_id, mode } => {
                baseline_files.insert(
                    node.path.as_str().to_string(),
                    BaselineFile {
                        node_id: *node_id,
                        kind: node.kind,
                        blob_id: *blob_id,
                        mode: *mode,
                    },
                );
            }
            NodeContent::Symlink { .. } => {
                baseline_symlinks.insert(node.path.as_str().to_string(), *node_id);
            }
        }
    }

    // E3: distinguish a snapshot-only baseline (path-keyed, no node identity) from a genuinely
    // empty node repo. An empty node state with a snapshot blob reference means the only identity
    // authority available is the path-keyed snapshot manifest, which Option A excludes — fail closed
    // rather than treat every snapshot-tracked file as untracked and mint fresh ids for it. This can
    // only arise for a published baseline; a genesis baseline has no block and no snapshot.
    if let WorktreeBaseline::Published { baseline_block, .. } = &baseline {
        if baseline_files.is_empty()
            && baseline_symlinks.is_empty()
            && baseline_block_has_snapshot_ref(&object_store, *baseline_block)?
        {
            return Err(AuthorError::NodeIdentityUnavailable(
                "baseline is snapshot-derived and carries no node identity; \
                 a node-addressed baseline is required for worktree authoring"
                    .to_string(),
            ));
        }
    }

    // Worktree view: path -> metadata for regular files (symlinks/non-regular fail closed). Content
    // is read on demand (below) so an unchanged file is never opened — DC-56.
    let worktree = enumerate_worktree_files(layout)?;

    // DC-56 changed-path index: per-path (size, mtime, mode) -> last-known content hash, so an
    // unchanged file's content read can be skipped. Rebuildable and never authoritative (NFR-PERF-04)
    // — a missing or corrupt index loads as empty and simply costs one full read per path, exactly as
    // the first commit against a repository already does. See the cache-validity specification at
    // `rfcs/handoffs/DC-56-commit-full-tree-scan-compliance/cache-validity-specification-v1.md`.
    let mut commit_index = CommitIndex::load(layout).map_err(AuthorError::Store)?;

    // Working state for same-session duplicate prevention (E1): clone the baseline; each fresh node
    // is inserted immediately after minting so the next mint sees it via contains_seen_node_id.
    let mut working_state = baseline_state.clone();

    let mut planned: Vec<PlannedOp> = Vec::new();

    // Fresh creates are collected first, then minted in canonical path order (E1) — so path->node_id
    // assignment is independent of worktree traversal order. Each carries its normalized mode.
    let mut create_candidates: Vec<(String, Vec<u8>, u32)> = Vec::new();

    for (path, meta) in &worktree {
        if let Some(base) = baseline_files.get(path) {
            // Existing node: kind is authoritative (E4); compare in that kind, never reclassify.
            match base.kind {
                NodeKind::TextFile => {
                    let resolved = resolve_existing_file(
                        layout,
                        &mut commit_index,
                        path,
                        meta,
                        BlobKind::Text,
                    )?;
                    if resolved.content_hash != base.blob_id {
                        let bytes = match resolved.bytes {
                            Some(bytes) => bytes,
                            None => read_existing_file_bytes(layout, path, BlobKind::Text)?,
                        };
                        planned.push(plan_edit_text(&object_store, base, &bytes, path)?);
                    }
                }
                NodeKind::BinaryFile => {
                    let resolved = resolve_existing_file(
                        layout,
                        &mut commit_index,
                        path,
                        meta,
                        BlobKind::Binary,
                    )?;
                    if resolved.content_hash != base.blob_id {
                        let bytes = match resolved.bytes {
                            Some(bytes) => bytes,
                            None => read_existing_file_bytes(layout, path, BlobKind::Binary)?,
                        };
                        planned.push(plan_replace_binary(&object_store, base, &bytes, path)?);
                    }
                }
                NodeKind::Symlink => {
                    return Err(AuthorError::UnsupportedSymlinkAuthoring(format!(
                        "{path}: symlink node modification is out of scope"
                    )));
                }
            }
            // Mode-change detection (4.4a-2b), independent of content, for regular file nodes only
            // (symlink nodes never reach here — they live in `baseline_symlinks`). The worktree mode
            // is normalized through the same `normalize_file_mode` rule as `CreateFile` (N2). If the
            // canonical mode differs from the replay-derived baseline mode, emit one `ChangePerm`;
            // the canonical operation sort places it before any `ReplaceBinary`/`EditText` content op.
            if meta.mode != base.mode {
                planned.push(plan_change_perm(base, meta.mode, path));
            }
        } else if baseline_symlinks.contains_key(path) {
            return Err(AuthorError::UnsupportedSymlinkAuthoring(format!(
                "{path}: symlink node modification is out of scope"
            )));
        } else {
            // A path with no baseline node is genuinely new; there is nothing a cache could have
            // recorded for it yet, so it is always read. The read is cached anyway so an unchanged
            // re-commit of the same content (once this path is itself a baseline node) can skip it.
            let bytes = read_worktree_file_bytes(layout, path)?;
            let (blob_kind, _node_kind) = classify_new(&bytes);
            let content_hash =
                commit_index::content_hash(blob_kind, &bytes).map_err(AuthorError::Store)?;
            commit_index.record(
                path.clone(),
                CommitIndexEntry {
                    size: meta.size,
                    mtime_secs: meta.mtime_secs,
                    mtime_nanos: meta.mtime_nanos,
                    mode: meta.mode,
                    kind: blob_kind,
                    content_hash,
                },
            );
            create_candidates.push((path.clone(), bytes, meta.mode));
        }
    }

    // Deletions: baseline files absent from the worktree.
    for (path, base) in &baseline_files {
        if !worktree.contains_key(path) {
            planned.push(plan_delete(base, path));
        }
    }
    for path in baseline_symlinks.keys() {
        if !worktree.contains_key(path) {
            return Err(AuthorError::UnsupportedSymlinkAuthoring(format!(
                "{path}: symlink node deletion is out of scope"
            )));
        }
    }

    // Persist the refreshed index: prune paths no longer in the worktree (so a future unrelated file
    // reusing the same path can never inherit a stale entry), then write through durably. Done
    // regardless of whether this commit finds any change to make (`planned` may still be empty below)
    // — the scan already paid for whatever reads happened, so the cache should keep the benefit.
    let live_paths: BTreeSet<String> = worktree.keys().cloned().collect();
    commit_index.retain_paths(&live_paths);
    commit_index.save(layout).map_err(AuthorError::Store)?;

    // E1: mint fresh ids in canonical path order, inserting into the working state immediately.
    create_candidates.sort_by(|a, b| a.0.cmp(&b.0));
    for (path, bytes, mode) in &create_candidates {
        let repo_path = RepoPath::parse(path).map_err(AuthorError::Store)?;
        let (blob_kind, node_kind) = classify_new(bytes);
        let blob_id = write_content_blob(&object_store, blob_kind, bytes)?;
        let node_id = generator
            .mint_fresh(&working_state)
            .map_err(AuthorError::Mint)?;
        // Insert into the working state so a subsequent mint cannot reuse this id (E1).
        working_state
            .create_node(
                node_id,
                LiveNode {
                    path: repo_path,
                    kind: node_kind,
                    content: NodeContent::File {
                        blob_id,
                        mode: *mode,
                    },
                },
            )
            .map_err(AuthorError::Store)?;
        planned.push(PlannedOp {
            kind: OperationKind::CreateFile(CreateFile {
                path: path.clone(),
                node_id,
                blob_id,
                mode: *mode,
            }),
            path: path.clone(),
            node_id,
            summary_kind: WorktreePatchOperationKind::CreateFile,
            blob_refs: 1,
        });
    }

    if planned.is_empty() {
        return Err(AuthorError::Store(PrikkError::InvalidName(
            "worktree has no node-addressed changes to commit".to_string(),
        )));
    }

    // Canonical operation ordering (review v2 §4): kind rank, then path bytes, then node_id bytes.
    planned.sort_by(|a, b| {
        kind_rank(&a.kind)
            .cmp(&kind_rank(&b.kind))
            .then_with(|| a.path.as_bytes().cmp(b.path.as_bytes()))
            .then_with(|| a.node_id.as_bytes().cmp(b.node_id.as_bytes()))
    });

    // Assemble the patch payload with contiguous op_seq from 1.
    let mut operations = Vec::with_capacity(planned.len());
    let mut referenced_blob_count = 0_usize;
    let mut text_edit_count = 0_usize;
    let mut summaries = Vec::with_capacity(planned.len());
    for (index, op) in planned.into_iter().enumerate() {
        let op_seq = next_op_seq(index).map_err(AuthorError::Store)?;
        referenced_blob_count += op.blob_refs;
        if matches!(op.kind, OperationKind::EditText(_)) {
            text_edit_count += 1;
        }
        summaries.push(WorktreePatchOperationSummary {
            path: op.path,
            operation: op.summary_kind,
        });
        operations.push(Operation {
            op_seq,
            op_id: None,
            preconditions: Vec::new(),
            kind: op.kind,
        });
    }

    let operation_count = operations.len();
    let patch_payload = PatchPayload {
        operations,
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    patch_payload.validate().map_err(AuthorError::Store)?;
    let mut patch = ObjectEnvelope::unsigned(
        ObjectType::Patch,
        1,
        patch_payload
            .to_canonical_bytes()
            .map_err(AuthorError::Store)?,
    );
    let patch_id = patch.object_id();
    // R1: real role-bound Ed25519 AUTHOR signature over the unsigned patch object id.
    let signature =
        crate::author_signing::author_signature(signer, patch_id).map_err(AuthorError::Store)?;
    patch.add_signature(signature).map_err(AuthorError::Store)?;

    prepare_empty_active_ref_for_append(layout, &canonical_ref).map_err(AuthorError::Store)?;
    let wal_sequence = wal.append_patch(&patch).map_err(AuthorError::Store)?;

    Ok(WorktreePatchCommitReport {
        ref_name: canonical_ref,
        patch_id,
        wal_sequence,
        operation_count,
        referenced_blob_count,
        text_edit_count,
        changes: summaries,
    })
}

/// True if the baseline block carries a snapshot blob reference. Used to reject a snapshot-only
/// baseline (review E3) while still allowing a genuinely empty node repo to create its first file.
fn baseline_block_has_snapshot_ref(
    object_store: &FileObjectStore,
    baseline_block: ObjectId,
) -> std::result::Result<bool, AuthorError> {
    let envelope = object_store
        .read_typed(baseline_block, ObjectType::Block)
        .map_err(AuthorError::Store)?
        .ok_or_else(|| {
            AuthorError::Store(PrikkError::Integrity(format!(
                "baseline Block {baseline_block} is missing"
            )))
        })?;
    let block = prikk_object::BlockPayload::decode_canonical(&envelope.canonical_payload)
        .map_err(AuthorError::Store)?;
    Ok(block.snapshot_blob_ref.is_some())
}

/// Classify a *new* file's blob/node kind by UTF-8 validity (existing nodes are never reclassified).
fn classify_new(bytes: &[u8]) -> (BlobKind, NodeKind) {
    if std::str::from_utf8(bytes).is_ok() {
        (BlobKind::Text, NodeKind::TextFile)
    } else {
        (BlobKind::Binary, NodeKind::BinaryFile)
    }
}

/// Plan an arbitrary-span `EditText` for a modified existing `TextFile`, with all span identity computed
/// through the shared `text_span` module (no authoring-local span logic).
fn plan_edit_text(
    object_store: &FileObjectStore,
    base: &BaselineFile,
    new_bytes: &[u8],
    path: &str,
) -> std::result::Result<PlannedOp, AuthorError> {
    let old_text = read_file_blob_bytes(object_store, base.blob_id)?;
    let span = text_span::plan_authored_text_span(&old_text, new_bytes, base.node_id)
        .map_err(|err| AuthorError::Store(PrikkError::Integrity(format!("EditText: {err}"))))?
        .ok_or_else(|| {
            AuthorError::Store(PrikkError::Integrity(
                "EditText requested for unchanged text".to_string(),
            ))
        })?;
    Ok(PlannedOp {
        kind: OperationKind::EditText(EditText {
            node_id: base.node_id,
            span_id: span.span_id,
            old_span_hash: span.old_span_hash,
            left_anchor_hash: span.left_anchor_hash,
            right_anchor_hash: span.right_anchor_hash,
            replacement_text: span.replacement_text,
            presentation_hint_line: None,
            presentation_hint_column: None,
            old_span_text: span.old_span_text,
        }),
        path: path.to_string(),
        node_id: base.node_id,
        summary_kind: WorktreePatchOperationKind::EditText,
        blob_refs: 0,
    })
}
fn plan_replace_binary(
    object_store: &FileObjectStore,
    base: &BaselineFile,
    new_bytes: &[u8],
    path: &str,
) -> std::result::Result<PlannedOp, AuthorError> {
    let new_blob_id = write_content_blob(object_store, BlobKind::Binary, new_bytes)?;
    Ok(PlannedOp {
        kind: OperationKind::ReplaceBinary(ReplaceBinary {
            node_id: base.node_id,
            old_blob_id: base.blob_id,
            new_blob_id,
        }),
        path: path.to_string(),
        node_id: base.node_id,
        summary_kind: WorktreePatchOperationKind::ReplaceBinary,
        blob_refs: 2,
    })
}

/// Plan a `ChangePerm` for an existing regular file node whose normalized worktree mode differs from
/// its replay-derived baseline mode (4.4a-2b). `old_mode` is the baseline mode; `new_mode` is the
/// normalized worktree mode.
fn plan_change_perm(base: &BaselineFile, new_mode: u32, path: &str) -> PlannedOp {
    PlannedOp {
        kind: OperationKind::ChangePerm(ChangePerm {
            node_id: base.node_id,
            old_mode: base.mode,
            new_mode,
        }),
        path: path.to_string(),
        node_id: base.node_id,
        summary_kind: WorktreePatchOperationKind::ChangePerm,
        blob_refs: 0,
    }
}

/// Plan a `DeleteNode` for a baseline file absent from the worktree.
fn plan_delete(base: &BaselineFile, path: &str) -> PlannedOp {
    PlannedOp {
        kind: OperationKind::DeleteNode(DeleteNode {
            path: path.to_string(),
            node_id: base.node_id,
            old_node_kind: base.kind,
            preimage: DeleteNodePreimage::File {
                old_blob_id: base.blob_id,
                old_mode: base.mode,
            },
        }),
        path: path.to_string(),
        node_id: base.node_id,
        summary_kind: WorktreePatchOperationKind::DeleteFile,
        blob_refs: 0,
    }
}

/// An existing node's resolved current content hash (DC-56). `bytes` is `Some` only when a real
/// read happened to produce it — a cache hit that still matches the baseline (the common,
/// unchanged case) never populates it.
struct ExistingFileResolution {
    content_hash: ObjectId,
    bytes: Option<Vec<u8>>,
}

/// Resolve an existing node's current content hash against the commit-index cache, reading the file
/// only when the cache cannot vouch for it unread. See the cache-validity specification for the
/// trust condition this implements (`CommitIndexEntry::matches_stat`).
fn resolve_existing_file(
    layout: &RepositoryLayout,
    commit_index: &mut CommitIndex,
    path: &str,
    meta: &WorktreeFileMeta,
    blob_kind: BlobKind,
) -> std::result::Result<ExistingFileResolution, AuthorError> {
    let stat = RootFileStat {
        size: meta.size,
        mtime_secs: meta.mtime_secs,
        mtime_nanos: meta.mtime_nanos,
        mode: meta.mode,
    };
    if let Some(cached) = commit_index.get(path) {
        if cached.kind == blob_kind && cached.matches_stat(&stat) {
            return Ok(ExistingFileResolution {
                content_hash: cached.content_hash,
                bytes: None,
            });
        }
    }
    let bytes = read_existing_file_bytes(layout, path, blob_kind)?;
    let content_hash = commit_index::content_hash(blob_kind, &bytes).map_err(AuthorError::Store)?;
    commit_index.record(
        path.to_string(),
        CommitIndexEntry {
            size: meta.size,
            mtime_secs: meta.mtime_secs,
            mtime_nanos: meta.mtime_nanos,
            mode: meta.mode,
            kind: blob_kind,
            content_hash,
        },
    );
    Ok(ExistingFileResolution {
        content_hash,
        bytes: Some(bytes),
    })
}

/// Read an existing node's current worktree bytes, enforcing the same rule the content comparison
/// always assumed: an existing `TextFile` node must remain valid UTF-8 (E4, existing-node kind is
/// authoritative, never reclassified).
fn read_existing_file_bytes(
    layout: &RepositoryLayout,
    path: &str,
    blob_kind: BlobKind,
) -> std::result::Result<Vec<u8>, AuthorError> {
    let bytes = read_worktree_file_bytes(layout, path)?;
    if matches!(blob_kind, BlobKind::Text) && std::str::from_utf8(&bytes).is_err() {
        return Err(AuthorError::UnsupportedKindTransition(format!(
            "{path}: existing TextFile cannot accept non-UTF-8 content"
        )));
    }
    Ok(bytes)
}

/// Read a worktree regular file's current bytes.
fn read_worktree_file_bytes(
    layout: &RepositoryLayout,
    path: &str,
) -> std::result::Result<Vec<u8>, AuthorError> {
    read_file_if_exists(layout.worktree_mutation_root(), Path::new(path))
        .map_err(AuthorError::Store)?
        .ok_or_else(|| {
            AuthorError::Store(PrikkError::Io(format!(
                "worktree entry disappeared: {path}"
            )))
        })
}

fn write_content_blob(
    object_store: &FileObjectStore,
    kind: BlobKind,
    bytes: &[u8],
) -> std::result::Result<ObjectId, AuthorError> {
    let payload = BlobPayload::new(kind, bytes.to_vec());
    let canonical = payload.to_canonical_bytes().map_err(AuthorError::Store)?;
    let envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, canonical);
    let mut store = object_store.clone();
    store.write_object(&envelope).map_err(AuthorError::Store)
}

fn read_file_blob_bytes(
    object_store: &FileObjectStore,
    blob_id: ObjectId,
) -> std::result::Result<Vec<u8>, AuthorError> {
    let envelope = object_store
        .read_object(blob_id)
        .map_err(AuthorError::Store)?
        .ok_or_else(|| {
            AuthorError::Store(PrikkError::Integrity(format!(
                "baseline content Blob {blob_id} is missing"
            )))
        })?;
    crate::blob_access::decode_file_content_blob(&envelope.canonical_payload)
        .map_err(AuthorError::Store)
}
