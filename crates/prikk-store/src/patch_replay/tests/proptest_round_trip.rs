//! DC-41 stage 4 property tests: patch operation decoding (target 5 of 5).
//!
//! Round-trip with bounded generation across all seven FDD-03 §9.3 operation kinds, plus
//! totality for arbitrary bytes. This is the real, comprehensive Patch-content decoder
//! (`decode_patch_operations`) — see `payload::tests::proptest_decoders`'s module doc for why
//! `prikk-object`'s `PatchPurpose::decode_from_patch_payload` is not itself a full round-trip
//! decoder and is not target 2's subject.
//!
//! **Generation bounds are test-tractability limits, not production thresholds.** Op count
//! (1..=5), path depth (1..=3 segments), path segment length (1..=8 chars), and content size
//! (0..=32 chars) exist to keep property-test cases fast to generate and shrink. They are
//! unrelated to the production `NFR-PERF-02` active-block thresholds (800/1000 patches), which
//! govern active-session size, not test-input shape.
//!
//! Case budget is proptest's own default (256/run), overridable with `PROPTEST_CASES` for a
//! campaign run.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use proptest::prelude::*;

use prikk_object::{
    CanonicalEncode, ChangePerm, CreateFile, CreateSymlink, DeleteNode, DeleteNodePreimage,
    EditText, NodeId, NodeKind, ObjectId, Operation, OperationKind, PatchPayload, PatchPurpose,
    RenamePath, ReplaceBinary, text_span_hash,
};

use crate::patch_replay::decode::{
    DecodedDeletePreimage, DecodedOperationKind, DecodedPatchOperation, decode_patch_operations,
};

fn node_id_strategy() -> impl Strategy<Value = NodeId> {
    // Every byte drawn from 1..=255 so the 32-byte value can never be the reserved all-zero id,
    // without needing a reject/retry loop.
    proptest::array::uniform32(1_u8..=255).prop_map(NodeId::from_bytes)
}

fn object_id_strategy() -> impl Strategy<Value = ObjectId> {
    proptest::array::uniform32(any::<u8>()).prop_map(ObjectId::from_bytes)
}

/// **DC-54 fixed the finding this exclusion used to guard against — filter retired.**
/// (Formerly excluded Windows-reserved device names here, because encode did not validate paths
/// and a generated `"com1"` would encode successfully but fail to decode. DC-54 added
/// `RepoPath`-equivalent validation to the write-side `validate()` of every path-carrying
/// operation kind, so encode now rejects exactly what decode rejects — see
/// `patch_operations_round_trip` below for how the property handles that legitimate rejection.
/// The original minimized reproducer remains committed at
/// `proptest-regressions/patch_replay/tests/proptest_round_trip.txt` as a permanent regression
/// guard: proptest replays it first on every run, and it now must fail at **encode**, not decode.)
fn path_segment_strategy() -> impl Strategy<Value = String> {
    "[a-z0-9]{1,8}"
}

fn repo_path_strategy() -> impl Strategy<Value = String> {
    proptest::collection::vec(path_segment_strategy(), 1..=3)
        .prop_map(|segments| segments.join("/"))
}

fn ascii_text_strategy() -> impl Strategy<Value = String> {
    "[a-z]{0,32}"
}

/// RFC 125 §2: only two `u32` values are canonical file modes, so `any::<u32>()` here would make
/// `patch_operations_round_trip` skip on essentially every generated `CreateFile`/`ChangePerm`/
/// file-`DeleteNode` case (2 valid values out of 2^32) rather than the occasional, tolerable skip
/// rate `repo_path_strategy()`'s Windows-reserved-name rejection has. Narrowed here so the property
/// keeps exercising real round-trips for these kinds; `non_canonical_mode_strategy` below covers the
/// refusal itself.
fn canonical_mode_strategy() -> impl Strategy<Value = u32> {
    prop_oneof![Just(0o100_644_u32), Just(0o100_755_u32)]
}

/// The complement of [`canonical_mode_strategy`], for exercising the new refusal itself (RFC 125
/// §2/§9's controls).
fn non_canonical_mode_strategy() -> impl Strategy<Value = u32> {
    any::<u32>().prop_filter("must not be one of the two canonical file modes", |mode| {
        !matches!(*mode, 0o100_644 | 0o100_755)
    })
}

fn create_file_strategy() -> impl Strategy<Value = CreateFile> {
    (
        repo_path_strategy(),
        node_id_strategy(),
        object_id_strategy(),
        canonical_mode_strategy(),
    )
        .prop_map(|(path, node_id, blob_id, mode)| CreateFile {
            path,
            node_id,
            blob_id,
            mode,
        })
}

