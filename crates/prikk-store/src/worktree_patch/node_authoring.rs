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

use std::collections::BTreeMap;
use std::fmt;
use std::fs;

use prikk_error::{PrikkError, Result};
use prikk_object::{
    BlobKind, BlobPayload, CanonicalEncode, ChangePerm, CreateFile, DeleteNode, DeleteNodePreimage,
    EditText, NodeId, NodeKind, ObjectEnvelope, ObjectId, ObjectType, Operation, OperationKind,
    PatchPayload, PatchPurpose, ReplaceBinary,
};

use crate::active::{prepare_empty_active_ref_for_append, require_active_ref_for_non_empty_wal};
use crate::author_signing::AuthorSigner;
use crate::layout::RepositoryLayout;
use crate::lifecycle_cache::replay_derived_state;
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

/// Canonical mode recorded for a created regular file with no executable bit, and the default on
/// platforms without an executable-bit source (4.4a-2aR, ratified rule).
const REGULAR_FILE_MODE: u32 = 0o100_644;
/// Canonical mode for a created regular file with an executable bit (4.4a-2aR, ratified rule).
const EXECUTABLE_FILE_MODE: u32 = 0o100_755;

/// Normalize a regular file's OS mode to a canonical file mode (4.4a-2aR, ratified rule):
/// any executable bit set (`mode & 0o111 != 0` on Unix) → `0o100755`, otherwise `0o100644`.
/// Read/write bits, setuid/setgid/sticky, and platform attributes are ignored. On platforms without
/// an executable-bit source (non-Unix), regular files default to `0o100644`.
#[cfg(unix)]
fn normalize_file_mode(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    if metadata.permissions().mode() & 0o111 != 0 {
        EXECUTABLE_FILE_MODE
    } else {
        REGULAR_FILE_MODE
    }
}

#[cfg(not(unix))]
fn normalize_file_mode(_metadata: &fs::Metadata) -> u32 {
    REGULAR_FILE_MODE
}

