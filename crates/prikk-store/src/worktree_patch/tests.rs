//! Node-addressed worktree authoring tests (DC-09 Phase 4.4a-2a).
//!
//! Baselines are node-addressed `CreateFile` lineages (review Option A / E3); the snapshot manifest
//! is never used as identity authority. A deterministic node-id generator is injected so fresh-id
//! assignment and patch identity are reproducible (review E1).

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use prikk_object::{
    BlobKind, BlobPayload, BlockKind, BlockPayload, CanonicalEncode, CreateFile, MerkleRoot,
    NodeId, ObjectEnvelope, ObjectId, ObjectType, Operation, OperationKind, PatchPayload,
    PatchPurpose, RefStatePayload, SignatureAlgorithm, SignerRole,
};

use crate::node_id_gen::{NodeIdGenerator, SequenceEntropySource};
use crate::test_support::{
    dummy_signature, maintainer_signature, signed_block, signed_ref_state_envelope,
    signed_ref_update_envelope, unique_temp_dir,
};
use crate::worktree_patch::commit_worktree_changes_with_generator;
use crate::{
    ActiveLock, ActiveRefMetadata, Ed25519AuthorSigner, FileObjectStore, ObjectWriter,
    RefPublication, RefStore, RepoPath, RepositoryLayout, Wal, WorktreePatchCommitOptions,
    WorktreePatchOperationKind, finish_active_publication_cleanup, read_active_ref_metadata,
};

/// Deterministic Ed25519 AUTHOR signer for reproducible authoring (real signing, fixed seed).
fn test_signer() -> Ed25519AuthorSigner {
    Ed25519AuthorSigner::from_seed("test-author-key", &[7_u8; 32]).unwrap()
}

/// Distinct nonzero scripted entropy candidates, disjoint from the baseline node ids ([1;32], …).
fn deterministic_generator() -> NodeIdGenerator<SequenceEntropySource> {
    let candidates: Vec<[u8; 32]> = (0..32u8)
        .map(|i| {
            let mut bytes = [0x90_u8; 32];
            bytes[31] = i.wrapping_add(1);
            bytes
        })
        .collect();
    NodeIdGenerator::with_source(SequenceEntropySource::new(&candidates))
}

/// Publish a node-addressed `CreateFile` baseline lineage on `heads/main` and write each file's
/// baseline bytes into the worktree (clean-checkout simulation). Returns the baseline block id.
fn publish_node_baseline(layout: &RepositoryLayout, files: &[(&str, &[u8], BlobKind)]) -> ObjectId {
    let mut object_store = FileObjectStore::new(layout.clone());

    // Canonical create order: sort by path so op_seq and node ids are stable.
    let mut sorted: Vec<&(&str, &[u8], BlobKind)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));

    let mut operations = Vec::new();
    for (index, (path, bytes, kind)) in sorted.iter().enumerate() {
        let op_seq = u32::try_from(index + 1).unwrap();
        let blob = BlobPayload::new(*kind, bytes.to_vec());
        let blob_bytes = blob.to_canonical_bytes().unwrap();
        let blob_id = ObjectId::from_canonical_payload(ObjectType::Blob, 1, &blob_bytes);
        let blob_env = ObjectEnvelope::unsigned(ObjectType::Blob, 1, blob_bytes);
        object_store.write_object(&blob_env).unwrap();

        let node_id = NodeId::from_bytes([op_seq as u8; 32]);
        operations.push(Operation {
            op_seq,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::CreateFile(CreateFile {
                path: (*path).to_string(),
                node_id,
                blob_id,
                mode: 0o100_644,
            }),
        });

        std::fs::write(layout.root().join(path), bytes).unwrap();
    }

    let patch = PatchPayload {
        operations,
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let mut patch_env =
        ObjectEnvelope::unsigned(ObjectType::Patch, 1, patch.to_canonical_bytes().unwrap());
    patch_env.add_signature(dummy_signature()).unwrap();
    let patch_id = patch_env.object_id();
    object_store.write_object(&patch_env).unwrap();

    let block = signed_block(BlockKind::Root, Vec::new(), vec![patch_id], None);
    let block_id = block.object_id();
    object_store.write_object(&block).unwrap();

    let ref_store = RefStore::new(layout.clone());
    let ref_state = signed_ref_state_envelope("heads/main", None, block_id, 1);
    let ref_state_id = ref_state.object_id();
    let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, block_id, 1);
    ref_store
        .publish(&RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        })
        .unwrap();
    block_id
}

/// Seal the active WAL's single patch record into a new block and publish the ref forward — the
/// store-level equivalent of `prikk seal --allow-no-audit`, for tests (DC-65) that need several real
/// sealed generations in sequence without driving the CLI binary. Returns the new block id.
fn seal_active_patch(layout: &RepositoryLayout, ref_name: &str) -> ObjectId {
    let wal = Wal::for_layout(layout);
    let replay = wal.replay().unwrap();
    assert_eq!(replay.records.len(), 1, "expected exactly one active patch");
    let patch_envelope = replay.records[0].envelope.clone();
    let patch_id = patch_envelope.object_id();

    let mut object_store = FileObjectStore::new(layout.clone());
    object_store.write_object(&patch_envelope).unwrap();

    let ref_store = RefStore::new(layout.clone());
    let current_ref_state_id = ref_store
        .read_current_ref_state_id(ref_name)
        .unwrap()
        .unwrap();
    let current_envelope = object_store
        .read_typed(current_ref_state_id, ObjectType::RefState)
        .unwrap()
        .unwrap();
    let current_payload = RefStatePayload::decode_canonical(
        &current_envelope.canonical_payload,
        current_envelope.schema_version,
    )
    .unwrap();
    let parent_block_id = current_payload.target_object_id;

    let block = signed_block(
        BlockKind::Normal,
        vec![parent_block_id],
        vec![patch_id],
        None,
    );
    let block_id = block.object_id();
    object_store.write_object(&block).unwrap();

    let next_seq = current_payload.update_seq + 1;
    let ref_state =
        signed_ref_state_envelope(ref_name, Some(current_ref_state_id), block_id, next_seq);
    let ref_state_id = ref_state.object_id();
    let ref_update = signed_ref_update_envelope(
        ref_name,
        Some(current_ref_state_id),
        ref_state_id,
        block_id,
        next_seq,
    );
    ref_store
        .publish(&RefPublication {
            ref_name: ref_name.to_string(),
            expected_previous_ref_state_id: Some(current_ref_state_id),
            ref_state,
            ref_update,
        })
        .unwrap();

    let active_lock = ActiveLock::acquire(layout).unwrap();
    finish_active_publication_cleanup(layout, &active_lock).unwrap();
    block_id
}

