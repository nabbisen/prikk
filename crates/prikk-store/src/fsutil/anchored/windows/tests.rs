//! DC-87 Stage 2: what is realistically demonstrable on Windows from a unit test running on this
//! host. This is not the full proof the handoff's acceptance criteria ask for -- see the review
//! submission for what remains CI-only (cross-platform history divergence, the reparse-point
//! refusal's dependence on Developer Mode / admin privilege on the CI runner, and DC-76's nine
//! negative controls, which need a Windows failpoint mechanism this stage did not build).

use std::path::Path;

use crate::RepositoryLayout;
use crate::fsutil::{
    MutationRoot, append_file_required, create_new_file_required, ensure_directory_required,
    read_file_if_exists, remove_file_required, set_regular_file_mode_required,
    sync_directory_required, truncate_existing_file_required, truncate_file_empty_required,
    write_file_atomically,
};
use crate::test_support::unique_temp_dir;

fn mutation_root(path: &Path) -> MutationRoot {
    match MutationRoot::open(path) {
        Ok(root) => root,
        Err(error) => panic!("test mutation root failed: {error}"),
    }
}

#[test]
fn create_exclusive_then_read_round_trips() {
    let root_path = unique_temp_dir("windows-create-read");
    let root = mutation_root(&root_path);
    let relative = Path::new("state");

    assert!(create_new_file_required(&root, relative, b"first").is_ok());
    assert_eq!(
        read_file_if_exists(&root, relative).ok().flatten(),
        Some(b"first".to_vec())
    );

    let _ = std::fs::remove_dir_all(root_path);
}

#[test]
fn create_exclusive_refuses_an_already_occupied_path() {
    let root_path = unique_temp_dir("windows-create-exclusive-refuse");
    let root = mutation_root(&root_path);
    let relative = Path::new("state");

    assert!(create_new_file_required(&root, relative, b"first").is_ok());
    assert!(create_new_file_required(&root, relative, b"second").is_err());
    assert_eq!(
        read_file_if_exists(&root, relative).ok().flatten(),
        Some(b"first".to_vec()),
        "a refused create must not have touched the existing content"
    );

    let _ = std::fs::remove_dir_all(root_path);
}

#[test]
fn durable_append_requires_an_existing_file_and_then_appends() {
    let root_path = unique_temp_dir("windows-append");
    let root = mutation_root(&root_path);
    let relative = Path::new("state");

    assert!(
        append_file_required(&root, relative, b"x").is_err(),
        "append to a name nothing created yet must refuse, not create it"
    );
    assert!(create_new_file_required(&root, relative, b"first-").is_ok());
    assert!(append_file_required(&root, relative, b"second").is_ok());
    assert_eq!(
        read_file_if_exists(&root, relative).ok().flatten(),
        Some(b"first-second".to_vec())
    );

    let _ = std::fs::remove_dir_all(root_path);
}

#[test]
fn durable_truncate_and_truncate_to_empty() {
    let root_path = unique_temp_dir("windows-truncate");
    let root = mutation_root(&root_path);
    let relative = Path::new("state");

    assert!(create_new_file_required(&root, relative, b"0123456789").is_ok());
    assert!(truncate_existing_file_required(&root, relative, 4).is_ok());
    assert_eq!(
        read_file_if_exists(&root, relative).ok().flatten(),
        Some(b"0123".to_vec())
    );
    assert!(truncate_file_empty_required(&root, relative).is_ok());
    assert_eq!(
        read_file_if_exists(&root, relative).ok().flatten(),
        Some(Vec::new())
    );

    let _ = std::fs::remove_dir_all(root_path);
}

#[test]
fn remove_if_present_reports_removal_and_absence() {
    let root_path = unique_temp_dir("windows-remove");
    let root = mutation_root(&root_path);
    let relative = Path::new("state");

    assert!(
        remove_file_required(&root, relative).is_ok(),
        "removing an absent file must not error -- absence is not a failure"
    );
    assert!(create_new_file_required(&root, relative, b"x").is_ok());
    assert!(remove_file_required(&root, relative).is_ok());
    assert!(
        read_file_if_exists(&root, relative)
            .ok()
            .flatten()
            .is_none()
    );

    let _ = std::fs::remove_dir_all(root_path);
}

