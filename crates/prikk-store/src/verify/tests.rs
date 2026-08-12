//! Repository verification tests.

mod ref_cluster;
mod root_authority;
mod trust;

use prikk_error::{PrikkError, Result};
use prikk_object::{
    BlobKind, BlobPayload, BlockKind, BlockPayload, CanonicalEncode, CreateFile, MerkleRoot,
    NodeId, ObjectEnvelope, ObjectId, ObjectType, Operation, OperationKind, PatchPayload,
    PatchPurpose,
};

use crate::maintainer_signing::MaintainerSigner;
use crate::wal::{WalRecord, encode_record_for_test};
use crate::{
    ActiveWalMetadataStatus, FileObjectStore, ObjectWriter, RepositoryLayout, Wal,
    derive_next_state_root, verify_repository, write_active_ref_metadata,
};

use crate::test_support::{
    dummy_signature, maintainer_signature, rollback_patch_envelope, sample_object_id,
    signed_patch_envelope, signed_ref_state_envelope, unique_temp_dir,
};

/// DC-95 Stage 1, round 2: the three "referenced object is missing" checks in `verify_block_payload`
/// (`verify.rs`) -- parent block, patch, and snapshot blob. Supersedes the older, weaker `verify_
/// repository_detects_block_with_missing_patch` (asserted only `.is_err()`) with a table asserting
/// each check's own specific message, per the round-1 condition's standard. Each fixture is shape-valid
/// (isolating the existence check from `validate_block_v2_shape`) -- a `Normal` block referencing one
/// nonexistent parent for the parent-existence row, `Root` blocks otherwise.
///
/// **The `missing-snapshot-blob` row uses a replay-correct root** (`snapshot_blob_ref` is metadata only
/// -- never read by state derivation, confirmed by tracing `derive_next_state_root`/`apply_candidate_
/// patches`, neither of which touch it -- so a correct root is always computable regardless of whether
/// the snapshot blob exists). Disabling the snapshot-blob check on this row was confirmed, by an actual
/// probe, to let `verify_repository` return `Ok` -- a clean pass, not a differently-worded rejection --
/// so this row genuinely demonstrates Stage 1's own rule. **Re-verified against a genuinely clean
/// baseline** (DC-95-stage-1-round-5-review-v1 §2-4): the original probe's fixture used a fake,
/// unadopted signer, so its `Ok` result always carried `PRIKK-TRUST-POLICY-INVALID` regardless of
/// the check's own state. Re-probed with a real, adopted signer behind the Block: disabling the
/// check now returns `Ok` with every issue vector empty. Classification unchanged: load-bearing.
///
/// **`missing-parent` and `missing-patch` use arbitrary roots, and cannot do otherwise: reported, not
/// silently inconsistent with round 1's standard.** Computing a replay-correct root for either requires
/// *reading* the referenced object to derive from it -- exactly what "missing" makes impossible. Probed
/// anyway, to learn what disabling each check actually does rather than assume it would be confounded
/// the same way an arbitrary root was in round 1: disabling the parent-existence loop still rejects the
/// block, via `validate_v2_lineage`'s own independent "format-2 parent Block {id} is missing" read in
/// Phase B; disabling the patch-existence check still rejects it, via the lifecycle-replay layer's own
/// "patch {id} is malformed (patch object is missing)" when Phase B tries to replay it. **Both are
/// redundant with a downstream read for `CurrentV2` blocks specifically** -- disabling `verify_block_
/// payload`'s own explicit check does not let a bad repository verify clean, because something else
/// already reads the same reference and fails closed too. That is a real property of the current design,
/// not a gap this round's test can paper over with a placeholder root, and it is why these two rows are
/// regression guards on `verify_block_payload`'s own message (useful for diagnostics -- "which check
/// said so" matters to an operator) rather than the "silent absence" demonstration `missing-snapshot-
/// blob` gives directly.
#[test]
fn verify_repository_detects_every_missing_referenced_object() -> Result<()> {
    type CaseFn = fn(&FileObjectStore, ObjectId) -> Result<(BlockPayload, &'static str)>;
    let cases: Vec<(&str, CaseFn)> = vec![
        ("missing-parent", |_store, missing| {
            Ok((
                BlockPayload {
                    parent_block_ids: vec![missing],
                    kind: BlockKind::Normal,
                    patch_ids: Vec::new(),
                    state_merkle_root: MerkleRoot([0xC0_u8; 32]),
                    snapshot_blob_ref: None,
                    mainline_parent_id: None,
                    merge_baseline_block_id: None,
                },
                "references missing parent block",
            ))
        }),
        ("missing-patch", |_store, missing| {
            Ok((
                BlockPayload {
                    parent_block_ids: Vec::new(),
                    kind: BlockKind::Root,
                    patch_ids: vec![missing],
                    state_merkle_root: MerkleRoot([0xC1_u8; 32]),
                    snapshot_blob_ref: None,
                    mainline_parent_id: None,
                    merge_baseline_block_id: None,
                },
                "references missing block patch",
            ))
        }),
        ("missing-snapshot-blob", |store, missing| {
            let state_merkle_root = derive_next_state_root(store, None, &[])?;
            Ok((
                BlockPayload {
                    parent_block_ids: Vec::new(),
                    kind: BlockKind::Root,
                    patch_ids: Vec::new(),
                    state_merkle_root,
                    snapshot_blob_ref: Some(missing),
                    mainline_parent_id: None,
                    merge_baseline_block_id: None,
                },
                "references missing snapshot blob",
            ))
        }),
    ];

    for (name, case_fn) in cases {
        let root = unique_temp_dir(&format!("missing-referenced-{name}"));
        let layout = RepositoryLayout::init(root.clone())?;
        let mut store = FileObjectStore::new(layout.clone());
        let missing = sample_object_id(&format!("{name}-target"));
        let (payload, expected_substring) = case_fn(&store, missing)?;
        write_signed_block(&mut store, &payload)?;

        let error = match verify_repository(&layout) {
            Ok(_) => {
                panic!("case {name:?}: expected verify_repository to reject a missing reference")
            }
            Err(error) => error.to_string(),
        };
        assert!(
            error.contains(expected_substring),
            "case {name:?}: expected error containing {expected_substring:?}, got: {error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

/// DC-92 implementation review §4: an end-to-end control that `verify` actually performs block state
/// verification through the real `verify_objects` wiring — Phase A's collection into
/// `pending_v2_blocks`, then Phase B's `verify_blocks_topological` — not merely in the unit-level
/// `verify_block_v2_state`/`verify_blocks_topological` calls `block_state::tests` exercises directly
/// against a `MemoryObjectStore`. The review found that removing the inline state-check call entirely
/// (on `main`, pre-DC-92, and again after DC-92's restructuring) left the whole workspace suite
/// green — nothing wired the two together. Built, not byte-corrupted, matching this module's own
/// `verify_repository_detects_every_missing_referenced_object`: content addressing means a post-hoc-
/// corrupted object is just a different, self-consistent valid object, never a mismatch.
#[test]
fn verify_repository_detects_block_with_state_root_mismatch() -> Result<()> {
    let root = unique_temp_dir("block-state-root-mismatch");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut store = FileObjectStore::new(layout.clone());

    // A Root block over empty history, claiming a state root that is not the true empty-state root
    // (`derive_next_state_root(&store, None, &[])`) -- wrong only in what it claims, otherwise
    // shape-valid and schema-valid.
    write_signed_block(
        &mut store,
        &BlockPayload {
            parent_block_ids: Vec::new(),
            kind: BlockKind::Root,
            patch_ids: Vec::new(),
            state_merkle_root: MerkleRoot([0xEE_u8; 32]),
            snapshot_blob_ref: None,
            mainline_parent_id: None,
            merge_baseline_block_id: None,
        },
    )?;

    assert!(verify_repository(&layout).is_err());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-95 Stage 1, §5.2: the 8 `validate_block_v2_shape` error arms, proven through `verify_repository`
/// rather than only at the unit level (`block_state/tests.rs`'s own `format2_parent_and_kind_matrix_
/// is_closed`/`format2_merge_shape_matrix`). The review's own probe found that disabling shape
/// validation entirely left every existing test passing -- including DC-92's own lineage-member shape
/// violation test, which calls `verify_blocks_topological` directly -- because none of them reach the
/// check through `verify_repository`. One fresh repository per row (not one growing repository across
/// rows: `verify_repository` stops at the first hard error, so a shared repository would only ever
/// prove whichever row's block sorts first by `ObjectId`, not the row under test). Each row's payload
/// is built from a shared set of real parent blocks so it's wrong only in the one field under test.
///
/// **Every row's `state_merkle_root` is the replay-*correct* root for what `state_derivation_parent`
/// would resolve if shape validation did not run first** -- computed via `derive_next_state_root`
/// against exactly that parent, never an arbitrary placeholder. This is DC-92's own isolation
/// discipline (`naive_continue`'s doc comment, `block_state/tests.rs`), required here for the same
/// reason a first review round found missing: an arbitrary root lets the *state-root* check catch a
/// shape-invalid fixture instead, so disabling shape validation alone would not make the row pass and
/// the test would prove nothing about shape specifically. Confirmed by re-deriving `state_derivation_
/// parent`'s own match arms for each row: non-`Merge` kinds resolve to `parent_block_ids.first()`
/// (`None` when empty, regardless of shape validity), `Merge` resolves to `mainline_parent_id` directly
/// (`None` when absent, unchecked against `parent_block_ids` when present) -- so every row here has a
/// well-defined resolved parent, and none needed `naive_continue`'s from-scratch-continuation trick,
/// since no row builds on an already-corrupted ancestor.
///
/// **Load-bearing classification re-verified against a genuinely clean baseline** (DC-95-stage-1-
/// round-5-review-v1 §2-4): the original disable-and-restore probe used these fixtures' own fake,
/// unadopted signer, so every probe's `Ok` result always carried a `PRIKK-TRUST-POLICY-INVALID`
/// finding regardless of the shape check's own state -- the repository could never have "verified
/// clean" either way, making the original probe's result unable to distinguish load-bearing from
/// downstream-redundant. Re-probed all 8 rows with a real, adopted `Ed25519MaintainerSigner` behind
/// every Block: with the check enabled, all 8 reject with their original messages unchanged; with
/// `validate_block_v2_shape` disabled, all 8 return `Ok` with `publication_trust_issues`,
/// `ref_publication_issues`, and `signature_envelope_issues` all empty -- genuinely clean, not
/// merely `Ok`. Classification unchanged: all 8 confirmed load-bearing.
#[test]
fn verify_repository_detects_every_block_shape_violation() -> Result<()> {
    type CaseFn =
        fn(&FileObjectStore, ObjectId, ObjectId, ObjectId) -> Result<(BlockPayload, &'static str)>;
    let cases: Vec<(&str, CaseFn)> = vec![
        ("root-with-parent", |store, genesis, _a, _b| {
            // state_derivation_parent(Root, ..) = parent_block_ids.first() = Some(genesis).
            let state_merkle_root = derive_next_state_root(store, Some(genesis), &[])?;
            Ok((
                BlockPayload {
                    parent_block_ids: vec![genesis],
                    kind: BlockKind::Root,
                    patch_ids: Vec::new(),
                    state_merkle_root,
                    snapshot_blob_ref: None,
                    mainline_parent_id: None,
                    merge_baseline_block_id: None,
                },
                "Root Block must have zero parents",
            ))
        }),
        ("normal-with-zero-parents", |store, _genesis, _a, _b| {
            // state_derivation_parent(Normal, []) = parent_block_ids.first() = None.
            let state_merkle_root = derive_next_state_root(store, None, &[])?;
            Ok((
                BlockPayload {
                    parent_block_ids: Vec::new(),
                    kind: BlockKind::Normal,
                    patch_ids: Vec::new(),
                    state_merkle_root,
                    snapshot_blob_ref: None,
                    mainline_parent_id: None,
                    merge_baseline_block_id: None,
                },
                "Normal Block must have exactly one parent",
            ))
        }),
        ("merge-with-one-parent", |store, genesis, _a, _b| {
            // state_derivation_parent(Merge, ..) = mainline_parent_id = Some(genesis).
            let state_merkle_root = derive_next_state_root(store, Some(genesis), &[])?;
            Ok((
                BlockPayload {
                    parent_block_ids: vec![genesis],
                    kind: BlockKind::Merge,
                    patch_ids: Vec::new(),
                    state_merkle_root,
                    snapshot_blob_ref: None,
                    mainline_parent_id: Some(genesis),
                    merge_baseline_block_id: Some(genesis),
                },
                "Merge Block must have exactly two parents",
            ))
        }),
        ("repair-kind-unauthorized", |store, _genesis, _a, _b| {
            // state_derivation_parent(Repair, []) = parent_block_ids.first() = None.
            let state_merkle_root = derive_next_state_root(store, None, &[])?;
            Ok((
                BlockPayload {
                    parent_block_ids: Vec::new(),
                    kind: BlockKind::Repair,
                    patch_ids: Vec::new(),
                    state_merkle_root,
                    snapshot_blob_ref: None,
                    mainline_parent_id: None,
                    merge_baseline_block_id: None,
                },
                "Block kind is not authorized",
            ))
        }),
        ("root-with-mainline-field", |store, genesis, _a, _b| {
            // state_derivation_parent ignores mainline_parent_id for non-Merge kinds:
            // parent_block_ids.first() = None (parent_block_ids is empty here).
            let state_merkle_root = derive_next_state_root(store, None, &[])?;
            Ok((
                BlockPayload {
                    parent_block_ids: Vec::new(),
                    kind: BlockKind::Root,
                    patch_ids: Vec::new(),
                    state_merkle_root,
                    snapshot_blob_ref: None,
                    mainline_parent_id: Some(genesis),
                    merge_baseline_block_id: None,
                },
                "must not carry a mainline parent or merge baseline",
            ))
        }),
        ("merge-without-mainline", |store, genesis, a, b| {
            // state_derivation_parent(Merge, ..) = mainline_parent_id = None here, regardless of
            // parent_block_ids -- so the resolved parent is genesis-equivalent (empty), not a or b.
            let mut parents = vec![a, b];
            parents.sort();
            let state_merkle_root = derive_next_state_root(store, None, &[])?;
            Ok((
                BlockPayload {
                    parent_block_ids: parents,
                    kind: BlockKind::Merge,
                    patch_ids: Vec::new(),
                    state_merkle_root,
                    snapshot_blob_ref: None,
                    mainline_parent_id: None,
                    merge_baseline_block_id: Some(genesis),
                },
                "Merge Block must name a mainline parent",
            ))
        }),
        ("merge-mainline-not-a-parent", |store, genesis, a, b| {
            // state_derivation_parent(Merge, ..) = mainline_parent_id = Some(genesis) directly --
            // never checked against parent_block_ids at this stage, that's exactly the shape rule
            // being bypassed.
            let mut parents = vec![a, b];
            parents.sort();
            let state_merkle_root = derive_next_state_root(store, Some(genesis), &[])?;
            Ok((
                BlockPayload {
                    parent_block_ids: parents,
                    kind: BlockKind::Merge,
                    patch_ids: Vec::new(),
                    state_merkle_root,
                    snapshot_blob_ref: None,
                    mainline_parent_id: Some(genesis),
                    merge_baseline_block_id: Some(genesis),
                },
                "mainline parent must be one of its own parents",
            ))
        }),
        ("merge-without-baseline", |store, _genesis, a, b| {
            // state_derivation_parent(Merge, ..) = mainline_parent_id = Some(a).
            let mut parents = vec![a, b];
            parents.sort();
            let state_merkle_root = derive_next_state_root(store, Some(a), &[])?;
            Ok((
                BlockPayload {
                    parent_block_ids: parents,
                    kind: BlockKind::Merge,
                    patch_ids: Vec::new(),
                    state_merkle_root,
                    snapshot_blob_ref: None,
                    mainline_parent_id: Some(a),
                    merge_baseline_block_id: None,
                },
                "must record the baseline confluence was proven against",
            ))
        }),
    ];

    for (name, case_fn) in cases {
        let root = unique_temp_dir(&format!("block-shape-{name}"));
        let layout = RepositoryLayout::init(root.clone())?;
        let mut store = FileObjectStore::new(layout.clone());

        let genesis_root = derive_next_state_root(&store, None, &[])?;
        let genesis = write_signed_block(
            &mut store,
            &BlockPayload {
                parent_block_ids: Vec::new(),
                kind: BlockKind::Root,
                patch_ids: Vec::new(),
                state_merkle_root: genesis_root,
                snapshot_blob_ref: None,
                mainline_parent_id: None,
                merge_baseline_block_id: None,
            },
        )?;
        let parent_a = write_create_child(&mut store, genesis, "a.txt", 0x51)?;
        let parent_b = write_create_child(&mut store, genesis, "b.txt", 0x52)?;

        let (payload, expected_substring) = case_fn(&store, genesis, parent_a, parent_b)?;
        write_signed_block(&mut store, &payload)?;

        let error = match verify_repository(&layout) {
            Ok(_) => {
                panic!("case {name:?}: expected verify_repository to reject a shape violation")
            }
            Err(error) => error.to_string(),
        };
        assert!(
            error.contains(expected_substring),
            "case {name:?}: expected error containing {expected_substring:?}, got: {error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

fn write_signed_block(store: &mut FileObjectStore, payload: &BlockPayload) -> Result<ObjectId> {
    let payload_bytes = payload.to_canonical_bytes()?;
    let mut block = ObjectEnvelope::unsigned(ObjectType::Block, 2, payload_bytes);
    block.add_signature(maintainer_signature())?;
    store.write_object(&block)
}

/// Seals a child block over `parent` with one `CreateFile` patch at `path` — real, replayable
/// content, so two children of the same parent are distinguishable (a content-addressed patch-free
/// `Normal` child of a given parent is otherwise a single, unique object; there is only one way to
/// say "nothing changed").
fn write_create_child(
    store: &mut FileObjectStore,
    parent: ObjectId,
    path: &str,
    node_byte: u8,
) -> Result<ObjectId> {
    let blob = BlobPayload::new(BlobKind::Text, format!("{path}\n").into_bytes());
    let mut blob_env = ObjectEnvelope::unsigned(ObjectType::Blob, 1, blob.to_canonical_bytes()?);
    blob_env.add_signature(maintainer_signature())?;
    let blob_id = store.write_object(&blob_env)?;

    let patch = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::CreateFile(CreateFile {
                path: path.to_string(),
                node_id: NodeId::from_bytes([node_byte; 32]),
                blob_id,
                mode: 0o100_644,
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let mut patch_env = ObjectEnvelope::unsigned(ObjectType::Patch, 1, patch.to_canonical_bytes()?);
    patch_env.add_signature(maintainer_signature())?;
    let patch_id = store.write_object(&patch_env)?;

    let state_merkle_root = derive_next_state_root(store, Some(parent), &[patch_id])?;
    write_signed_block(
        store,
        &BlockPayload {
            parent_block_ids: vec![parent],
            kind: BlockKind::Normal,
            patch_ids: vec![patch_id],
            state_merkle_root,
            snapshot_blob_ref: None,
            mainline_parent_id: None,
            merge_baseline_block_id: None,
        },
    )
}

/// DC-75: a `Merge` block's `merge_baseline_block_id` is a claim `verify` independently re-derives,
/// not trusts. This constructs a `Merge` block whose recorded baseline is not reachable from either
/// parent at all — forged, distinct from `genesis`, the actual common ancestor — and confirms
/// `verify` still passes structurally (shape and state root are both genuinely valid) but reports
/// the divergence rather than silently accepting the claim.
#[test]
fn verify_repository_flags_merge_block_with_baseline_not_a_common_ancestor() -> Result<()> {
    let root = unique_temp_dir("merge-baseline-forged");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut store = FileObjectStore::new(layout.clone());

    let genesis_root = derive_next_state_root(&store, None, &[])?;
    let genesis = write_signed_block(
        &mut store,
        &BlockPayload {
            parent_block_ids: Vec::new(),
            kind: BlockKind::Root,
            patch_ids: Vec::new(),
            state_merkle_root: genesis_root,
            snapshot_blob_ref: None,
            mainline_parent_id: None,
            merge_baseline_block_id: None,
        },
    )?;
    let mainline_parent = write_create_child(&mut store, genesis, "mainline.txt", 0x10)?;
    let secondary_parent = write_create_child(&mut store, genesis, "secondary.txt", 0x20)?;
    let mut parents = vec![mainline_parent, secondary_parent];
    parents.sort();
    let forged_baseline = sample_object_id("forged-baseline-not-an-ancestor");
    let merge_root = derive_next_state_root(&store, Some(mainline_parent), &[])?;
    let merge_block = write_signed_block(
        &mut store,
        &BlockPayload {
            parent_block_ids: parents,
            kind: BlockKind::Merge,
            patch_ids: Vec::new(),
            state_merkle_root: merge_root,
            snapshot_blob_ref: None,
            mainline_parent_id: Some(mainline_parent),
            merge_baseline_block_id: Some(forged_baseline),
        },
    )?;

    let report = verify_repository(&layout)?;
    assert!(report.has_merge_baseline_divergence());
    assert_eq!(report.merge_baseline_divergences.len(), 1);
    let Some(divergence) = report.merge_baseline_divergences.first() else {
        panic!("expected exactly one merge-baseline divergence");
    };
    assert_eq!(divergence.block_id, merge_block);
    assert_eq!(divergence.recorded_baseline, forged_baseline);
    assert_eq!(divergence.mainline_parent_id, mainline_parent);
    assert_eq!(divergence.secondary_parent_id, secondary_parent);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// The positive-path counterpart: a `Merge` block recording its true, genuine common ancestor as the
/// baseline reports no divergence.
#[test]
fn verify_repository_accepts_merge_block_with_genuine_common_ancestor_baseline() -> Result<()> {
    let root = unique_temp_dir("merge-baseline-genuine");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut store = FileObjectStore::new(layout.clone());

    let genesis_root = derive_next_state_root(&store, None, &[])?;
    let genesis = write_signed_block(
        &mut store,
        &BlockPayload {
            parent_block_ids: Vec::new(),
            kind: BlockKind::Root,
            patch_ids: Vec::new(),
            state_merkle_root: genesis_root,
            snapshot_blob_ref: None,
            mainline_parent_id: None,
            merge_baseline_block_id: None,
        },
    )?;
    let mainline_parent = write_create_child(&mut store, genesis, "mainline.txt", 0x30)?;
    let secondary_parent = write_create_child(&mut store, genesis, "secondary.txt", 0x40)?;
    let mut parents = vec![mainline_parent, secondary_parent];
    parents.sort();
    let merge_root = derive_next_state_root(&store, Some(mainline_parent), &[])?;
    write_signed_block(
        &mut store,
        &BlockPayload {
            parent_block_ids: parents,
            kind: BlockKind::Merge,
            patch_ids: Vec::new(),
            state_merkle_root: merge_root,
            snapshot_blob_ref: None,
            mainline_parent_id: Some(mainline_parent),
            merge_baseline_block_id: Some(genesis),
        },
    )?;

    let report = verify_repository(&layout)?;
    assert!(!report.has_merge_baseline_divergence());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn verify_repository_counts_objects_and_wal_records() {
    let root = unique_temp_dir("verify");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut store = FileObjectStore::new(layout.clone());
        let mut blob = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"payload".to_vec());
        assert!(blob.add_signature(dummy_signature()).is_ok());
        assert!(store.write_object(&blob).is_ok());

        let wal = Wal::for_layout(&layout);
        assert!(write_active_ref_metadata(&layout, "heads/main").is_ok());
        assert!(wal.append_patch(&signed_patch_envelope()).is_ok());

        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.checked_objects, 1);
            assert_eq!(report.checked_blocks, 0);
            assert_eq!(report.checked_wal_records, 1);
            assert_eq!(report.persisted_wal_patches, 0);
            assert_eq!(report.checked_refs, 0);
            assert_eq!(report.checked_ref_log_records, 0);
            assert_eq!(report.trailing_partial_wal_bytes, 0);
            assert_eq!(
                report.active_wal_metadata_status,
                ActiveWalMetadataStatus::ValidForNonEmptyWal {
                    ref_name: "heads/main".to_string()
                }
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

/// DC-66 criterion 6: `verify` reports queue ordering explicitly. Reachable only by direct file
/// tampering — `Wal::append_patch` always assigns `previous.seq + 1` — but a queue of N gives
/// "ordering" a meaning worth verifying rather than assuming from successful structural decode.
#[test]
fn verify_repository_reports_active_wal_ordering_violation() {
    let root = unique_temp_dir("verify-wal-ordering");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let wal = Wal::for_layout(&layout);
        // Ensure the WAL file and its parent directory exist, then overwrite with two hand-crafted
        // records sharing sequence 1 — an ordering violation no append path can produce.
        assert!(wal.append_patch(&signed_patch_envelope()).is_ok());
        let first = WalRecord {
            seq: 1,
            envelope: signed_patch_envelope(),
        };
        let second = WalRecord {
            seq: 1,
            envelope: rollback_patch_envelope(),
        };
        let mut bytes = Vec::new();
        let first_encoded = encode_record_for_test(&first);
        assert!(first_encoded.is_ok());
        if let Ok(first_encoded) = first_encoded {
            bytes.extend(first_encoded);
        }
        let second_encoded = encode_record_for_test(&second);
        assert!(second_encoded.is_ok());
        if let Ok(second_encoded) = second_encoded {
            bytes.extend(second_encoded);
        }
        assert!(std::fs::write(wal.path(), &bytes).is_ok());
        assert!(write_active_ref_metadata(&layout, "heads/main").is_ok());

        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.checked_wal_records, 2);
            assert!(report.has_active_wal_ordering_issue());
            assert_eq!(
                report.active_wal_ordering_issues,
                vec![crate::ActiveWalOrderingIssue {
                    index: 1,
                    previous_seq: 1,
                    seq: 1,
                }]
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn verify_repository_reports_missing_active_metadata_for_non_empty_wal() {
    let root = unique_temp_dir("verify-active-metadata-missing");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let wal = Wal::for_layout(&layout);
        assert!(wal.append_patch(&signed_patch_envelope()).is_ok());

        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(
                report.active_wal_metadata_status,
                ActiveWalMetadataStatus::MissingForNonEmptyWal
            );
            assert!(report.has_active_wal_metadata_integrity_issue());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn verify_repository_reports_malformed_empty_active_metadata_as_warning_state() {
    let root = unique_temp_dir("verify-active-metadata-debris");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(std::fs::write(layout.default_active_ref_name_path(), b"tags/v1").is_ok());

        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert!(matches!(
                report.active_wal_metadata_status,
                ActiveWalMetadataStatus::InvalidForEmptyWal { .. }
            ));
            assert!(report.has_active_wal_metadata_warning());
            assert!(!report.has_active_wal_metadata_integrity_issue());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn verify_repository_detects_object_file_in_wrong_prefix() {
    let root = unique_temp_dir("verify-wrong-prefix");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut store = FileObjectStore::new(layout.clone());
        let envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"payload".to_vec());
        let id = store.write_object(&envelope);
        assert!(id.is_ok());
        if let Ok(id) = id {
            let correct = layout.object_path(ObjectType::Blob, id);
            let wrong_dir = layout.object_type_dir(ObjectType::Blob).join("ff");
            assert!(std::fs::create_dir_all(&wrong_dir).is_ok());
            let wrong = wrong_dir.join(format!("{}.pobj", id.to_hex()));
            assert!(std::fs::rename(correct, wrong).is_ok());
            assert!(verify_repository(&layout).is_err());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

/// DC-95 Stage 1, round 3: `verify_object_file`'s envelope-type-mismatch check
/// (`verify/objects.rs:230-237`) -- a file physically placed under one type's directory whose decoded
/// envelope names a different `ObjectType`. Placed via a raw `std::fs::write` at the type-mismatched
/// directory's own canonical path for the *envelope's real id* (so the file's id and the envelope's
/// computed id agree -- isolating the type mismatch from the id-mismatch check the next test covers),
/// bypassing `store.write_object`, which can never produce this fixture since it always derives the
/// write path from `envelope.object_type` itself. **Probed, load-bearing, confirmed**: disabling the
/// check (commenting out the `if envelope.object_type != object_type` arm) lets `verify_repository`
/// return `Ok` -- nothing downstream re-checks that a directory's contents match its own declared type,
/// unlike the two missing-reference checks round 2 found redundant. **Re-verified against a
/// genuinely clean baseline** (DC-95-stage-1-round-5-review-v1 §2-4): unlike the Block-carrying
/// fixtures elsewhere in this file, this fixture writes only a `Blob` and a `Patch` -- neither type
/// `PublicationTrustVerifier::verify` ever checks (`verify/objects.rs`'s `matches!(object_type,
/// Block | RefState)` gate) -- so no trust policy was ever consulted here and the original probe's
/// `Ok` result was never confounded by `PRIKK-TRUST-POLICY-INVALID` the way the Block-carrying
/// fixtures were. Confirmed by re-probing with the full report printed: every issue vector is
/// empty. Classification unchanged: load-bearing.
#[test]
fn verify_repository_detects_envelope_type_mismatch() -> Result<()> {
    let root = unique_temp_dir("verify-envelope-type-mismatch");
    let layout = RepositoryLayout::init(root.clone())?;

    let blob = BlobPayload::new(BlobKind::Text, b"type-mismatch-fixture\n".to_vec());
    let mut blob_env = ObjectEnvelope::unsigned(ObjectType::Blob, 1, blob.to_canonical_bytes()?);
    blob_env.add_signature(maintainer_signature())?;
    let mut store = FileObjectStore::new(layout.clone());
    let blob_id = store.write_object(&blob_env)?;

    let patch = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::CreateFile(CreateFile {
                path: "type-mismatch-fixture.txt".to_string(),
                node_id: NodeId::from_bytes([0x71; 32]),
                blob_id,
                mode: 0o100_644,
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, patch.to_canonical_bytes()?);
    envelope.add_signature(maintainer_signature())?;
    let patch_id = envelope.object_id();

    // Placed under the Blob directory, at the path that id would canonically occupy there --
    // self-consistent in id, wrong only in which directory holds it.
    let misplaced = layout.object_path(ObjectType::Blob, patch_id);
    std::fs::create_dir_all(
        misplaced
            .parent()
            .ok_or_else(|| PrikkError::Io("misplaced object path has no parent".to_string()))?,
    )?;
    std::fs::write(
        &misplaced,
        crate::file_codec::encode_envelope_file(&envelope)?,
    )?;

    let error = match verify_repository(&layout) {
        Ok(_) => panic!("expected verify_repository to reject a type-mismatched object file"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("is under type") && error.contains("but envelope type is"),
        "expected an envelope-type-mismatch error, got: {error}"
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-95 Stage 1, round 3: `verify_object_file`'s object-id-mismatch check
/// (`verify/objects.rs:239-246`) -- a file's filename-derived id disagreeing with its envelope's own
/// computed content hash. Placed at the canonical path for an *arbitrary* id (not the envelope's real
/// one) within the *correct* type directory, so the type check passes and only the id disagrees.
/// **Probed, load-bearing, confirmed**: disabling the check lets `verify_repository` return `Ok` --
/// content addressing is enforced only by this one explicit comparison at read time, nothing else
/// re-derives a stored file's id independently. **Re-verified against a genuinely clean baseline**
/// (DC-95-stage-1-round-5-review-v1 §2-4), for the same reason the type-mismatch test above is: a
/// lone `Blob`, never checked by `PublicationTrustVerifier`, so the original probe was never
/// confounded by an absent trust policy. Re-probed with the full report printed: every issue vector
/// is empty. Classification unchanged: load-bearing.
#[test]
fn verify_repository_detects_object_id_mismatch() -> Result<()> {
    let root = unique_temp_dir("verify-object-id-mismatch");
    let layout = RepositoryLayout::init(root.clone())?;

    let blob = BlobPayload::new(BlobKind::Text, b"payload".to_vec());
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, blob.to_canonical_bytes()?);
    envelope.add_signature(maintainer_signature())?;

    let wrong_id = sample_object_id("not-this-blob-s-real-id");
    let misplaced = layout.object_path(ObjectType::Blob, wrong_id);
    std::fs::create_dir_all(
        misplaced
            .parent()
            .ok_or_else(|| PrikkError::Io("misplaced object path has no parent".to_string()))?,
    )?;
    std::fs::write(
        &misplaced,
        crate::file_codec::encode_envelope_file(&envelope)?,
    )?;

    let error = match verify_repository(&layout) {
        Ok(_) => panic!("expected verify_repository to reject an id-mismatched object file"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("has id") && error.contains("but computed id is"),
        "expected an object-id-mismatch error, got: {error}"
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-95 Stage 1, round 3: the two "unexpected entry kind" structural checks in `verify_object_type`/
/// `verify_prefix_dir` (`verify/objects.rs`) -- a plain file sitting directly under an object-type
/// directory (where a two-character hex prefix *directory* is expected), and a directory sitting
/// inside a prefix directory (where an object *file* is expected). **Both probed, both
/// downstream-redundant** -- neither disabling arm lets `verify_repository` return `Ok`; each is
/// independently caught one layer further in, with a different, less specific message. Disabling the
/// type-directory check: `list_directory` itself rejects treating the stray file as a directory (a
/// plain filesystem `i/o error: Not a directory (os error 20)`, not an integrity error at all).
/// Disabling the prefix-directory check: `object_id_from_path` (`verify/objects.rs:291`) rejects the
/// stray directory's name for lacking a `.pobj` extension, before the entry is ever read as an object
/// file. Both rows are regression guards on `verify_object_type`'s/`verify_prefix_dir`'s own friendlier,
/// more specific messages -- worth keeping for that diagnostic value -- not demonstrations of Stage 1's
/// silent-absence rule; today's code catches both defects some other way even without these two arms.
#[test]
fn verify_repository_detects_every_directory_shape_violation() -> Result<()> {
    let non_directory_root = unique_temp_dir("verify-non-directory-in-type-dir");
    let layout = RepositoryLayout::init(non_directory_root.clone())?;
    let mut store = FileObjectStore::new(layout.clone());
    store.write_object(&ObjectEnvelope::unsigned(
        ObjectType::Blob,
        1,
        b"payload".to_vec(),
    ))?;
    let stray_file = layout.object_type_dir(ObjectType::Blob).join("zz");
    std::fs::write(&stray_file, b"not a prefix directory")?;
    let error = match verify_repository(&layout) {
        Ok(_) => {
            panic!("expected verify_repository to reject a non-directory in the type directory")
        }
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("unexpected non-directory in object type directory"),
        "expected that specific error, got: {error}"
    );
    let _ = std::fs::remove_dir_all(non_directory_root);

    let non_file_root = unique_temp_dir("verify-non-file-in-prefix-dir");
    let layout = RepositoryLayout::init(non_file_root.clone())?;
    let mut store = FileObjectStore::new(layout.clone());
    let id = store.write_object(&ObjectEnvelope::unsigned(
        ObjectType::Blob,
        1,
        b"payload".to_vec(),
    ))?;
    let prefix_dir = layout
        .object_path(ObjectType::Blob, id)
        .parent()
        .ok_or_else(|| PrikkError::Io("object path has no parent".to_string()))?
        .to_path_buf();
    std::fs::create_dir_all(prefix_dir.join("stray-directory"))?;
    let error = match verify_repository(&layout) {
        Ok(_) => panic!("expected verify_repository to reject a non-file in a prefix directory"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("unexpected non-file in object prefix directory"),
        "expected that specific error, got: {error}"
    );
    let _ = std::fs::remove_dir_all(non_file_root);
    Ok(())
}

/// DC-95 Stage 1, round 3: publication-trust failure proven for a `Block` specifically through
/// `verify_repository`, not only for a `Blob` at the unit level (`verify/tests/trust.rs`, which calls
/// `PublicationTrustVerifier` directly). Every existing Block-fixture test in this file signs with
/// `test_support::maintainer_signature()` -- a fixed, non-cryptographic placeholder signature (`key_id
/// "maintainer-key"`, `signature_bytes: vec![5; 64]`) -- and, until this test, none of them ever
/// established a trust policy at all. **That distinction matters and was learned by getting it wrong
/// first**: an absent trust policy produces `PRIKK-TRUST-POLICY-INVALID` (`verify/trust.rs:39-50`), not
/// `PRIKK-TRUST-PUBLICATION-UNTRUSTED` -- confirmed by an initial version of this test asserting the
/// wrong code and failing. The fixture below establishes a *valid* policy (via `add_trusted_maintainer`,
/// trusting a different, genuinely keyed signer) before writing the untrusted block, so the untrusted
/// signer's own key is checked against a real policy that legitimately doesn't name it, not against a
/// missing one. The trusted contrast then uses *genuinely, cryptographically* signed material --
/// `crate::Ed25519MaintainerSigner` and the real, argument-taking `crate::maintainer_signature`
/// (distinct from this file's already-imported `test_support::maintainer_signature`; the placeholder's
/// fixed bytes are not a real signature under any keypair, so trusting its literal key id would not make
/// it verify). **Not independently probed by disabling production code** -- `publication_trust_issues`
/// is accumulated, never a hard `Err` (confirmed in `verify/tests/trust.rs`'s own tests), so "disable the
/// check" would need to suppress the whole `PublicationTrustVerifier::verify` call rather than one arm;
/// the untrusted-vs-trusted contrast within this one test, checked in a single `verify_repository` call,
/// is the isolation instead.
#[test]
fn verify_repository_flags_untrusted_block_signer_and_clears_once_trusted() -> Result<()> {
    let root = unique_temp_dir("verify-untrusted-block-signer");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut store = FileObjectStore::new(layout.clone());

    // Establish a real, valid trust policy first, naming only `trusted_signer` -- so the untrusted
    // block below is checked against a policy that legitimately excludes it, not a missing one.
    let trusted_signer =
        crate::Ed25519MaintainerSigner::from_seed("verify-trust-maintainer", &[0x63; 32])?;
    let trusted_public_key_hex: String = trusted_signer
        .public_key_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    crate::add_trusted_maintainer(&layout, trusted_signer.key_id(), &trusted_public_key_hex)?;

    let untrusted_payload = BlockPayload {
        parent_block_ids: Vec::new(),
        kind: BlockKind::Root,
        patch_ids: Vec::new(),
        state_merkle_root: derive_next_state_root(&store, None, &[])?,
        snapshot_blob_ref: None,
        mainline_parent_id: None,
        merge_baseline_block_id: None,
    };
    let untrusted_block_id = write_signed_block(&mut store, &untrusted_payload)?;

    // A second, independent Root block (no relationship to the first needed -- verify_objects scans
    // every persisted object regardless of any ref pointing to it, matching every other block-only
    // fixture in this file, none of which create a ref/pointer either), signed for real by the
    // already-trusted key. It names a real, existing snapshot blob -- distinguishing its payload bytes
    // (and so its content-addressed id) from the untrusted block above, which is otherwise identical --
    // without tripping the unrelated missing-snapshot-blob check.
    let snapshot_blob = BlobPayload::new(BlobKind::Text, b"trusted-block-snapshot".to_vec());
    let mut snapshot_envelope =
        ObjectEnvelope::unsigned(ObjectType::Blob, 1, snapshot_blob.to_canonical_bytes()?);
    snapshot_envelope.add_signature(maintainer_signature())?;
    let snapshot_blob_id = snapshot_envelope.object_id();
    store.write_object(&snapshot_envelope)?;

    let trusted_payload = BlockPayload {
        parent_block_ids: Vec::new(),
        kind: BlockKind::Root,
        patch_ids: Vec::new(),
        state_merkle_root: derive_next_state_root(&store, None, &[])?,
        snapshot_blob_ref: Some(snapshot_blob_id),
        mainline_parent_id: None,
        merge_baseline_block_id: None,
    };
    let trusted_payload_bytes = trusted_payload.to_canonical_bytes()?;
    let mut trusted_envelope =
        ObjectEnvelope::unsigned(ObjectType::Block, 2, trusted_payload_bytes);
    let trusted_id = trusted_envelope.object_id();
    trusted_envelope.add_signature(crate::maintainer_signature(
        &trusted_signer,
        ObjectType::Block,
        trusted_id,
    )?)?;
    store.write_object(&trusted_envelope)?;

    let report = verify_repository(&layout)?;
    assert!(report.has_publication_trust_issues());
    assert!(report.publication_trust_issues.iter().any(|issue| {
        issue.code == "PRIKK-TRUST-PUBLICATION-UNTRUSTED"
            && issue.message.contains(&untrusted_block_id.to_string())
    }));
    assert!(
        !report
            .publication_trust_issues
            .iter()
            .any(|issue| issue.message.contains(&trusted_id.to_string())),
        "the trusted block must carry no publication-trust issue of its own: {:?}",
        report.publication_trust_issues
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-95 Stage 1, round 4: the `RefState` half of the publication-trust check, mirroring round 3's
/// `Block` test above. `verify_object_file` (`verify/objects.rs:255`) routes `Block` and `RefState`
/// through `PublicationTrustVerifier::verify` identically (`matches!(object_type, ObjectType::Block |
/// ObjectType::RefState)`); nothing else about the check's own arm needs re-probing, but this codebase's
/// DC-95 bar is per-object-type end-to-end proof, not "the same `matches!` arm should behave the same".
/// A `RefState` object is written raw via `store.write_object`, orphaned (no ref pointer created,
/// matching the Block test's own precedent): `verify_refs`/`verify_ref_publication` only resolve
/// `RefState` objects reachable through an actual ref pointer, never by scanning the `ref-state`
/// object-type directory, so an orphan is invisible to every check except this one and the general
/// object-count scan. Two distinct `ref_name`s keep the two payloads' canonical bytes apart, unlike the
/// Block test, which needed a distinguishing snapshot-blob reference since an empty Root block has no
/// other field to vary.
#[test]
fn verify_repository_flags_untrusted_ref_state_signer_and_clears_once_trusted() -> Result<()> {
    let root = unique_temp_dir("verify-untrusted-refstate-signer");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut store = FileObjectStore::new(layout.clone());

    let trusted_signer =
        crate::Ed25519MaintainerSigner::from_seed("verify-trust-maintainer-rs", &[0x64; 32])?;
    let trusted_public_key_hex: String = trusted_signer
        .public_key_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    crate::add_trusted_maintainer(&layout, trusted_signer.key_id(), &trusted_public_key_hex)?;

    let untrusted_envelope = signed_ref_state_envelope(
        "heads/untrusted",
        None,
        sample_object_id("untrusted-target"),
        1,
    );
    let untrusted_id = store.write_object(&untrusted_envelope)?;

    let trusted_payload = prikk_object::RefStatePayload {
        ref_name: "heads/trusted".to_string(),
        kind: prikk_object::RefKind::Branch,
        target_object_id: sample_object_id("trusted-target"),
        update_seq: 1,
        previous_ref_state_id: None,
        required_attestation_ids: Vec::new(),
        closed: false,
    };
    let trusted_bytes = trusted_payload.to_canonical_bytes()?;
    let mut trusted_envelope = ObjectEnvelope::unsigned(ObjectType::RefState, 1, trusted_bytes);
    let trusted_id = trusted_envelope.object_id();
    trusted_envelope.add_signature(crate::maintainer_signature(
        &trusted_signer,
        ObjectType::RefState,
        trusted_id,
    )?)?;
    store.write_object(&trusted_envelope)?;

    let report = verify_repository(&layout)?;
    assert!(report.has_publication_trust_issues());
    assert!(report.publication_trust_issues.iter().any(|issue| {
        issue.code == "PRIKK-TRUST-PUBLICATION-UNTRUSTED"
            && issue.message.contains(&untrusted_id.to_string())
    }));
    assert!(
        !report
            .publication_trust_issues
            .iter()
            .any(|issue| issue.message.contains(&trusted_id.to_string())),
        "the trusted ref-state must carry no publication-trust issue of its own: {:?}",
        report.publication_trust_issues
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// DC-95 Stage 1, round 4: `format::validate_read_schema`'s strict-signature-shape branch
/// (`ObjectEnvelope::validate_strict`, `prikk-object/src/envelope.rs:98-117`) -- an Ed25519 signature
/// whose `signature_bytes` is not exactly 64 bytes. Previously only "Partial" coverage: `signature_
/// contract_tests/read_admission.rs`'s `format2_object_reads_reject_every_strict_envelope_failure`
/// already runs this through `verify_repository` end to end, but only asserts `.is_err()` over three
/// bundled variants (malformed shape, duplicate signature, non-canonical order) without pinning the
/// rejection to this specific one's message -- so a regression that swapped which variant fired first,
/// or which of the three rejected at all, could pass silently. This isolates the shape variant alone
/// with a specific-message assertion, matching this file's own bar for every other round.
///
/// **Must bypass both `write_object` and `verify/tests.rs`'s own established `encode_envelope_file`
/// helper** (used by every other raw-placement test in this file, e.g. the type/id-mismatch tests
/// above): both enforce `validate_strict()` at encode time and would reject a malformed-shape signature
/// before any bytes could be written. Only the `#[cfg(test)]`-only `encode_envelope_file_structural`
/// (which validates shape loosely, not strictly) permits constructing this fixture at all -- confirming
/// this rule is enforced exactly once, at read time in `verify_object_file`, with production write paths
/// closed to it entirely.
///
/// **Probed, load-bearing, confirmed -- but not the way the type/id-mismatch checks were.** Disabling
/// `validate_strict`'s `malformed_shape` arm does not produce a clean `Ok` with zero issues: the
/// downstream, independent `classify_signature_envelope` (`verify/objects.rs`) still records the same
/// defect, as a `SignatureEnvelopeIssue` with code `PRIKK-VERIFY-SIGNATURE-MALFORMED`, in `report.
/// signature_envelope_issues`. The reason this still counts as load-bearing under Stage 1's rule: unlike
/// `publication_trust_issues` or `merge_baseline_divergences`, `signature_envelope_issues` backs none of
/// `RepositoryVerification`'s eight `has_*` blocking predicates (`verify.rs:153-212`) -- the exact set
/// `run_verify`'s priority chain (`prikk-cli/src/main.rs:530-544`) reads to decide pass/fail. So a
/// malformed-shape signature caught only by the downstream classifier, with this hard check removed,
/// would report as an informational note while `prikk verify` still exits clean: precisely the "silent
/// absence lets a repository verify clean" scenario the rule is about, just realized through a
/// non-blocking sibling finding rather than through total silence.
///
/// **Re-verified against a genuinely clean baseline** (DC-95-stage-1-round-5-review-v1 §2-4): this
/// fixture, like the type/id-mismatch tests, writes only a `Blob`, never checked by `Publication
/// TrustVerifier` -- so the original probe's report (`publication_trust_issues: []`, `checked_
/// publication_trust_records: 0`, already printed at the time this classification was first made)
/// was never confounded by an absent trust policy. Re-confirmed by re-running the probe: unchanged.
#[test]
fn verify_repository_rejects_malformed_signature_shape() -> Result<()> {
    let root = unique_temp_dir("verify-malformed-signature-shape");
    let layout = RepositoryLayout::init(root.clone())?;

    let blob = BlobPayload::new(BlobKind::Text, b"strict-shape fixture\n".to_vec());
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Blob, 1, blob.to_canonical_bytes()?);
    // Bypasses add_signature's own shape gate by setting the field directly -- Ed25519 requires
    // exactly 64 bytes; this is 63.
    envelope.signatures = vec![prikk_object::Signature {
        algorithm: prikk_object::SignatureAlgorithm::Ed25519,
        key_id: "maintainer-key".to_string(),
        signature_bytes: vec![5_u8; 63],
        created_at: 8,
        signer_role: prikk_object::SignerRole::Maintainer,
    }];

    let object_id = envelope.object_id();
    let path = layout.object_path(envelope.object_type, object_id);
    std::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| PrikkError::Io("test object path has no parent".to_string()))?,
    )?;
    std::fs::write(
        &path,
        crate::file_codec::encode_envelope_file_structural(&envelope)?,
    )?;

    let error = match verify_repository(&layout) {
        Ok(_) => panic!("expected verify_repository to reject a malformed-shape signature"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("malformed algorithm shape"),
        "expected the strict-signature-shape rejection, got: {error}"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