/// Publish a snapshot-only baseline (path-keyed, no node identity) for the E3 rejection test.
fn publish_snapshot_baseline(layout: &RepositoryLayout, path: &str, bytes: &[u8]) {
    use crate::snapshot::{SnapshotEntry, SnapshotManifest};
    let mut object_store = FileObjectStore::new(layout.clone());
    let manifest = SnapshotManifest {
        files: vec![SnapshotEntry {
            path: RepoPath::parse(path).unwrap(),
            bytes: bytes.to_vec(),
        }],
    };
    let blob = BlobPayload::new(BlobKind::Snapshot, manifest.encode().unwrap());
    let mut blob_env =
        ObjectEnvelope::unsigned(ObjectType::Blob, 1, blob.to_canonical_bytes().unwrap());
    blob_env.add_signature(maintainer_signature()).unwrap();
    let blob_id = blob_env.object_id();
    object_store.write_object(&blob_env).unwrap();

    let payload = BlockPayload {
        parent_block_ids: Vec::new(),
        kind: BlockKind::Normal,
        patch_ids: Vec::new(),
        state_merkle_root: MerkleRoot([0_u8; 32]),
        snapshot_blob_ref: Some(blob_id),
    };
    let mut block =
        ObjectEnvelope::unsigned(ObjectType::Block, 2, payload.to_canonical_bytes().unwrap());
    block.add_signature(maintainer_signature()).unwrap();
    let block_id = block.object_id();
    object_store.write_object(&block).unwrap();
    std::fs::write(layout.root().join(path), bytes).unwrap();

    let ref_store = RefStore::new(layout.clone());
    let ref_state = signed_ref_state_envelope("heads/main", None, block_id, 1);
    let ref_state_id = ref_state.object_id();
    let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, block_id, 1);
    ref_store
        .publish(&RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        })
        .unwrap();
}

#[test]
fn binary_baseline_modified_file_authors_replace_binary() {
    let root = unique_temp_dir("wt-modified-binary");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("data.bin", &[0xff, 0x00], BlobKind::Binary)]);
    std::fs::write(root.join("data.bin"), [0xfe, 0x01]).unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "change binary",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();

    assert_eq!(report.operation_count, 1);
    assert_eq!(report.referenced_blob_count, 2);
    assert_eq!(report.text_edit_count, 0);
    assert_eq!(
        report.changes[0].operation,
        WorktreePatchOperationKind::ReplaceBinary
    );

    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    assert_eq!(replay.records.len(), 1);
    assert_eq!(replay.records[0].envelope.object_id(), report.patch_id);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn text_baseline_modified_file_authors_edit_text() {
    let root = unique_temp_dir("wt-modified-text");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("README.md", b"hello\n", BlobKind::Text)]);
    std::fs::write(root.join("README.md"), b"changed\n").unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "change text",
        WorktreePatchCommitOptions::prefer_text_edits(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();

    assert_eq!(report.operation_count, 1);
    assert_eq!(report.referenced_blob_count, 0);
    assert_eq!(report.text_edit_count, 1);
    assert_eq!(
        report.changes[0].operation,
        WorktreePatchOperationKind::EditText
    );
    let _ = std::fs::remove_dir_all(root);
}

/// DC-65: editing the same text file across N >= 3 separate sealed commits must succeed. Two was the
/// boundary that was missed (a node's `blob_id` after its *first* `EditText` is a content identity,
/// not a stored object — `plan_edit_text` assumed otherwise; `EditText`'s wire shape is a diff, and
/// nothing writes the derived text as a `Blob`). Four consecutive edits here, each sealed before the
/// next, exercises the boundary with margin, and also crosses DC-64's incremental-cache reanchor
/// (the second and later commits are eligible for `resolve_baseline_state`'s incremental path, so
/// this also proves the fix holds under both full-replay and incremental baseline resolution).
#[test]
fn text_file_edited_across_four_sealed_commits_succeeds() {
    let root = unique_temp_dir("wt-four-sealed-edits");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("notes.txt", b"version 0", BlobKind::Text)]);

    let mut generator = deterministic_generator();
    let contents = ["version 1", "version 2", "version 3", "version 4"];
    for content in contents {
        std::fs::write(root.join("notes.txt"), content).unwrap();
        let report = commit_worktree_changes_with_generator(
            &layout,
            "heads/main",
            "edit notes",
            WorktreePatchCommitOptions::prefer_text_edits(),
            &mut generator,
            &test_signer(),
        )
        .unwrap_or_else(|err| panic!("commit for {content:?} failed: {err}"));
        assert_eq!(
            report.text_edit_count, 1,
            "expected an EditText for {content:?}"
        );
        assert_eq!(
            report.changes[0].operation,
            WorktreePatchOperationKind::EditText
        );
        seal_active_patch(&layout, "heads/main");
    }

    // Ground truth: an independent full replay from genesis must reconstruct the final text
    // exactly, proving the chain of diffs — not just each individual commit — is consistent.
    let object_store = FileObjectStore::new(layout.clone());
    let ref_store = RefStore::new(layout.clone());
    let final_ref_state_id = ref_store
        .read_current_ref_state_id("heads/main")
        .unwrap()
        .unwrap();
    let final_envelope = object_store
        .read_typed(final_ref_state_id, ObjectType::RefState)
        .unwrap()
        .unwrap();
    let final_payload = RefStatePayload::decode_canonical(
        &final_envelope.canonical_payload,
        final_envelope.schema_version,
    )
    .unwrap();
    let plan = crate::patch_replay::prepare_patch_replay_plan(&layout, "heads/main").unwrap();
    assert_eq!(plan.target_block_id, final_payload.target_object_id);
    assert_eq!(plan.file_count, 1);
    assert_eq!(plan.total_content_bytes, "version 4".len() as u64);

    let _ = std::fs::remove_dir_all(root);
}