/// `remove_if_present`'s guarantee depends on every open in this backend requesting
/// `FILE_SHARE_DELETE` (design-v1.md §3.1) -- demonstrated by holding a second, independent handle
/// open (via plain `std::fs::File`, `std` default share mode, which already includes
/// `FILE_SHARE_DELETE`) across the removal, rather than trusted from the doc comment alone.
#[test]
fn remove_if_present_succeeds_while_another_handle_holds_the_file_open() {
    let root_path = unique_temp_dir("windows-remove-share-delete");
    let root = mutation_root(&root_path);
    let relative = Path::new("state");
    assert!(create_new_file_required(&root, relative, b"x").is_ok());

    let held_open = std::fs::File::open(root_path.join("state"));
    assert!(held_open.is_ok(), "test setup: could not open the file");

    assert!(remove_file_required(&root, relative).is_ok());

    drop(held_open);
    let _ = std::fs::remove_dir_all(root_path);
}

#[test]
fn atomic_replace_overwrites_existing_content() {
    let root_path = unique_temp_dir("windows-atomic-replace");
    let root = mutation_root(&root_path);
    let relative = Path::new("state");

    assert!(write_file_atomically(&root, relative, b"first").is_ok());
    assert_eq!(
        read_file_if_exists(&root, relative).ok().flatten(),
        Some(b"first".to_vec())
    );
    assert!(write_file_atomically(&root, relative, b"second-longer").is_ok());
    assert_eq!(
        read_file_if_exists(&root, relative).ok().flatten(),
        Some(b"second-longer".to_vec()),
        "std::fs::rename must overwrite an existing destination on Windows, not refuse or leave \
         stale content -- verified here rather than only cited from documentation"
    );

    let _ = std::fs::remove_dir_all(root_path);
}

#[test]
fn ensure_directory_is_idempotent_under_a_concurrent_creator_shape() {
    let root_path = unique_temp_dir("windows-ensure-directory");
    let root = mutation_root(&root_path);
    let relative = Path::new("nested/deeper");

    assert!(ensure_directory_required(&root, relative).is_ok());
    assert!(
        ensure_directory_required(&root, relative).is_ok(),
        "G8: a second ensure over an already-created directory must validate and succeed, not error"
    );
    assert!(root_path.join("nested").join("deeper").is_dir());

    let _ = std::fs::remove_dir_all(root_path);
}

/// design-v1.md §3.3, ruled 2026-08-16: documented no-op. Demonstrated, not merely asserted by
/// comment -- the file's own bytes and existence are unaffected by the call.
#[test]
fn set_permission_bits_is_a_documented_noop() {
    let root_path = unique_temp_dir("windows-set-permission-noop");
    let root = mutation_root(&root_path);
    let relative = Path::new("state");
    assert!(create_new_file_required(&root, relative, b"unchanged").is_ok());

    assert!(set_regular_file_mode_required(&root, relative, 0o100_755).is_ok());

    assert_eq!(
        read_file_if_exists(&root, relative).ok().flatten(),
        Some(b"unchanged".to_vec()),
        "the documented no-op must not have touched the file's content"
    );

    let _ = std::fs::remove_dir_all(root_path);
}

/// design-v1.md §3.4: documented no-op, safe because the worktree marker brackets its only two
/// production callers. Demonstrated here only as "returns Ok and touches nothing" -- the marker's
/// own safety property is `worktree_marker.rs`'s test coverage, not re-proven here.
#[test]
fn durable_directory_entry_is_a_documented_noop() {
    let root_path = unique_temp_dir("windows-durable-directory-entry-noop");
    let root = mutation_root(&root_path);
    let relative = Path::new("state");
    assert!(create_new_file_required(&root, relative, b"unchanged").is_ok());

    assert!(sync_directory_required(&root, relative).is_ok());

    assert_eq!(
        read_file_if_exists(&root, relative).ok().flatten(),
        Some(b"unchanged".to_vec())
    );

    let _ = std::fs::remove_dir_all(root_path);
}

