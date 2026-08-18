#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use super::{
    EntryKind, check, check_entries, check_self_guard, conforms_file, conforms_slug,
    file_declaration_count,
};

#[test]
fn real_tree_passes_unchanged() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repository root");
    let mut errors = Vec::new();
    check(root, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn conforms_file_accepts_a_well_formed_name() {
    assert!(conforms_file("105-rfc-naming-gate.md"));
    assert!(conforms_file("000-rfc-lifecycle-policy.md"));
    assert!(conforms_file("106-anchor-race-control.md"));
}

#[test]
fn conforms_file_rejects_missing_extension() {
    assert!(!conforms_file("105-rfc-naming-gate"));
}

#[test]
fn conforms_slug_accepts_a_bare_directory_name() {
    assert!(conforms_slug("105-rfc-naming-gate"));
    assert!(conforms_slug("101-first-appearance-durability"));
}

#[test]
fn conforms_slug_rejects_two_digit_numbers() {
    assert!(!conforms_slug("99-two-digits"));
}

#[test]
fn conforms_slug_rejects_four_digit_numbers() {
    assert!(!conforms_slug("0105-four-digits"));
}

#[test]
fn conforms_slug_rejects_uppercase() {
    assert!(!conforms_slug("105-RFC-naming-gate"));
}

#[test]
fn conforms_slug_rejects_missing_slug() {
    assert!(!conforms_slug("105"));
    assert!(!conforms_slug("105-"));
}

#[test]
fn conforms_slug_rejects_a_double_hyphen() {
    assert!(!conforms_slug("105-rfc--naming-gate"));
}

#[test]
fn conforms_slug_rejects_the_legacy_dc_scheme() {
    assert!(!conforms_slug("DC-96-windows-anchor-identity"));
}

#[test]
fn conforms_slug_accepts_digits_within_a_segment() {
    assert!(conforms_slug("046-workspace-rust-1-85-compatibility"));
}

fn write_files(root: &Path, relative_dir: &str, names: &[&str]) {
    let directory = root.join(relative_dir);
    std::fs::create_dir_all(&directory).unwrap();
    for name in names {
        std::fs::write(directory.join(name), b"").unwrap();
    }
}

fn write_dirs(root: &Path, relative_dir: &str, names: &[&str]) {
    let directory = root.join(relative_dir);
    std::fs::create_dir_all(&directory).unwrap();
    for name in names {
        std::fs::create_dir(directory.join(name)).unwrap();
    }
}

#[test]
fn a_conforming_file_with_an_empty_legacy_list_passes() {
    let temporary = tempfile::tempdir().unwrap();
    write_files(temporary.path(), "rfcs/accepted", &["107-example.md"]);
    let mut errors = Vec::new();
    check_entries(
        temporary.path(),
        "rfcs/accepted",
        EntryKind::File,
        &[],
        &mut errors,
    );
    assert!(errors.is_empty(), "{errors:?}");
}

/// The negative control RFC criterion 4 asks for by name: a non-conforming name, not in the legacy
/// allowlist, must fail -- observed directly, not reasoned about.
#[test]
fn a_non_conforming_file_not_in_the_legacy_list_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_files(temporary.path(), "rfcs/accepted", &["DC-999-NOT-LEGACY.md"]);
    let mut errors = Vec::new();
    check_entries(
        temporary.path(),
        "rfcs/accepted",
        EntryKind::File,
        &[],
        &mut errors,
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].detail,
        "rfcs/accepted/DC-999-NOT-LEGACY.md: does not conform and is not in the legacy allowlist"
    );
}

#[test]
fn a_non_conforming_file_in_the_legacy_list_passes() {
    let temporary = tempfile::tempdir().unwrap();
    write_files(temporary.path(), "rfcs/accepted", &["DC-34-EXAMPLE.md"]);
    let mut errors = Vec::new();
    check_entries(
        temporary.path(),
        "rfcs/accepted",
        EntryKind::File,
        &["DC-34-EXAMPLE.md"],
        &mut errors,
    );
    assert!(errors.is_empty(), "{errors:?}");
}