/// A regular worktree file: its bytes plus its canonical (normalized) mode.
struct WorktreeFile {
    bytes: Vec<u8>,
    mode: u32,
}

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
    let _active_lock =
        ActiveLock::acquire(layout.default_active_lock_path()).map_err(AuthorError::Store)?;

    let wal = Wal::new(layout.default_queue_wal_path());
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
    // baseline (4.4b) when the target ref has never been published.
    let baseline = resolve_worktree_baseline(layout, ref_name)?;
    let baseline_state: NodeLifecycleState = match &baseline {
        WorktreeBaseline::Published {
            baseline_block,
            horizon,
        } => replay_derived_state(&object_store, *baseline_block, *horizon)?
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

    // Worktree view: path -> bytes for regular files (symlinks/non-regular fail closed).
    let worktree = enumerate_worktree_files(layout)?;

    // Working state for same-session duplicate prevention (E1): clone the baseline; each fresh node
    // is inserted immediately after minting so the next mint sees it via contains_seen_node_id.
    let mut working_state = baseline_state.clone();

    let mut planned: Vec<PlannedOp> = Vec::new();

    // Fresh creates are collected first, then minted in canonical path order (E1) — so path->node_id
    // assignment is independent of worktree traversal order. Each carries its normalized mode.
    let mut create_candidates: Vec<(String, Vec<u8>, u32)> = Vec::new();

    for (path, file) in &worktree {
        let bytes = &file.bytes;
        if let Some(base) = baseline_files.get(path) {
            // Existing node: kind is authoritative (E4); compare in that kind, never reclassify.
            match base.kind {
                NodeKind::TextFile => {
                    if std::str::from_utf8(bytes).is_err() {
                        return Err(AuthorError::UnsupportedKindTransition(format!(
                            "{path}: existing TextFile cannot accept non-UTF-8 content"
                        )));
                    }
                    let new_blob = text_span::text_blob_id(bytes)?;
                    if new_blob != base.blob_id {
                        planned.push(plan_edit_text(&object_store, base, bytes, path)?);
                    }
                }
                NodeKind::BinaryFile => {
                    let new_blob = binary_blob_id(bytes)?;
                    if new_blob != base.blob_id {
                        planned.push(plan_replace_binary(&object_store, base, bytes, path)?);
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
            if file.mode != base.mode {
                planned.push(plan_change_perm(base, file.mode, path));
            }
        } else if baseline_symlinks.contains_key(path) {
            return Err(AuthorError::UnsupportedSymlinkAuthoring(format!(
                "{path}: symlink node modification is out of scope"
            )));
        } else {
            create_candidates.push((path.clone(), bytes.clone(), file.mode));
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

fn binary_blob_id(bytes: &[u8]) -> std::result::Result<ObjectId, AuthorError> {
    let payload = BlobPayload::new(BlobKind::Binary, bytes.to_vec());
    let canonical = payload.to_canonical_bytes().map_err(AuthorError::Store)?;
    Ok(ObjectId::from_canonical_payload(
        ObjectType::Blob,
        1,
        &canonical,
    ))
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

/// Enumerate regular worktree files as `path -> (bytes, normalized mode)`, rejecting symlinks /
/// non-regular files and validating each path through `prikk-path`. The `.prikk` repository
/// directory is skipped.
fn enumerate_worktree_files(
    layout: &RepositoryLayout,
) -> std::result::Result<BTreeMap<String, WorktreeFile>, AuthorError> {
    let mut out = BTreeMap::new();
    let root = layout.root().to_path_buf();
    walk_dir(&root, &root, &mut out)?;
    Ok(out)
}

fn walk_dir(
    root: &std::path::Path,
    dir: &std::path::Path,
    out: &mut BTreeMap<String, WorktreeFile>,
) -> std::result::Result<(), AuthorError> {
    let entries =
        fs::read_dir(dir).map_err(|e| AuthorError::Store(PrikkError::Io(e.to_string())))?;
    for entry in entries {
        let entry = entry.map_err(|e| AuthorError::Store(PrikkError::Io(e.to_string())))?;
        let file_name = entry.file_name();
        if file_name == ".prikk" {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|e| AuthorError::Store(PrikkError::Io(e.to_string())))?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            // Determine the repo-relative path for the error message.
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            return Err(AuthorError::UnsupportedSymlinkAuthoring(format!(
                "{rel}: worktree symlink authoring is out of scope"
            )));
        }
        if file_type.is_dir() {
            walk_dir(root, &path, out)?;
            continue;
        }
        if !file_type.is_file() {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();
            return Err(AuthorError::Store(PrikkError::InvalidName(format!(
                "{rel}: worktree entry is not a regular file"
            ))));
        }
        let relative = path.strip_prefix(root).map_err(|_| {
            AuthorError::Store(PrikkError::Integrity(
                "worktree path escaped repository root".to_string(),
            ))
        })?;
        // Strict conversion (N2): a non-UTF-8 OS path fails closed here rather than being silently
        // lossily replaced before `RepoPath::parse`. Identity-bearing paths never derive from lossy
        // bytes.
        let rel = relative.to_str().ok_or_else(|| {
            AuthorError::Store(PrikkError::InvalidName(format!(
                "worktree path is not valid UTF-8: {}",
                relative.to_string_lossy()
            )))
        })?;
        // Validate through prikk-path (rejects traversal, reserved names, etc.).
        let repo_path = RepoPath::parse(rel).map_err(AuthorError::Store)?;
        let mode = normalize_file_mode(&metadata);
        let bytes =
            fs::read(&path).map_err(|e| AuthorError::Store(PrikkError::Io(e.to_string())))?;
        out.insert(repo_path.as_str().to_string(), WorktreeFile { bytes, mode });
    }
    Ok(())
}