/// design-v1.md §4: an interrupted `init` has nothing to lose and a re-run completes it
/// idempotently -- the argument is stated to hold on Windows unchanged because it depends on
/// ordering (`FORMAT` written last), not on a durability primitive. Demonstrated the same way the
/// Linux/macOS equivalent is: `init` twice on the same root, both succeed. Not a literal
/// crash-injection -- Windows has no failpoint mechanism (§3.6-adjacent finding, reported
/// separately) -- but it is the same idempotent-completion property the exemption's argument rests
/// on.
#[test]
fn repository_init_is_idempotent() {
    let root_path = unique_temp_dir("windows-init-idempotent");
    let first = RepositoryLayout::init(root_path.clone());
    assert!(first.is_ok(), "first init failed: {first:?}");
    let second = RepositoryLayout::init(root_path.clone());
    assert!(
        second.is_ok(),
        "a second init over an already-initialized repository must complete idempotently, not \
         error: {second:?}"
    );

    let _ = std::fs::remove_dir_all(root_path);
}

/// design-v1.md §2: a reparse point substituted for a plain directory or file at any component
/// must be detected and refused. Requires `std::os::windows::fs::symlink_dir`, which needs either
/// Administrator privilege or Developer Mode enabled -- GitHub's hosted `windows-latest` runner is
/// expected to have this, but it was never actually confirmed.
///
/// **DC-97 §2 correction**: this test used to return silently (reporting `ok`) if either
/// precondition failed, on the stated intent that a missing precondition should be "reported as an
/// environment gap." **The code never did that -- it reported nothing, and the guarantee this test
/// exists to pin (G1, the anchoring property DC-96 hardened) could have been asserting nothing on
/// every run with no red test to notice.** Both preconditions now `panic!` with a diagnostic naming
/// exactly what failed, distinguishable from the refusal assertion itself failing. A control that
/// cannot run must fail loudly, not pass silently.
///
/// **DC-97 ordered-list-ruling-v1.md §1-§2 correction**: a probe that disabled `is_reparse_point`
/// entirely (making the reparse-point check itself a no-op) left this test **passing** -- diagnostic
/// evidence (run `31955440915`) showed `validate_directory_not_reparse_point`'s *second* check
/// (`!metadata.is_dir()`) caught the same case, because a no-follow handle on a directory symlink
/// reports `is_dir=false` on Windows. The guarantee holds -- nothing was created through the symlink
/// -- but the old `is_err()` assertion could not tell the two checks apart, so it proved less than
/// it looked like it proved. Tightened to assert the refusal specifically names the reparse point
/// (check 1's own message), not merely that some error occurred.
#[test]
fn a_reparse_point_substituted_for_a_directory_component_is_refused() {
    let root_path = unique_temp_dir("windows-reparse-refusal");
    let root = mutation_root(&root_path);

    let real_target = root_path.join("real-target");
    if let Err(error) = std::fs::create_dir(&real_target) {
        let _ = std::fs::remove_dir_all(&root_path);
        panic!(
            "DC-97 G1: could not create this test's own fixture directory (not the property under \
             test -- an ordinary directory create failed): {error}"
        );
    }
    let link_name = root_path.join("nested");
    if let Err(error) = std::os::windows::fs::symlink_dir(&real_target, &link_name) {
        let _ = std::fs::remove_dir_all(&root_path);
        panic!(
            "DC-97 G1: symlink_dir failed, so this test cannot demonstrate the reparse-point \
             refusal at all on this runner -- likely missing Developer Mode or Administrator \
             privilege: {error}. If this fires in CI, G1 currently has no working Windows control \
             and must be reclassified, not silently skipped."
        );
    }

    let result = ensure_directory_required(&root, Path::new("nested/deeper"));
    let _ = std::fs::remove_dir_all(&root_path);
    let Err(error) = result else {
        panic!(
            "a reparse point standing in for a plain directory component must be refused, but the \
             call succeeded"
        );
    };
    let message = error.to_string();
    assert!(
        message.contains("reparse point"),
        "the refusal must specifically identify the reparse point (`validate_directory_not_reparse_\
         point`'s first check), not merely occur for any reason -- a coincidental type-check \
         failure would satisfy a bare `is_err()` without proving this guarantee at all: {message}"
    );
}