/// RFC criterion 3, the self-guard: an allowlisted name that does not exist must fail -- this is
/// what closes the cheap bypass of pre-authorising a future name before creating it.
#[test]
fn an_allowlisted_name_with_no_corresponding_file_fails() {
    let temporary = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temporary.path().join("rfcs/accepted")).unwrap();
    let mut errors = Vec::new();
    check_self_guard(
        temporary.path(),
        "rfcs/accepted",
        &["rfcs/accepted"],
        EntryKind::File,
        &["DC-100-NOT-YET-CREATED.md"],
        &mut errors,
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].detail,
        "rfcs/accepted/DC-100-NOT-YET-CREATED.md: allowlisted but does not exist under any of \
         rfcs/accepted"
    );
}

/// A file-form allowlist entry does not satisfy the self-guard for a directory of the same name,
/// and vice versa -- the entry scan skips the mismatched kind, and the self-guard then finds no
/// matching entry of the right kind, either.
#[test]
fn an_allowlisted_name_that_exists_as_the_wrong_kind_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_dirs(temporary.path(), "rfcs/accepted", &["DC-34-EXAMPLE.md"]);
    let mut errors = Vec::new();
    check_entries(
        temporary.path(),
        "rfcs/accepted",
        EntryKind::File,
        &["DC-34-EXAMPLE.md"],
        &mut errors,
    );
    check_self_guard(
        temporary.path(),
        "rfcs/accepted",
        &["rfcs/accepted"],
        EntryKind::File,
        &["DC-34-EXAMPLE.md"],
        &mut errors,
    );
    // The directory itself is skipped by the entry scan (not a file, out of this location's
    // governed kind), and the self-guard then finds no *file* at that name -- both surface as one
    // failure, on the self-guard side.
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].detail,
        "rfcs/accepted/DC-34-EXAMPLE.md: allowlisted but does not exist under any of rfcs/accepted"
    );
}

#[test]
fn a_conforming_directory_under_handoffs_passes() {
    let temporary = tempfile::tempdir().unwrap();
    write_dirs(temporary.path(), "rfcs/handoffs", &["107-example"]);
    let mut errors = Vec::new();
    check_entries(
        temporary.path(),
        "rfcs/handoffs",
        EntryKind::Directory,
        &[],
        &mut errors,
    );
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_non_conforming_directory_under_handoffs_not_in_the_legacy_list_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_dirs(temporary.path(), "rfcs/handoffs", &["not-numbered"]);
    let mut errors = Vec::new();
    check_entries(
        temporary.path(),
        "rfcs/handoffs",
        EntryKind::Directory,
        &[],
        &mut errors,
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].detail,
        "rfcs/handoffs/not-numbered: does not conform and is not in the legacy allowlist"
    );
}

#[test]
fn an_unreadable_location_fails_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let mut errors = Vec::new();
    check_entries(
        temporary.path(),
        "rfcs/does-not-exist",
        EntryKind::File,
        &[],
        &mut errors,
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].detail,
        "rfcs/does-not-exist: directory unreadable"
    );
}

/// RFC 105 lifecycle-move-friction handoff: the actual regression this fix is for. A name declared
/// in one location's legacy list (mirroring `RFC_PROPOSED_LEGACY`'s real `DC-53-...md` entry) but
/// physically moved to a *different* file-governed location must pass both the entry scan (via the
/// combined allowlist) and the self-guard (via the cross-location search) -- reproduced and shown
/// fixed, not just argued.
#[test]
fn a_name_declared_in_one_list_but_moved_to_a_different_file_location_passes() {
    let temporary = tempfile::tempdir().unwrap();
    write_files(temporary.path(), "rfcs/done", &["DC-53-EXAMPLE.md"]);
    let combined = ["DC-53-EXAMPLE.md"];
    let file_locations = [
        "rfcs/proposed",
        "rfcs/accepted",
        "rfcs/done",
        "rfcs/archive",
    ];
    let mut errors = Vec::new();
    for location in file_locations {
        std::fs::create_dir_all(temporary.path().join(location)).unwrap();
        check_entries(
            temporary.path(),
            location,
            EntryKind::File,
            &combined,
            &mut errors,
        );
    }
    // Declared under `rfcs/proposed` (where it used to live), searched across all four -- found
    // under `rfcs/done` instead, which must still satisfy the self-guard.
    check_self_guard(
        temporary.path(),
        "rfcs/proposed",
        &file_locations,
        EntryKind::File,
        &combined,
        &mut errors,
    );
    assert!(errors.is_empty(), "{errors:?}");
}