/// DC-65 criterion 4: the `ReplaceBinary` equivalent, confirming binary files were never affected —
/// every `ReplaceBinary` writes its new content as a real stored `Blob` (`plan_replace_binary` always
/// calls `write_content_blob`), so a node's `blob_id` after any number of binary edits always names a
/// stored object.
#[test]
fn binary_file_replaced_across_four_sealed_commits_succeeds() {
    let root = unique_temp_dir("wt-four-sealed-binary-edits");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("data.bin", &[0x00], BlobKind::Binary)]);

    let mut generator = deterministic_generator();
    let contents: [&[u8]; 4] = [&[0x01], &[0x02], &[0x03], &[0x04]];
    for content in contents {
        std::fs::write(root.join("data.bin"), content).unwrap();
        let report = commit_worktree_changes_with_generator(
            &layout,
            "heads/main",
            "replace binary",
            WorktreePatchCommitOptions::file_level(),
            &mut generator,
            &test_signer(),
        )
        .unwrap_or_else(|err| panic!("commit for {content:?} failed: {err}"));
        assert_eq!(
            report.changes[0].operation,
            WorktreePatchOperationKind::ReplaceBinary
        );
        seal_active_patch(&layout, "heads/main");
    }

    // `patch_replay::prepare_patch_replay_plan` (checkout) does not yet support `ReplaceBinary`
    // replay at all — a pre-existing, documented scope limit (`patch_replay.rs`'s module doc:
    // "EditText and ReplaceBinary ... application is deferred to the node model"), unrelated to
    // DC-65. The commit-side authoring loop above, which four consecutive successful
    // `ReplaceBinary` commits already exercised, is what this test verifies.
    let object_store = FileObjectStore::new(layout.clone());
    let ref_store = RefStore::new(layout.clone());
    let final_ref_state_id = ref_store
        .read_current_ref_state_id("heads/main")
        .unwrap()
        .unwrap();
    let final_envelope = object_store
        .read_typed(final_ref_state_id, ObjectType::RefState)
        .unwrap()
        .unwrap();
    let final_payload = RefStatePayload::decode_canonical(
        &final_envelope.canonical_payload,
        final_envelope.schema_version,
    )
    .unwrap();
    assert_eq!(
        final_payload.update_seq, 5,
        "genesis plus four sealed edits"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn binary_baseline_under_prefer_text_still_replace_binary() {
    let root = unique_temp_dir("wt-binary-prefertext");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("data.bin", &[0xff, 0x00], BlobKind::Binary)]);
    std::fs::write(root.join("data.bin"), [0xfe, 0x01]).unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "change binary",
        WorktreePatchCommitOptions::prefer_text_edits(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();

    assert_eq!(report.operation_count, 1);
    assert_eq!(report.text_edit_count, 0);
    assert_eq!(
        report.changes[0].operation,
        WorktreePatchOperationKind::ReplaceBinary
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn untracked_file_authors_create_file() {
    let root = unique_temp_dir("wt-untracked");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("README.md", b"hello\n", BlobKind::Text)]);
    std::fs::write(root.join("extra.txt"), b"extra\n").unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "add extra",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();

    assert_eq!(report.operation_count, 1);
    assert_eq!(report.referenced_blob_count, 1);
    assert_eq!(
        report.changes[0].operation,
        WorktreePatchOperationKind::CreateFile
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn existing_text_node_rejects_non_utf8_content() {
    // E4: an existing TextFile cannot accept non-UTF-8 bytes (text->binary transition fails closed).
    let root = unique_temp_dir("wt-kind-transition");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("README.md", b"hello\n", BlobKind::Text)]);
    std::fs::write(root.join("README.md"), [0xff, 0xfe, 0x00]).unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "corrupt",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    );
    assert!(report.is_err());
    let message = report.err().unwrap().to_string();
    assert!(
        message.contains("unsupported kind transition"),
        "expected kind-transition class, got: {message}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn snapshot_only_baseline_fails_closed() {
    // E3: a snapshot-only baseline carries no node identity; authoring must fail closed.
    let root = unique_temp_dir("wt-snapshot-reject");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_snapshot_baseline(&layout, "README.md", b"hello\n");
    std::fs::write(root.join("README.md"), b"changed\n").unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "change",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    );
    assert!(report.is_err());
    let message = report.err().unwrap().to_string();
    assert!(
        message.contains("node identity unavailable"),
        "expected node-identity-unavailable class, got: {message}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn same_session_creates_get_distinct_node_ids_in_canonical_order() {
    // E1: two fresh creates in one pass are minted in canonical path order with distinct ids.
    let root = unique_temp_dir("wt-same-session");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("README.md", b"hello\n", BlobKind::Text)]);
    std::fs::write(root.join("b.txt"), b"bbb\n").unwrap();
    std::fs::write(root.join("a.txt"), b"aaa\n").unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "two creates",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();

    assert_eq!(report.operation_count, 2);
    assert_eq!(report.referenced_blob_count, 2);
    for change in &report.changes {
        assert_eq!(change.operation, WorktreePatchOperationKind::CreateFile);
    }
    // Inspect the authored patch: a.txt minted before b.txt (canonical path order), distinct ids.
    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    let ops = crate::patch_replay::decode::decode_patch_operations(
        &replay.records[0].envelope.canonical_payload,
    )
    .unwrap();
    let mut node_ids = Vec::new();
    for op in &ops {
        if let crate::patch_replay::decode::DecodedOperationKind::CreateFile {
            path, node_id, ..
        } = &op.kind
        {
            node_ids.push((path.clone(), *node_id.as_bytes()));
        }
    }
    assert_eq!(node_ids.len(), 2);
    assert_ne!(node_ids[0].1, node_ids[1].1);
    // First scripted candidate (…0x01) goes to the canonically-first create path a.txt.
    let a = node_ids.iter().find(|(p, _)| p == "a.txt").unwrap();
    let b = node_ids.iter().find(|(p, _)| p == "b.txt").unwrap();
    assert!(a.1[31] < b.1[31], "a.txt must be minted before b.txt");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn deterministic_patch_identity_across_independent_runs() {
    // E1/ordering: the same change set yields identical patch identity regardless of run, because
    // enumeration is canonical (BTreeMap) and minting is path-sorted under a fixed entropy script.
    let patch_id_of = || {
        let root = unique_temp_dir("wt-determinism");
        let layout = RepositoryLayout::init(root.clone()).unwrap();
        publish_node_baseline(&layout, &[("README.md", b"hello\n", BlobKind::Text)]);
        std::fs::write(root.join("z.txt"), b"zzz\n").unwrap();
        std::fs::write(root.join("m.txt"), b"mmm\n").unwrap();
        let mut generator = deterministic_generator();
        let report = commit_worktree_changes_with_generator(
            &layout,
            "heads/main",
            "two creates",
            WorktreePatchCommitOptions::file_level(),
            &mut generator,
            &test_signer(),
        )
        .unwrap();
        let _ = std::fs::remove_dir_all(root);
        report.patch_id
    };
    assert_eq!(patch_id_of(), patch_id_of());
}

