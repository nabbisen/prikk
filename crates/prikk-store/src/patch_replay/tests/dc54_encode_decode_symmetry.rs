//! DC-54 - the symmetry proof: encode's rejection and decode's rejection are the same error.
//!
//! `RenamePath::validate()` (encode, `prikk-object`) and `decode_rename_path` (decode,
//! `prikk-store`, via `RepoPath::parse` in `prikk-replay`) now both bottom out in the same
//! `validate_repo_path` function moved to `prikk-object` by this increment — so for the same
//! invalid path, both sides must report *identical* error text, not merely "both reject it".
//! That identity is the actual thesis of DC-54's "symmetry" name; this test proves it directly
//! rather than asserting each side in isolation.
//!
//! Encode now rejects the DC-41 reproducer before any bytes exist, so there is no way to produce
//! "bad" canonical bytes to feed the decoder anymore (this is the fix working as intended) — the
//! comparison below is against `RepoPath::parse`, the exact function `decode_rename_path` calls.

#![allow(clippy::expect_used)]

use prikk_object::{NodeId, RenamePath};
use prikk_replay::RepoPath;

#[test]
fn encode_and_decode_reject_the_same_invalid_path_with_identical_error_text() {
    for bad_path in ["com1", "../escape", "/absolute", ".prikk/FORMAT"] {
        let operation = RenamePath {
            node_id: NodeId::from_bytes([0x51; 32]),
            old_path: "a".to_string(),
            new_path: bad_path.to_string(),
        };
        let encode_error = operation
            .validate()
            .expect_err("encode must reject this path")
            .to_string();
        let decode_error = RepoPath::parse(bad_path)
            .expect_err("decode's RepoPath::parse must reject this path")
            .to_string();
        assert_eq!(
            encode_error, decode_error,
            "encode and decode must report identical text for {bad_path:?}"
        );
    }
}

#[test]
fn encode_and_decode_agree_a_valid_path_is_accepted() {
    let operation = RenamePath {
        node_id: NodeId::from_bytes([0x52; 32]),
        old_path: "a".to_string(),
        new_path: "b/c.txt".to_string(),
    };
    assert!(operation.validate().is_ok());
    assert!(RepoPath::parse("b/c.txt").is_ok());
}

/// Pins the exact DC-41 committed reproducer
/// (`proptest-regressions/patch_replay/tests/proptest_round_trip.txt`) to fail at **encode**, not
/// decode — the concrete before/after this increment exists to produce. `PatchPayload` construction
/// mirrors the regression file's shrunk case exactly (`node_id: [1; 32]`, `old_path: "a"`,
/// `new_path: "com1"`).
#[test]
fn committed_dc41_reproducer_now_fails_at_encode() {
    use prikk_object::{CanonicalEncode, Operation, OperationKind, PatchPayload, PatchPurpose};

    let operation = RenamePath {
        node_id: NodeId::from_bytes([1; 32]),
        old_path: "a".to_string(),
        new_path: "com1".to_string(),
    };
    let encode_error = operation
        .validate()
        .expect_err("the committed reproducer's path must be rejected at encode");
    assert!(
        encode_error.to_string().contains("com1"),
        "expected the reserved-name error to name the offending component, got: {encode_error}"
    );

    let payload = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::RenamePath(operation),
        }],
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
        message: None,
    };
    assert!(
        payload.to_canonical_bytes().is_err(),
        "the full PatchPayload encode must also fail — there must be no bytes for decode to see"
    );
}
