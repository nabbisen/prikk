//! Repository verification tests.

mod root_authority;
mod trust;

use prikk_error::Result;
use prikk_object::{
    BlobKind, BlobPayload, BlockKind, BlockPayload, CanonicalEncode, CreateFile, MerkleRoot,
    NodeId, ObjectEnvelope, ObjectId, ObjectType, Operation, OperationKind, PatchPayload,
    PatchPurpose,
};

use crate::wal::{WalRecord, encode_record_for_test};
use crate::{
    ActiveWalMetadataStatus, FileObjectStore, ObjectWriter, RepositoryLayout, Wal,
    derive_next_state_root, verify_repository, write_active_ref_metadata,
};

use crate::test_support::{
    dummy_signature, maintainer_signature, rollback_patch_envelope, sample_object_id,
    signed_patch_envelope, unique_temp_dir,
};

#[test]
fn verify_repository_detects_block_with_missing_patch() {
    let root = unique_temp_dir("block-missing-patch");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut store = FileObjectStore::new(layout.clone());
        let missing_patch = sample_object_id("missing-patch");
        let payload = BlockPayload {
            parent_block_ids: Vec::new(),
            kind: BlockKind::Root,
            patch_ids: vec![missing_patch],
            state_merkle_root: MerkleRoot([0_u8; 32]),
            snapshot_blob_ref: None,
            mainline_parent_id: None,
            merge_baseline_block_id: None,
        };
        let payload_bytes = payload.to_canonical_bytes();
        assert!(payload_bytes.is_ok());
        if let Ok(payload_bytes) = payload_bytes {
            let mut block = ObjectEnvelope::unsigned(ObjectType::Block, 2, payload_bytes);
            assert!(block.add_signature(maintainer_signature()).is_ok());
            assert!(store.write_object(&block).is_ok());
            assert!(verify_repository(&layout).is_err());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

/// DC-92 implementation review §4: an end-to-end control that `verify` actually performs block state
/// verification through the real `verify_objects` wiring — Phase A's collection into
/// `pending_v2_blocks`, then Phase B's `verify_blocks_topological` — not merely in the unit-level
/// `verify_block_v2_state`/`verify_blocks_topological` calls `block_state::tests` exercises directly
/// against a `MemoryObjectStore`. The review found that removing the inline state-check call entirely
/// (on `main`, pre-DC-92, and again after DC-92's restructuring) left the whole workspace suite
/// green — nothing wired the two together. Built, not byte-corrupted, matching this module's own
/// `verify_repository_detects_block_with_missing_patch`: content addressing means a post-hoc-
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
#[test]
fn verify_repository_detects_every_block_shape_violation() -> Result<()> {
    type CaseFn = fn(ObjectId, ObjectId, ObjectId) -> (BlockPayload, &'static str);
    let cases: Vec<(&str, CaseFn)> = vec![
        ("root-with-parent", |genesis, _a, _b| {
            (
                BlockPayload {
                    parent_block_ids: vec![genesis],
                    kind: BlockKind::Root,
                    patch_ids: Vec::new(),
                    state_merkle_root: MerkleRoot([0xAA_u8; 32]),
                    snapshot_blob_ref: None,
                    mainline_parent_id: None,
                    merge_baseline_block_id: None,
                },
                "Root Block must have zero parents",
            )
        }),
        ("normal-with-zero-parents", |_genesis, _a, _b| {
            (
                BlockPayload {
                    parent_block_ids: Vec::new(),
                    kind: BlockKind::Normal,
                    patch_ids: Vec::new(),
                    state_merkle_root: MerkleRoot([0xAB_u8; 32]),
                    snapshot_blob_ref: None,
                    mainline_parent_id: None,
                    merge_baseline_block_id: None,
                },
                "Normal Block must have exactly one parent",
            )
        }),
        ("merge-with-one-parent", |genesis, _a, _b| {
            (
                BlockPayload {
                    parent_block_ids: vec![genesis],
                    kind: BlockKind::Merge,
                    patch_ids: Vec::new(),
                    state_merkle_root: MerkleRoot([0xAC_u8; 32]),
                    snapshot_blob_ref: None,
                    mainline_parent_id: Some(genesis),
                    merge_baseline_block_id: Some(genesis),
                },
                "Merge Block must have exactly two parents",
            )
        }),
        ("repair-kind-unauthorized", |_genesis, _a, _b| {
            (
                BlockPayload {
                    parent_block_ids: Vec::new(),
                    kind: BlockKind::Repair,
                    patch_ids: Vec::new(),
                    state_merkle_root: MerkleRoot([0xAD_u8; 32]),
                    snapshot_blob_ref: None,
                    mainline_parent_id: None,
                    merge_baseline_block_id: None,
                },
                "Block kind is not authorized",
            )
        }),
        ("root-with-mainline-field", |genesis, _a, _b| {
            (
                BlockPayload {
                    parent_block_ids: Vec::new(),
                    kind: BlockKind::Root,
                    patch_ids: Vec::new(),
                    state_merkle_root: MerkleRoot([0xAE_u8; 32]),
                    snapshot_blob_ref: None,
                    mainline_parent_id: Some(genesis),
                    merge_baseline_block_id: None,
                },
                "must not carry a mainline parent or merge baseline",
            )
        }),
        ("merge-without-mainline", |genesis, a, b| {
            let mut parents = vec![a, b];
            parents.sort();
            (
                BlockPayload {
                    parent_block_ids: parents,
                    kind: BlockKind::Merge,
                    patch_ids: Vec::new(),
                    state_merkle_root: MerkleRoot([0xAF_u8; 32]),
                    snapshot_blob_ref: None,
                    mainline_parent_id: None,
                    merge_baseline_block_id: Some(genesis),
                },
                "Merge Block must name a mainline parent",
            )
        }),
        ("merge-mainline-not-a-parent", |genesis, a, b| {
            let mut parents = vec![a, b];
            parents.sort();
            (
                BlockPayload {
                    parent_block_ids: parents,
                    kind: BlockKind::Merge,
                    patch_ids: Vec::new(),
                    state_merkle_root: MerkleRoot([0xB0_u8; 32]),
                    snapshot_blob_ref: None,
                    mainline_parent_id: Some(genesis),
                    merge_baseline_block_id: Some(genesis),
                },
                "mainline parent must be one of its own parents",
            )
        }),
        ("merge-without-baseline", |_genesis, a, b| {
            let mut parents = vec![a, b];
            parents.sort();
            (
                BlockPayload {
                    parent_block_ids: parents,
                    kind: BlockKind::Merge,
                    patch_ids: Vec::new(),
                    state_merkle_root: MerkleRoot([0xB1_u8; 32]),
                    snapshot_blob_ref: None,
                    mainline_parent_id: Some(a),
                    merge_baseline_block_id: None,
                },
                "must record the baseline confluence was proven against",
            )
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

        let (payload, expected_substring) = case_fn(genesis, parent_a, parent_b);
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