#[test]
fn missing_baseline_file_authors_delete_node() {
    let root = unique_temp_dir("wt-delete");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(
        &layout,
        &[
            ("keep.txt", b"keep\n", BlobKind::Text),
            ("gone.txt", b"gone\n", BlobKind::Text),
        ],
    );
    std::fs::remove_file(root.join("gone.txt")).unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "delete gone",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();

    assert_eq!(report.operation_count, 1);
    assert_eq!(
        report.changes[0].operation,
        WorktreePatchOperationKind::DeleteFile
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn authored_edit_text_locates_and_splices_arbitrary_span_through_shared_text_span() {
    // Authoring↔replay symmetry: the authored arbitrary-span EditText, localized and spliced through the
    // same `text_span` primitives replay uses, reproduces the new bytes and the same text blob id.
    let root = unique_temp_dir("wt-edit-symmetry");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    let old_text = b"hello world\n";
    let new_text = b"hello prikk\n";
    publish_node_baseline(&layout, &[("README.md", old_text, BlobKind::Text)]);
    std::fs::write(root.join("README.md"), new_text).unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "edit",
        WorktreePatchCommitOptions::prefer_text_edits(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();
    assert_eq!(report.text_edit_count, 1);

    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    let ops = crate::patch_replay::decode::decode_patch_operations(
        &replay.records[0].envelope.canonical_payload,
    )
    .unwrap();
    let edit = ops
        .iter()
        .find_map(|op| match &op.kind {
            crate::patch_replay::decode::DecodedOperationKind::EditText {
                node_id,
                span_id,
                old_span_hash,
                left_anchor_hash,
                right_anchor_hash,
                replacement_text,
                old_span_text,
            } => Some((
                *node_id,
                *span_id,
                *old_span_hash,
                *left_anchor_hash,
                *right_anchor_hash,
                replacement_text.clone(),
                old_span_text.clone(),
            )),
            _ => None,
        })
        .expect("authored patch must carry an EditText op");
    let (node_id, span_id, old_span_hash, left, right, replacement, op_old_text) = edit;
    assert_eq!(op_old_text, b"world");
    assert_eq!(replacement, b"prikk");

    // Replay-side localization over the baseline text, using the shared module.
    let (start, end) = crate::text_span::locate_text_span(
        old_text,
        &op_old_text,
        &left,
        &right,
        &span_id,
        node_id,
        &old_span_hash,
    )
    .expect("authored span must localize uniquely in the baseline text");
    let spliced = crate::text_span::splice_text(old_text, start, end, &replacement).unwrap();
    assert_eq!(spliced, new_text);
    assert_eq!(
        crate::text_span::text_blob_id(&spliced).unwrap(),
        crate::text_span::text_blob_id(new_text).unwrap()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn authored_edit_text_widens_subcharacter_utf8_span() {
    let root = unique_temp_dir("wt-edit-subchar");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    let old_text = "é\n".as_bytes();
    let new_text = "è\n".as_bytes();
    publish_node_baseline(&layout, &[("README.md", old_text, BlobKind::Text)]);
    std::fs::write(root.join("README.md"), new_text).unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "edit",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();
    assert_eq!(report.text_edit_count, 1);

    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    let ops = crate::patch_replay::decode::decode_patch_operations(
        &replay.records[0].envelope.canonical_payload,
    )
    .unwrap();
    let edit = ops
        .iter()
        .find_map(|op| match &op.kind {
            crate::patch_replay::decode::DecodedOperationKind::EditText {
                old_span_text,
                replacement_text,
                ..
            } => Some((old_span_text.clone(), replacement_text.clone())),
            _ => None,
        })
        .expect("authored patch must carry an EditText op");
    assert_eq!(edit.0, "é".as_bytes());
    assert_eq!(edit.1, "è".as_bytes());
    let _ = std::fs::remove_dir_all(root);
}

/// Decode the WAL patch and return the `(path, mode)` of every `CreateFile` op.
fn created_file_modes(layout: &RepositoryLayout) -> Vec<(String, u32)> {
    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    let ops = crate::patch_replay::decode::decode_patch_operations(
        &replay.records[0].envelope.canonical_payload,
    )
    .unwrap();
    let mut out = Vec::new();
    for op in &ops {
        if let crate::patch_replay::decode::DecodedOperationKind::CreateFile {
            path, mode, ..
        } = &op.kind
        {
            out.push((path.clone(), *mode));
        }
    }
    out
}

#[test]
fn untracked_regular_file_authors_regular_mode() {
    // 4.4a-2aR: a new non-executable regular file records canonical mode 0o100644.
    let root = unique_temp_dir("wt-create-regular-mode");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("README.md", b"hello\n", BlobKind::Text)]);
    std::fs::write(root.join("extra.txt"), b"extra\n").unwrap();

    let mut generator = deterministic_generator();
    commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "add regular",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();

    let modes = created_file_modes(&layout);
    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ("extra.txt".to_string(), 0o100_644));
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn untracked_executable_file_authors_executable_mode() {
    // 4.4a-2aR: a new file with an executable bit records canonical mode 0o100755 (ratified rule).
    use std::os::unix::fs::PermissionsExt;
    let root = unique_temp_dir("wt-create-exec-mode");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("README.md", b"hello\n", BlobKind::Text)]);
    let script = root.join("run.sh");
    std::fs::write(&script, b"#!/bin/sh\necho hi\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut generator = deterministic_generator();
    commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "add script",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();

    let modes = created_file_modes(&layout);
    assert_eq!(modes.len(), 1);
    assert_eq!(modes[0], ("run.sh".to_string(), 0o100_755));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn mixed_operations_follow_canonical_op_seq_order() {
    // P2-2: delete + create + binary replace + text edit must be emitted in canonical kind order
    // (DeleteNode < CreateFile < ReplaceBinary < EditText), assigned contiguous op_seq after sort.
    let root = unique_temp_dir("wt-mixed-order");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(
        &layout,
        &[
            ("gone.txt", b"gone\n", BlobKind::Text),
            ("bin.dat", &[0xff, 0x00], BlobKind::Binary),
            ("edit.txt", b"old\n", BlobKind::Text),
        ],
    );
    std::fs::remove_file(root.join("gone.txt")).unwrap();
    std::fs::write(root.join("bin.dat"), [0xfe, 0x01]).unwrap();
    std::fs::write(root.join("edit.txt"), b"new\n").unwrap();
    std::fs::write(root.join("new.txt"), b"fresh\n").unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "mixed",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();
    assert_eq!(report.operation_count, 4);

    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    let ops = crate::patch_replay::decode::decode_patch_operations(
        &replay.records[0].envelope.canonical_payload,
    )
    .unwrap();
    use crate::patch_replay::decode::DecodedOperationKind;
    let rank = |kind: &DecodedOperationKind| match kind {
        DecodedOperationKind::DeleteNode { .. } => 0,
        DecodedOperationKind::CreateFile { .. } => 1,
        DecodedOperationKind::ChangePerm { .. } => 2,
        DecodedOperationKind::ReplaceBinary { .. } => 3,
        DecodedOperationKind::EditText { .. } => 4,
        _ => 9,
    };
    let ranks: Vec<i32> = ops.iter().map(|op| rank(&op.kind)).collect();
    assert_eq!(
        ranks,
        vec![0, 1, 3, 4],
        "canonical kind order not preserved"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// Decode the WAL patch and return the `(old_mode, new_mode)` of every `ChangePerm` op.
fn change_perm_modes(layout: &RepositoryLayout) -> Vec<(u32, u32)> {
    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    let ops = crate::patch_replay::decode::decode_patch_operations(
        &replay.records[0].envelope.canonical_payload,
    )
    .unwrap();
    let mut out = Vec::new();
    for op in &ops {
        if let crate::patch_replay::decode::DecodedOperationKind::ChangePerm {
            old_mode,
            new_mode,
            ..
        } = &op.kind
        {
            out.push((*old_mode, *new_mode));
        }
    }
    out
}

#[cfg(unix)]
#[test]
fn mode_only_change_authors_single_change_perm() {
    // 4.4a-2b criteria 2/4/5: content unchanged, normalized mode changed → exactly one ChangePerm
    // with old_mode = baseline (0o100644), new_mode = normalized worktree mode (0o100755).
    use std::os::unix::fs::PermissionsExt;
    let root = unique_temp_dir("wt-mode-only");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("run.sh", b"#!/bin/sh\n", BlobKind::Text)]);
    // Same content, executable bit flipped.
    std::fs::set_permissions(root.join("run.sh"), std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "chmod +x",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();

    assert_eq!(report.operation_count, 1);
    assert_eq!(report.text_edit_count, 0);
    assert_eq!(
        report.changes[0].operation,
        WorktreePatchOperationKind::ChangePerm
    );
    assert_eq!(change_perm_modes(&layout), vec![(0o100_644, 0o100_755)]);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn content_and_mode_change_orders_change_perm_before_edit_text() {
    // 4.4a-2b criterion 3: content + mode change emits ChangePerm + the content op, ChangePerm first.
    use std::os::unix::fs::PermissionsExt;
    let root = unique_temp_dir("wt-content-and-mode");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("run.sh", b"old\n", BlobKind::Text)]);
    std::fs::write(root.join("run.sh"), b"new\n").unwrap();
    std::fs::set_permissions(root.join("run.sh"), std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "edit + chmod",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();
    assert_eq!(report.operation_count, 2);

    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    let ops = crate::patch_replay::decode::decode_patch_operations(
        &replay.records[0].envelope.canonical_payload,
    )
    .unwrap();
    use crate::patch_replay::decode::DecodedOperationKind;
    let ranks: Vec<i32> = ops
        .iter()
        .map(|op| match &op.kind {
            DecodedOperationKind::ChangePerm { .. } => 2,
            DecodedOperationKind::EditText { .. } => 4,
            _ => 9,
        })
        .collect();
    assert_eq!(ranks, vec![2, 4], "ChangePerm must precede EditText");
    assert_eq!(change_perm_modes(&layout), vec![(0o100_644, 0o100_755)]);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn mixed_operations_with_change_perm_follow_canonical_order() {
    // 4.4a-2b criterion 7: extend the mixed-op ordering witness to include ChangePerm — full kind
    // order DeleteNode < CreateFile < ChangePerm < ReplaceBinary < EditText.
    use std::os::unix::fs::PermissionsExt;
    let root = unique_temp_dir("wt-mixed-changeperm");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(
        &layout,
        &[
            ("gone.txt", b"gone\n", BlobKind::Text),
            ("perm.sh", b"#!/bin/sh\n", BlobKind::Text),
            ("bin.dat", &[0xff, 0x00], BlobKind::Binary),
            ("edit.txt", b"old\n", BlobKind::Text),
        ],
    );
    std::fs::remove_file(root.join("gone.txt")).unwrap();
    std::fs::set_permissions(root.join("perm.sh"), std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(root.join("bin.dat"), [0xfe, 0x01]).unwrap();
    std::fs::write(root.join("edit.txt"), b"new\n").unwrap();
    std::fs::write(root.join("new.txt"), b"fresh\n").unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "mixed with chmod",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();
    assert_eq!(report.operation_count, 5);

    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    let ops = crate::patch_replay::decode::decode_patch_operations(
        &replay.records[0].envelope.canonical_payload,
    )
    .unwrap();
    use crate::patch_replay::decode::DecodedOperationKind;
    let ranks: Vec<i32> = ops
        .iter()
        .map(|op| match &op.kind {
            DecodedOperationKind::DeleteNode { .. } => 0,
            DecodedOperationKind::CreateFile { .. } => 1,
            DecodedOperationKind::ChangePerm { .. } => 2,
            DecodedOperationKind::ReplaceBinary { .. } => 3,
            DecodedOperationKind::EditText { .. } => 4,
            _ => 9,
        })
        .collect();
    assert_eq!(
        ranks,
        vec![0, 1, 2, 3, 4],
        "full canonical kind order not preserved"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn content_and_mode_change_orders_change_perm_before_replace_binary() {
    // N1: binary content + mode change → ChangePerm before ReplaceBinary (symmetric to the text case).
    use std::os::unix::fs::PermissionsExt;
    let root = unique_temp_dir("wt-bin-content-and-mode");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("blob.bin", &[0xff, 0x00], BlobKind::Binary)]);
    std::fs::write(root.join("blob.bin"), [0xfe, 0x01]).unwrap();
    std::fs::set_permissions(
        root.join("blob.bin"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "replace + chmod",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();
    assert_eq!(report.operation_count, 2);

    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    let ops = crate::patch_replay::decode::decode_patch_operations(
        &replay.records[0].envelope.canonical_payload,
    )
    .unwrap();
    use crate::patch_replay::decode::DecodedOperationKind;
    let ranks: Vec<i32> = ops
        .iter()
        .map(|op| match &op.kind {
            DecodedOperationKind::ChangePerm { .. } => 2,
            DecodedOperationKind::ReplaceBinary { .. } => 3,
            _ => 9,
        })
        .collect();
    assert_eq!(ranks, vec![2, 3], "ChangePerm must precede ReplaceBinary");
    assert_eq!(change_perm_modes(&layout), vec![(0o100_644, 0o100_755)]);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn non_utf8_worktree_path_fails_closed() {
    // N2: a non-UTF-8 OS path fails closed at the strict conversion boundary, not lossily.
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let root = unique_temp_dir("wt-non-utf8-path");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("README.md", b"hello\n", BlobKind::Text)]);
    // 0xFF is never valid UTF-8.
    let bad = root.join(OsStr::from_bytes(b"bad\xffname.txt"));
    std::fs::write(&bad, b"x\n").unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "non-utf8",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    );
    assert!(report.is_err());
    let message = report.err().unwrap().to_string();
    assert!(
        message.contains("not valid UTF-8"),
        "expected utf-8 rejection, got: {message}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn authored_patch_carries_verifiable_author_signature() {
    // R1: the authored patch carries a real role-bound Ed25519 AUTHOR signature that verifies, and
    // fails verification if object id, role, or key id changes.
    let root = unique_temp_dir("wt-r1-verify");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("README.md", b"hello\n", BlobKind::Text)]);
    std::fs::write(root.join("extra.txt"), b"x\n").unwrap();

    let signer = test_signer();
    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "add",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &signer,
    )
    .unwrap();

    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    let envelope = &replay.records[0].envelope;
    let sig = envelope
        .signatures
        .first()
        .expect("authored patch must carry a signature");
    assert_eq!(sig.algorithm, SignatureAlgorithm::Ed25519);
    assert_eq!(sig.signer_role, SignerRole::Author);
    assert_eq!(sig.key_id, "test-author-key");
    assert_eq!(sig.signature_bytes.len(), 64);

    let public_key = signer.public_key_bytes();
    let good = prikk_object::Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        ObjectType::Patch,
        report.patch_id,
        SignerRole::Author,
        &sig.key_id,
    )
    .unwrap();
    prikk_crypto::verify_ed25519(&public_key, &good, &sig.signature_bytes)
        .expect("the authored AUTHOR signature must verify against the signer's public key");

    // Tamper: a preimage with a different object id, role, or key id must fail verification.
    let other_id = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"different payload");
    let bad_id = prikk_object::Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        ObjectType::Patch,
        other_id,
        SignerRole::Author,
        &sig.key_id,
    )
    .unwrap();
    assert!(prikk_crypto::verify_ed25519(&public_key, &bad_id, &sig.signature_bytes).is_err());

    let bad_role = prikk_object::Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        ObjectType::Patch,
        report.patch_id,
        SignerRole::Maintainer,
        &sig.key_id,
    )
    .unwrap();
    assert!(prikk_crypto::verify_ed25519(&public_key, &bad_role, &sig.signature_bytes).is_err());

    let bad_key = prikk_object::Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        ObjectType::Patch,
        report.patch_id,
        SignerRole::Author,
        "someone-else",
    )
    .unwrap();
    assert!(prikk_crypto::verify_ed25519(&public_key, &bad_key, &sig.signature_bytes).is_err());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn clean_worktree_is_rejected() {
    let root = unique_temp_dir("wt-clean");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    publish_node_baseline(&layout, &[("README.md", b"hello\n", BlobKind::Text)]);

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "nothing",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    );
    assert!(report.is_err());
    let _ = std::fs::remove_dir_all(root);
}