fn delete_node_strategy() -> impl Strategy<Value = DeleteNode> {
    prop_oneof![
        (
            repo_path_strategy(),
            node_id_strategy(),
            object_id_strategy(),
            canonical_mode_strategy()
        )
            .prop_map(|(path, node_id, old_blob_id, old_mode)| DeleteNode {
                path,
                node_id,
                old_node_kind: NodeKind::TextFile,
                preimage: DeleteNodePreimage::File {
                    old_blob_id,
                    old_mode
                },
            }),
        (
            repo_path_strategy(),
            node_id_strategy(),
            object_id_strategy(),
            canonical_mode_strategy()
        )
            .prop_map(|(path, node_id, old_blob_id, old_mode)| DeleteNode {
                path,
                node_id,
                old_node_kind: NodeKind::BinaryFile,
                preimage: DeleteNodePreimage::File {
                    old_blob_id,
                    old_mode
                },
            }),
        (
            repo_path_strategy(),
            node_id_strategy(),
            ascii_text_strategy()
        )
            .prop_map(|(path, node_id, old_target)| DeleteNode {
                path,
                node_id,
                old_node_kind: NodeKind::Symlink,
                preimage: DeleteNodePreimage::Symlink { old_target },
            }),
    ]
}

fn edit_text_strategy() -> impl Strategy<Value = EditText> {
    (
        node_id_strategy(),
        proptest::array::uniform32(any::<u8>()),
        proptest::array::uniform32(any::<u8>()),
        proptest::array::uniform32(any::<u8>()),
        ascii_text_strategy(),
        ascii_text_strategy(),
    )
        .prop_map(
            |(
                node_id,
                span_id,
                left_anchor_hash,
                right_anchor_hash,
                replacement_text,
                old_span_text,
            )| {
                let old_span_text = old_span_text.into_bytes();
                EditText {
                    node_id,
                    span_id,
                    old_span_hash: text_span_hash(&old_span_text),
                    left_anchor_hash,
                    right_anchor_hash,
                    replacement_text: replacement_text.into_bytes(),
                    presentation_hint_line: None,
                    presentation_hint_column: None,
                    old_span_text,
                }
            },
        )
}

fn rename_path_strategy() -> impl Strategy<Value = RenamePath> {
    (
        node_id_strategy(),
        repo_path_strategy(),
        repo_path_strategy(),
    )
        .prop_map(|(node_id, old_path, new_path)| RenamePath {
            node_id,
            old_path,
            new_path,
        })
}

fn change_perm_strategy() -> impl Strategy<Value = ChangePerm> {
    (
        node_id_strategy(),
        canonical_mode_strategy(),
        canonical_mode_strategy(),
    )
        .prop_map(|(node_id, old_mode, new_mode)| ChangePerm {
            node_id,
            old_mode,
            new_mode,
        })
}

fn create_symlink_strategy() -> impl Strategy<Value = CreateSymlink> {
    (
        repo_path_strategy(),
        node_id_strategy(),
        ascii_text_strategy(),
    )
        .prop_map(|(path, node_id, target)| CreateSymlink {
            path,
            node_id,
            target,
        })
}

fn replace_binary_strategy() -> impl Strategy<Value = ReplaceBinary> {
    (
        node_id_strategy(),
        object_id_strategy(),
        object_id_strategy(),
    )
        .prop_map(|(node_id, old_blob_id, new_blob_id)| ReplaceBinary {
            node_id,
            old_blob_id,
            new_blob_id,
        })
}

fn operation_kind_strategy() -> impl Strategy<Value = OperationKind> {
    prop_oneof![
        create_file_strategy().prop_map(OperationKind::CreateFile),
        delete_node_strategy().prop_map(OperationKind::DeleteNode),
        edit_text_strategy().prop_map(OperationKind::EditText),
        rename_path_strategy().prop_map(OperationKind::RenamePath),
        change_perm_strategy().prop_map(OperationKind::ChangePerm),
        create_symlink_strategy().prop_map(OperationKind::CreateSymlink),
        replace_binary_strategy().prop_map(OperationKind::ReplaceBinary),
    ]
}

/// `op_id` and `preconditions` are generated as empty/`None` always: `decode_operation` reads and
/// discards both (tags 2 and 3 are matched to `{}`), so they carry no round-trip signal for this
/// decoder and including them would only add generation cost without strengthening the property.
fn operations_strategy() -> impl Strategy<Value = Vec<Operation>> {
    proptest::collection::vec(operation_kind_strategy(), 1..=5).prop_map(|kinds| {
        kinds
            .into_iter()
            .enumerate()
            .map(|(index, kind)| Operation {
                op_seq: (index as u32) + 1,
                op_id: None,
                preconditions: Vec::new(),
                kind,
            })
            .collect()
    })
}

