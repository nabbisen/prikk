//! DC-54 - operation path validation symmetry: encode-side test matrix.
//!
//! Encode must now reject exactly what decode rejects, for the five path-carrying fields across
//! `CreateFile`, `DeleteNode`, `RenamePath` (two fields), and `CreateSymlink`. The symmetry case
//! itself (same input, identical error text through encode and through decode) lives in
//! `prikk-store`, since only that crate can call the decoder; this module covers what a single
//! crate can prove on its own — encode's side of the matrix, plus the untouched opaque fields.

use crate::{
    CreateFile, CreateSymlink, DeleteNode, DeleteNodePreimage, NodeId, NodeKind, ObjectId,
    RenamePath,
};

const NODE_ID: NodeId = NodeId::from_bytes([0x30; 32]);
const BLOB_ID: ObjectId = ObjectId::from_bytes([0x40; 32]);

const VALID_PATHS: [&str; 1] = ["a/b.txt"];
const INVALID_PATHS: [(&str, &str); 4] = [
    ("com1", "Windows-reserved device name"),
    ("../escape", "traversal component"),
    ("/absolute", "absolute path"),
    (".prikk/FORMAT", ".prikk-prefixed path"),
];

fn create_file(path: &str) -> CreateFile {
    CreateFile {
        path: path.to_string(),
        node_id: NODE_ID,
        blob_id: BLOB_ID,
        mode: 0o100_644,
    }
}

fn delete_node(path: &str) -> DeleteNode {
    DeleteNode {
        path: path.to_string(),
        node_id: NODE_ID,
        old_node_kind: NodeKind::TextFile,
        preimage: DeleteNodePreimage::File {
            old_blob_id: BLOB_ID,
            old_mode: 0o100_644,
        },
    }
}

fn rename_path(old_path: &str, new_path: &str) -> RenamePath {
    RenamePath {
        node_id: NODE_ID,
        old_path: old_path.to_string(),
        new_path: new_path.to_string(),
    }
}

fn create_symlink(path: &str) -> CreateSymlink {
    CreateSymlink {
        path: path.to_string(),
        node_id: NODE_ID,
        target: "somewhere".to_string(),
    }
}

#[test]
fn valid_paths_encode_successfully_for_every_kind() {
    for path in VALID_PATHS {
        assert!(create_file(path).validate().is_ok(), "CreateFile {path}");
        assert!(delete_node(path).validate().is_ok(), "DeleteNode {path}");
        assert!(
            rename_path(path, path).validate().is_ok(),
            "RenamePath {path}"
        );
        assert!(
            create_symlink(path).validate().is_ok(),
            "CreateSymlink {path}"
        );
    }
}

#[test]
fn invalid_paths_fail_at_encode_for_every_kind() {
    for (path, reason) in INVALID_PATHS {
        assert!(
            create_file(path).validate().is_err(),
            "CreateFile should reject {path} ({reason})"
        );
        assert!(
            delete_node(path).validate().is_err(),
            "DeleteNode should reject {path} ({reason})"
        );
        assert!(
            rename_path("a", path).validate().is_err(),
            "RenamePath.new_path should reject {path} ({reason})"
        );
        assert!(
            create_symlink(path).validate().is_err(),
            "CreateSymlink should reject {path} ({reason})"
        );
    }
}

/// Guards the two-call requirement in `RenamePath::validate()`: a single check covering only
/// `new_path` would leave `old_path` asymmetric, and would still pass the DC-41 reproducer
/// (`new_path: "com1"`, `old_path: "a"`) because `"a"` alone is valid.
#[test]
fn rename_path_rejects_a_bad_old_path_even_when_new_path_is_valid() {
    let op = rename_path("com1", "valid");
    assert!(
        op.validate().is_err(),
        "old_path=\"com1\" must be rejected even though new_path is valid"
    );
}

/// `DeleteNode.old_target` (symlink preimage) and `CreateSymlink.target` are opaque by accepted
/// DC-40 design and must remain unvalidated — this is the "do not touch" boundary DC-54 must not
/// cross by accident.
#[test]
fn opaque_target_fields_still_accept_arbitrary_utf8() {
    let delete_symlink = DeleteNode {
        path: "a".to_string(),
        node_id: NODE_ID,
        old_node_kind: NodeKind::Symlink,
        preimage: DeleteNodePreimage::Symlink {
            old_target: "com1".to_string(), // would be rejected if this were a `path` field
        },
    };
    assert!(delete_symlink.validate().is_ok());

    let symlink = CreateSymlink {
        path: "a".to_string(),
        node_id: NODE_ID,
        target: "../anywhere".to_string(), // would be rejected if this were a `path` field
    };
    assert!(symlink.validate().is_ok());
}