// ---- DC-09 Phase 4.4b: genesis / first-commit authoring ----

/// Genesis: a fresh repo (ref never published) authors every worktree file as a `CreateFile`, in
/// canonical path order, carrying a real role-bound Ed25519 AUTHOR signature (acceptance 2, 3, 8).
#[test]
fn genesis_commit_authors_all_create_file() {
    let root = unique_temp_dir("wt-genesis-create");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    // No published baseline. Two files in the worktree.
    std::fs::write(root.join("readme.txt"), b"hello\n").unwrap();
    std::fs::write(root.join("main.rs"), b"fn main() {}\n").unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "genesis",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();

    assert_eq!(report.operation_count, 2);
    // Canonical path order: main.rs before readme.txt.
    assert_eq!(
        report.changes[0].operation,
        WorktreePatchOperationKind::CreateFile
    );
    assert_eq!(report.changes[0].path, "main.rs");
    assert_eq!(report.changes[1].path, "readme.txt");

    // Real AUTHOR signature on the genesis patch (same signer path as published authoring).
    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    assert_eq!(replay.records.len(), 1);
    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Valid("heads/main".to_string())
    );
    let env = &replay.records[0].envelope;
    assert_eq!(env.object_id(), report.patch_id);
    let sig = env.signatures.first().expect("genesis patch is signed");
    assert_eq!(sig.algorithm, SignatureAlgorithm::Ed25519);
    assert_eq!(sig.signer_role, SignerRole::Author);
    assert_eq!(sig.key_id, "test-author-key");
    assert_ne!(sig.key_id, "dev-placeholder-author");
    let _ = std::fs::remove_dir_all(root);
}