fn expected_decoded(op: &Operation) -> DecodedPatchOperation {
    let kind = match &op.kind {
        OperationKind::CreateFile(value) => DecodedOperationKind::CreateFile {
            path: value.path.clone(),
            node_id: value.node_id,
            blob_id: value.blob_id,
            mode: value.mode,
        },
        OperationKind::DeleteNode(value) => DecodedOperationKind::DeleteNode {
            path: value.path.clone(),
            node_id: value.node_id,
            preimage: match &value.preimage {
                DeleteNodePreimage::File {
                    old_blob_id,
                    old_mode,
                } => DecodedDeletePreimage::File {
                    old_node_kind: value.old_node_kind,
                    old_blob_id: *old_blob_id,
                    old_mode: *old_mode,
                },
                DeleteNodePreimage::Symlink { old_target } => DecodedDeletePreimage::Symlink {
                    old_target: old_target.clone(),
                },
            },
        },
        OperationKind::EditText(value) => DecodedOperationKind::EditText {
            node_id: value.node_id,
            span_id: value.span_id,
            old_span_hash: value.old_span_hash,
            left_anchor_hash: value.left_anchor_hash,
            right_anchor_hash: value.right_anchor_hash,
            replacement_text: value.replacement_text.clone(),
            old_span_text: value.old_span_text.clone(),
        },
        OperationKind::RenamePath(value) => DecodedOperationKind::RenamePath {
            node_id: value.node_id,
            old_path: value.old_path.clone(),
            new_path: value.new_path.clone(),
        },
        OperationKind::ChangePerm(value) => DecodedOperationKind::ChangePerm {
            node_id: value.node_id,
            old_mode: value.old_mode,
            new_mode: value.new_mode,
        },
        OperationKind::CreateSymlink(value) => DecodedOperationKind::CreateSymlink {
            path: value.path.clone(),
            node_id: value.node_id,
            target: value.target.clone(),
        },
        OperationKind::ReplaceBinary(value) => DecodedOperationKind::ReplaceBinary {
            node_id: value.node_id,
            old_blob_id: value.old_blob_id,
            new_blob_id: value.new_blob_id,
        },
    };
    DecodedPatchOperation {
        op_seq: op.op_seq,
        kind,
    }
}

/// Find a canonical TLV field matching `target_tag` in `bytes` and return its value's byte range
/// (not the whole field). Each canonical field is `[2-byte tag BE][1-byte wire type]
/// [8-byte length BE][value]` (`CanonicalWriter`'s own format) -- shared with
/// `payload::tests::proptest_decoders`'s own copy of this scan (kept separate per crate rather than
/// a shared test-only dependency for two call sites).
fn find_field_value_range(bytes: &[u8], target_tag: u16) -> Option<(usize, usize)> {
    let mut pos = 0;
    while pos < bytes.len() {
        let header_end = pos.checked_add(11)?;
        let tag = u16::from_be_bytes(bytes.get(pos..pos + 2)?.try_into().ok()?);
        let len = u64::from_be_bytes(bytes.get(pos + 3..header_end)?.try_into().ok()?);
        let len = usize::try_from(len).ok()?;
        let field_end = header_end.checked_add(len)?;
        if tag == target_tag {
            return Some((header_end, field_end));
        }
        pos = field_end;
    }
    None
}

/// Overwrite a 4-byte `u32` field's value, found by following `path` through successively nested
/// canonical records (e.g. `[1, 10, 4]`: `PatchPayload` tag 1's `Operation` record, its tag 10
/// `CreateFile` sub-record, that sub-record's tag 4 `mode`). Used to hand-craft bytes a validating
/// encoder can no longer produce (RFC 125 §2 closed that path at `CreateFile::validate` and
/// friends), so the decoder's own independent refusal can still be exercised directly.
fn overwrite_u32_at_path(bytes: &[u8], path: &[u16], new_value: u32) -> Option<Vec<u8>> {
    let mut base = 0usize;
    let mut current: &[u8] = bytes;
    for (index, &tag) in path.iter().enumerate() {
        let (value_start, value_end) = find_field_value_range(current, tag)?;
        if index + 1 == path.len() {
            if value_end - value_start != 4 {
                return None;
            }
            let absolute_start = base + value_start;
            let absolute_end = base + value_end;
            let mut out = bytes.to_vec();
            out[absolute_start..absolute_end].copy_from_slice(&new_value.to_be_bytes());
            return Some(out);
        }
        base += value_start;
        current = current.get(value_start..value_end)?;
    }
    None
}

