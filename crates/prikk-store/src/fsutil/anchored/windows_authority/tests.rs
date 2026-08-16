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

/// DC-96 implementation-ruling-v1 §5: §8.4 reopened by retaining a handle -- confirm with a test,
/// not by reading the share flags. `WindowsAuthority::bind` now keeps a directory handle open for
/// the authority's whole lifetime; every open in this backend already requests
/// `FILE_SHARE_DELETE` (`windows.rs`'s own module doc), which is specifically what lets Windows
/// rename or delete a path while another handle holds it open. If that ever regressed, a user
/// renaming their own repository while prikk holds it open would start failing with a sharing
/// violation instead of the retained handle simply following the rename, as it must.
#[test]
fn a_users_own_rename_of_the_anchor_succeeds_while_the_handle_is_retained() {
    let root_path = unique_temp_dir("windows-authority-rename-not-blocked");
    let root = mutation_root(&root_path);
    let renamed_path = root_path.with_extension("renamed-by-user");
    let _ = std::fs::remove_dir_all(&renamed_path);

    let rename_result = std::fs::rename(&root_path, &renamed_path);

    assert!(
        rename_result.is_ok(),
        "a user must be able to rename the repository root even while prikk retains an open \
         handle to it -- FILE_SHARE_DELETE exists exactly for this: {rename_result:?}"
    );

    drop(root);
    let _ = std::fs::remove_dir_all(&renamed_path);
}
