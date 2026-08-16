//! DC-96 Windows Anchor Identity: the positive control (design-v1.md §6.3) -- ordinary operation,
//! no anchor swap, must still succeed. Without this, a `verify_anchor` that always refuses would
//! pass acceptance criterion 1 (the two negative-control tests would just never run their write)
//! for the wrong reason.

use std::path::Path;

use crate::fsutil::{MutationRoot, create_new_file_required, read_file_if_exists};
use crate::test_support::unique_temp_dir;

fn mutation_root(path: &Path) -> MutationRoot {
    match MutationRoot::open(path) {
        Ok(root) => root,
        Err(error) => panic!("test mutation root failed: {error}"),
    }
}

#[test]
fn identity_verification_does_not_disturb_ordinary_operation() {
    let root_path = unique_temp_dir("windows-authority-positive-control");
    let root = mutation_root(&root_path);
    let relative = Path::new("state");

    assert!(
        create_new_file_required(&root, relative, b"unchanged anchor").is_ok(),
        "a write through the same, unswapped anchor must still succeed"
    );
    assert_eq!(
        read_file_if_exists(&root, relative).ok().flatten(),
        Some(b"unchanged anchor".to_vec()),
        "a read through the same, unswapped anchor must still see what was written"
    );

    let _ = std::fs::remove_dir_all(root_path);
}