/// RFC 125 §2, control 1: a crafted input the decoder accepted at base and refuses on this commit.
/// Encoding a non-canonical mode is refused before this point ever exists in real bytes
/// (`CreateFile::validate`), so this hand-crafts what the encoder can no longer produce, to prove
/// the decoder holds the same line independently rather than relying only on encode's refusal.
#[test]
fn decode_patch_operations_rejects_a_hand_crafted_non_canonical_create_file_mode() {
    let payload = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::CreateFile(CreateFile {
                path: "a.txt".to_string(),
                node_id: NodeId::from_bytes([1u8; 32]),
                blob_id: ObjectId::from_bytes([2u8; 32]),
                mode: 0o100_644,
            }),
        }],
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let bytes = payload
        .to_canonical_bytes()
        .expect("a canonical mode encodes");
    // PatchPayload tag 1 (this Operation record) -> Operation tag 10 (CreateFile) -> CreateFile
    // tag 4 (mode).
    let mutated = overwrite_u32_at_path(&bytes, &[1, 10, 4], 0o100_000)
        .expect("mode field present at the expected path");
    let err = decode_patch_operations(&mutated, prikk_object::PATCH_PARENT_IDS_RETIRED_SCHEMA)
        .expect_err("a hand-crafted non-canonical mode must be refused at decode");
    assert!(format!("{err}").contains("canonical"), "{err}");
}

proptest! {
    /// RFC 125 §2/§9 control 1, broadened: an out-of-canonical-set mode is refused **at encode**,
    /// for every operation kind that carries one, across whatever other field values the strategies
    /// generate -- not only the one hand-crafted example above.
    #[test]
    fn create_file_with_a_non_canonical_mode_is_refused_at_encode(
        path in repo_path_strategy(),
        node_id in node_id_strategy(),
        blob_id in object_id_strategy(),
        mode in non_canonical_mode_strategy(),
    ) {
        let operation = CreateFile { path, node_id, blob_id, mode };
        prop_assert!(operation.to_canonical_bytes().is_err());
    }

    #[test]
    fn change_perm_with_a_non_canonical_mode_is_refused_at_encode(
        node_id in node_id_strategy(),
        old_mode in non_canonical_mode_strategy(),
        new_mode in canonical_mode_strategy(),
    ) {
        let operation = ChangePerm { node_id, old_mode, new_mode };
        prop_assert!(operation.to_canonical_bytes().is_err());
    }

    #[test]
    fn delete_node_file_kind_with_a_non_canonical_old_mode_is_refused_at_encode(
        path in repo_path_strategy(),
        node_id in node_id_strategy(),
        old_blob_id in object_id_strategy(),
        old_mode in non_canonical_mode_strategy(),
    ) {
        let operation = DeleteNode {
            path,
            node_id,
            old_node_kind: NodeKind::TextFile,
            preimage: DeleteNodePreimage::File { old_blob_id, old_mode },
        };
        prop_assert!(operation.to_canonical_bytes().is_err());
    }
}

proptest! {
    #[test]
    fn patch_operations_round_trip(operations in operations_strategy()) {
        let payload = PatchPayload {
            operations: operations.clone(),
            intent: None,
            preconditions: Vec::new(),
            purpose: PatchPurpose::Normal,
        };
        // DC-54: encode now legitimately rejects an operation whose path violates the RepoPath
        // grammar (reserved names, traversal, absolute, `.prikk`-prefixed) — generation is
        // unrestricted (path_segment_strategy() can still produce e.g. "com1"), so that is an
        // expected outcome, not a round-trip failure. There is nothing to decode in that case;
        // skip it rather than assert round-trip on a value that was correctly never produced.
        let Ok(bytes) = payload.to_canonical_bytes() else {
            return Ok(());
        };
        // Every construction site now writes `PATCH_PARENT_IDS_RETIRED_SCHEMA` (Patch schema 2
        // handoff) -- matching that here, not schema 1, since this proves round-trip fidelity for
        // what the encoder actually produces today.
        let decoded = decode_patch_operations(&bytes, prikk_object::PATCH_PARENT_IDS_RETIRED_SCHEMA)
            .expect("bytes produced by the encoder must always decode");
        let expected: Vec<DecodedPatchOperation> = operations.iter().map(expected_decoded).collect();
        prop_assert_eq!(decoded, expected);
    }

    #[test]
    fn decode_patch_operations_never_panics_on_arbitrary_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..512)
    ) {
        let _ = decode_patch_operations(&bytes, 1);
    }
}
