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

#![allow(clippy::expect_used)]

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

fn create_file_strategy() -> impl Strategy<Value = CreateFile> {
    (
        repo_path_strategy(),
        node_id_strategy(),
        object_id_strategy(),
        any::<u32>(),
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
            any::<u32>()
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
            any::<u32>()
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
    (node_id_strategy(), any::<u32>(), any::<u32>()).prop_map(|(node_id, old_mode, new_mode)| {
        ChangePerm {
            node_id,
            old_mode,
            new_mode,
        }
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