/// Genesis on an empty worktree fails closed (no zero-operation patch) (acceptance 5).
#[test]
fn genesis_empty_worktree_fails_closed() {
    let root = unique_temp_dir("wt-genesis-empty");
    let layout = RepositoryLayout::init(root.clone()).unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "genesis-empty",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    );
    assert!(report.is_err());
    let _ = std::fs::remove_dir_all(root);
}

/// Genesis E1 guard: a second commit before the first seal fails closed rather than authoring a
/// duplicate genesis patch (review E1, acceptance 7).
#[test]
fn genesis_second_commit_before_seal_fails_closed() {
    let root = unique_temp_dir("wt-genesis-double");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    std::fs::write(root.join("a.txt"), b"one\n").unwrap();

    let mut generator = deterministic_generator();
    commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "genesis",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();

    // Second commit before seal: active WAL already has the genesis patch.
    std::fs::write(root.join("b.txt"), b"two\n").unwrap();
    let mut generator2 = deterministic_generator();
    let err = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "again",
        WorktreePatchCommitOptions::file_level(),
        &mut generator2,
        &test_signer(),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("active WAL already contains patches"),
        "unexpected error: {err}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// Genesis-vs-corruption: a missing ref pointer with existing ref-log history is NOT genesis; it
/// fails closed and points at recovery, never silently re-genesis (design §4, acceptance 6).
#[test]
fn genesis_missing_pointer_with_log_fails_closed() {
    let root = unique_temp_dir("wt-genesis-corrupt");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    // Publish a baseline (writes ref pointer + ref log), then remove only the pointer.
    publish_node_baseline(&layout, &[("readme.txt", b"hello\n", BlobKind::Text)]);
    std::fs::remove_file(layout.ref_pointer_path("heads/main")).unwrap();
    std::fs::write(root.join("readme.txt"), b"changed\n").unwrap();

    let mut generator = deterministic_generator();
    let err = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "should-not-genesis",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("repository mutation is blocked by incomplete ref publication"),
        "unexpected error: {err}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// DC-13: a first commit onto an explicit unborn non-default branch ref authors an independent Root
/// history from the current worktree and records active-WAL ref ownership.
#[test]
fn genesis_on_non_default_ref_authors_create_file() {
    let root = unique_temp_dir("wt-genesis-nondefault");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    std::fs::write(root.join("a.txt"), b"one\n").unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/feature",
        "genesis-nondefault",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();
    assert_eq!(report.ref_name, "heads/feature");
    assert_eq!(report.operation_count, 1);
    assert_eq!(
        report.changes[0].operation,
        WorktreePatchOperationKind::CreateFile
    );
    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Valid("heads/feature".to_string())
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn invalid_branch_ref_fails_before_active_mutation() {
    let root = unique_temp_dir("wt-genesis-invalid-ref");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    std::fs::write(root.join("a.txt"), b"one\n").unwrap();

    let mut generator = deterministic_generator();
    let err = commit_worktree_changes_with_generator(
        &layout,
        "tags/v1",
        "invalid-ref",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("reserved"),
        "unexpected error: {err}"
    );
    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    assert!(replay.records.is_empty());
    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Missing
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn empty_wal_malformed_active_ref_metadata_is_cleaned_before_commit() {
    let root = unique_temp_dir("wt-genesis-clean-stale-ref");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    std::fs::write(layout.default_active_ref_name_path(), b"heads//bad").unwrap();
    std::fs::write(root.join("a.txt"), b"one\n").unwrap();

    let mut generator = deterministic_generator();
    let report = commit_worktree_changes_with_generator(
        &layout,
        "heads/topic",
        "genesis-topic",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();
    assert_eq!(report.ref_name, "heads/topic");
    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Valid("heads/topic".to_string())
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn non_empty_wal_missing_active_ref_metadata_fails_closed() {
    let root = unique_temp_dir("wt-active-ref-missing");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    std::fs::write(root.join("a.txt"), b"one\n").unwrap();

    let mut generator = deterministic_generator();
    commit_worktree_changes_with_generator(
        &layout,
        "heads/topic",
        "genesis-topic",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();
    std::fs::remove_file(layout.default_active_ref_name_path()).unwrap();
    std::fs::write(root.join("b.txt"), b"two\n").unwrap();

    let mut generator2 = deterministic_generator();
    let err = commit_worktree_changes_with_generator(
        &layout,
        "heads/topic",
        "again",
        WorktreePatchCommitOptions::file_level(),
        &mut generator2,
        &test_signer(),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("metadata is missing"),
        "unexpected error: {err}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn non_empty_wal_malformed_active_ref_metadata_fails_closed() {
    let root = unique_temp_dir("wt-active-ref-malformed");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    std::fs::write(root.join("a.txt"), b"one\n").unwrap();

    let mut generator = deterministic_generator();
    commit_worktree_changes_with_generator(
        &layout,
        "heads/topic",
        "genesis-topic",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap();
    std::fs::write(layout.default_active_ref_name_path(), b"heads//bad").unwrap();
    std::fs::write(root.join("b.txt"), b"two\n").unwrap();

    let mut generator2 = deterministic_generator();
    let err = commit_worktree_changes_with_generator(
        &layout,
        "heads/topic",
        "again",
        WorktreePatchCommitOptions::file_level(),
        &mut generator2,
        &test_signer(),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("metadata is malformed"),
        "unexpected error: {err}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// 4.4bR P1b: genesis requires a clean active WAL. A trailing partial WAL tail (fewer bytes than a
/// record header) fails closed and points at `doctor --repair-wal-tail` rather than appending after
/// the partial tail.
#[test]
fn genesis_with_trailing_partial_wal_fails_closed() {
    let root = unique_temp_dir("wt-genesis-partialwal");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    std::fs::write(root.join("a.txt"), b"one\n").unwrap();

    // Seed the active WAL with a trailing partial tail (< one record header, 0 complete records).
    let wal_path = layout.default_queue_wal_path();
    if let Some(parent) = wal_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&wal_path, [0xAB_u8; 10]).unwrap();

    let mut generator = deterministic_generator();
    let err = commit_worktree_changes_with_generator(
        &layout,
        "heads/main",
        "genesis-partial",
        WorktreePatchCommitOptions::file_level(),
        &mut generator,
        &test_signer(),
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("trailing partial bytes"),
        "unexpected error: {err}"
    );
    let _ = std::fs::remove_dir_all(root);
}

/// 4.4bR2: the active-WAL guard is atomic with the append under the active-session lock. Two
/// concurrent genesis commits on the same fresh repo serialize — exactly one succeeds, and the active
/// WAL ends with exactly one Patch record (seq 1, no trailing partial). The loser fails closed via a
/// lock conflict or the post-lock "seal first" active-WAL guard.
#[test]
fn concurrent_genesis_commits_serialize_to_one_record() {
    let root = unique_temp_dir("wt-genesis-concurrent");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    std::fs::write(root.join("a.txt"), b"one\n").unwrap();
    std::fs::write(root.join("b.txt"), b"two\n").unwrap();

    let la = layout.clone();
    let lb = layout.clone();
    let h1 = std::thread::spawn(move || {
        super::commit_worktree_changes_signed(
            &la,
            "heads/main",
            "genesis-a",
            WorktreePatchCommitOptions::file_level(),
            &test_signer(),
        )
        .is_ok()
    });
    let h2 = std::thread::spawn(move || {
        super::commit_worktree_changes_signed(
            &lb,
            "heads/main",
            "genesis-b",
            WorktreePatchCommitOptions::file_level(),
            &test_signer(),
        )
        .is_ok()
    });
    let ok1 = h1.join().unwrap();
    let ok2 = h2.join().unwrap();

    let ok_count = [ok1, ok2].into_iter().filter(|ok| *ok).count();
    assert_eq!(
        ok_count, 1,
        "exactly one concurrent genesis commit must succeed"
    );

    // The active WAL holds exactly one Patch record, sequence 1, no trailing partial bytes.
    let replay = Wal::new(layout.default_queue_wal_path()).replay().unwrap();
    assert_eq!(
        replay.records.len(),
        1,
        "active WAL must hold exactly one record"
    );
    assert_eq!(replay.records[0].seq, 1);
    assert_eq!(replay.trailing_partial_bytes, 0);
    let _ = std::fs::remove_dir_all(root);
}