/// RFC 105 lifecycle-move-friction handoff §4's explicit ask: combining the four file lists must
/// not let a genuinely non-conforming, unlisted name slip through at *any* of them.
#[test]
fn a_non_conforming_name_absent_from_every_file_list_fails_at_every_file_location() {
    let combined: [&str; 0] = [];
    let file_locations = [
        "rfcs/proposed",
        "rfcs/accepted",
        "rfcs/done",
        "rfcs/archive",
    ];
    for location in file_locations {
        let temporary = tempfile::tempdir().unwrap();
        write_files(temporary.path(), location, &["DC-999-NOT-LEGACY.md"]);
        let mut errors = Vec::new();
        check_entries(
            temporary.path(),
            location,
            EntryKind::File,
            &combined,
            &mut errors,
        );
        assert_eq!(errors.len(), 1, "{location}: {errors:?}");
        assert_eq!(
            errors[0].detail,
            format!(
                "{location}/DC-999-NOT-LEGACY.md: does not conform and is not in the legacy allowlist"
            )
        );
    }
}

/// RFC 105 lifecycle-move-friction handoff, plan review §1's required amendment: an allowlisted
/// name that exists at *two* file locations at once must fail as a duplicate -- this project has
/// shipped one before (RFC 104, withdrawn as a duplicate of DC-87) -- not silently pass because "at
/// least one" was satisfied.
#[test]
fn an_allowlisted_name_that_exists_at_two_locations_fails_as_a_duplicate() {
    let temporary = tempfile::tempdir().unwrap();
    write_files(temporary.path(), "rfcs/proposed", &["DC-53-EXAMPLE.md"]);
    write_files(temporary.path(), "rfcs/done", &["DC-53-EXAMPLE.md"]);
    let file_locations = [
        "rfcs/proposed",
        "rfcs/accepted",
        "rfcs/done",
        "rfcs/archive",
    ];
    let mut errors = Vec::new();
    check_self_guard(
        temporary.path(),
        "rfcs/proposed",
        &file_locations,
        EntryKind::File,
        &["DC-53-EXAMPLE.md"],
        &mut errors,
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].detail,
        "rfcs/proposed/DC-53-EXAMPLE.md: allowlisted but exists more than once, at rfcs/proposed, \
         rfcs/done"
    );
}

/// `.gitkeep` is the reason `check`'s self-guard cannot treat every name the same way: it is
/// declared independently in three of the four file lists (one per directory that needs a
/// placeholder), each an unrelated file that happens to share a name -- not one identity that moved.
/// Asserted against the real, current lists rather than a synthetic stand-in, so this fails the
/// moment someone edits a list in a way that changes the count.
#[test]
fn gitkeep_is_declared_in_more_than_one_file_list() {
    assert_eq!(file_declaration_count(".gitkeep"), 3);
}

/// The ordinary case: a genuine legacy RFC name is declared in exactly one file list -- the
/// property that makes it a single movable identity rather than a repeated placeholder.
#[test]
fn a_real_legacy_rfc_name_is_declared_in_exactly_one_file_list() {
    assert_eq!(
        file_declaration_count("DC-53-REPOSITORY-WIDE-AUTHOR-TRUST-VERIFICATION.md"),
        1
    );
}
