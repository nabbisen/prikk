//! DC-76: the durability contract's conformance suite. Every function here is generic over
//! `impl DurabilityContract`, so a future platform's implementation is checked by the *same* code,
//! not a re-derived parallel suite — the whole argument for stating a contract rather than leaving
//! the guarantee implicit in Linux's own implementation.
//!
//! **Coverage map — where each guarantee is conformance-tested, new or pre-existing:**
//!
//! | Guarantee | Test |
//! |---|---|
//! | G1 (root-anchored, no-follow) | pre-existing: `tests::directory::required_directory_rejects_symlink_component` |
//! | G2 (atomic content replacement) | pre-existing: `tests::mutable_atomic_write_replaces_complete_content` |
//! | G3 (durable-after-return) | pre-existing: the `fsutil::tests`/`caller_tests` failpoint suite (DC-41); [`durable_directory_entry_makes_prior_mutations_survive_a_process_restart`] below is the guarantee's worked example, stated directly |
//! | G4 (exclusive creation) | new: [`create_exclusive_refuses_an_already_occupied_path`] — no prior direct coverage found |
//! | G5 (race-safe no-clobber publication) | pre-existing: `object_store::tests::immutable::*` |
//! | G6 (regular-file validation) | pre-existing, alongside G7: `tests::append_and_truncate_reject_fifo_without_blocking` |
//! | G7 (non-blocking opens) | same as G6 |
//! | G8 (concurrent-safe directory creation) | pre-existing: `tests::directory::concurrent_required_directory_creation_is_idempotent` |
//! | G9 (mode-bit isolation) | new: [`set_permission_bits_masks_file_type_bits_out_of_a_recorded_mode`] — no prior direct coverage found |

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::fsutil::{DurabilityContract, LinuxDurability, MutationRoot};
use crate::test_support::unique_temp_dir;

fn mutation_root(path: &Path) -> MutationRoot {
    match MutationRoot::open(path) {
        Ok(root) => root,
        Err(error) => panic!("test mutation root failed: {error}"),
    }
}

/// G4: creating the same path twice through `create_exclusive` must refuse the second attempt
/// rather than silently overwrite the first's content — the guarantee has no room for "usually".
fn assert_create_exclusive_refuses_an_already_occupied_path(durability: &impl DurabilityContract) {
    let path = unique_temp_dir("conformance-create-exclusive");
    let root = mutation_root(&path);
    let relative = Path::new("object");

    assert!(
        durability
            .create_exclusive(&root, relative, b"first")
            .is_ok(),
        "first create must succeed"
    );
    let second = durability.create_exclusive(&root, relative, b"second");
    assert!(second.is_err(), "second create at the same path must be refused");
    assert_eq!(
        std::fs::read(path.join("object")).unwrap_or_default(),
        b"first",
        "the refused second create must not have touched the first's content"
    );
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn create_exclusive_refuses_an_already_occupied_path() {
    assert_create_exclusive_refuses_an_already_occupied_path(&LinuxDurability);
}

/// G9: `set_permission_bits` must accept a "recorded mode" carrying file-type bits (exactly the
/// shape a sealed `CreateFile`/`ChangePerm` operation's `mode` field has, e.g. `0o100_755` for a
/// regular file) without corrupting the file's actual type, and the permission bits it applies
/// must match what was asked, masked to `0o7777`.
fn assert_set_permission_bits_masks_file_type_bits_out_of_a_recorded_mode(
    durability: &impl DurabilityContract,
) {
    let path = unique_temp_dir("conformance-set-permission-bits");
    let root = mutation_root(&path);
    let relative = Path::new("object");
    assert!(durability.create_exclusive(&root, relative, b"content").is_ok());

    // S_IFREG (0o100000) | 0o755 -- exactly the shape a recorded CreateFile/ChangePerm mode has.
    let recorded_mode = 0o100_755_u32;
    assert!(
        durability
            .set_permission_bits(&root, relative, recorded_mode)
            .is_ok(),
        "a mode carrying file-type bits must still be accepted"
    );

    let metadata = std::fs::symlink_metadata(path.join("object")).expect("stat after chmod");
    assert!(
        metadata.file_type().is_file(),
        "file type must be unaffected by a permission-bit-setting call"
    );
    assert_eq!(
        metadata.permissions().mode() & 0o7777,
        0o755,
        "permission bits must match what was requested, masked to 0o7777"
    );
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn set_permission_bits_masks_file_type_bits_out_of_a_recorded_mode() {
    assert_set_permission_bits_masks_file_type_bits_out_of_a_recorded_mode(&LinuxDurability);
}

/// G3's worked example, stated directly rather than only through the existing failpoint suite:
/// once `durable_directory_entry` (or any operation that calls it internally, like
/// `atomic_replace`) returns, the mutation survives losing every in-memory and page-cache state —
/// which a fresh `MutationRoot::open` against the same path, after the original root is dropped,
/// stands in for (a real process restart is not reproducible in a unit test; a fresh, unrelated
/// capability observing the same on-disk state is the closest available proxy).
fn assert_durable_directory_entry_makes_prior_mutations_survive_a_process_restart(
    durability: &impl DurabilityContract,
) {
    let path = unique_temp_dir("conformance-durable-directory-entry");
    let root = mutation_root(&path);
    let relative = Path::new("object");
    assert!(durability.atomic_replace(&root, relative, b"durable").is_ok());
    drop(root);

    let fresh_root = mutation_root(&path);
    assert!(durability.durable_directory_entry(&fresh_root, Path::new("")).is_ok());
    assert_eq!(std::fs::read(path.join("object")).unwrap_or_default(), b"durable");
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn durable_directory_entry_makes_prior_mutations_survive_a_process_restart() {
    assert_durable_directory_entry_makes_prior_mutations_survive_a_process_restart(&LinuxDurability);
}
