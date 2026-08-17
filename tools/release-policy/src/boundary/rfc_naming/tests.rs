#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use super::{EntryKind, check, check_location, conforms_file, conforms_slug};

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
    check_location(
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
    check_location(
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
    check_location(
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
    check_location(
        temporary.path(),
        "rfcs/accepted",
        EntryKind::File,
        &["DC-100-NOT-YET-CREATED.md"],
        &mut errors,
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].detail,
        "rfcs/accepted/DC-100-NOT-YET-CREATED.md: allowlisted but does not exist"
    );
}

/// A file-form allowlist entry does not satisfy the self-guard for a directory of the same name,
/// and vice versa -- each location's list is checked against its own entry kind.
#[test]
fn an_allowlisted_name_that_exists_as_the_wrong_kind_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_dirs(temporary.path(), "rfcs/accepted", &["DC-34-EXAMPLE.md"]);
    let mut errors = Vec::new();
    check_location(
        temporary.path(),
        "rfcs/accepted",
        EntryKind::File,
        &["DC-34-EXAMPLE.md"],
        &mut errors,
    );
    // The directory itself is skipped (not a file, out of this location's governed kind), and the
    // self-guard then finds no *file* at that name -- both surface as one failure, on the
    // self-guard side.
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].detail,
        "rfcs/accepted/DC-34-EXAMPLE.md: allowlisted but does not exist"
    );
}

#[test]
fn a_conforming_directory_under_handoffs_passes() {
    let temporary = tempfile::tempdir().unwrap();
    write_dirs(temporary.path(), "rfcs/handoffs", &["107-example"]);
    let mut errors = Vec::new();
    check_location(
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
    check_location(
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
    check_location(
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
