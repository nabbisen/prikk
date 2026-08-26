#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use super::{PRODUCTS, check_descriptions};

fn write_baseline(root: &Path) {
    for (name, manifest_path) in PRODUCTS {
        write_manifest(
            root,
            manifest_path,
            &format!("[package]\ndescription = \"A component named {name}.\"\n"),
        );
    }
}

fn write_manifest(root: &Path, relative: &str, body: &str) {
    let full = root.join(relative);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, body).unwrap();
}

#[test]
fn real_tree_descriptions_pass_unchanged() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repository root");
    let mut errors = Vec::new();
    check_descriptions(root, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn provisional_word_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-cli/Cargo.toml",
        "[package]\ndescription = \"Prikk CLI initial scaffold.\"\n",
    );
    let mut errors = Vec::new();
    check_descriptions(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors
            .iter()
            .all(|error| error.category == "package-description")
    );
    assert_eq!(errors[0].detail, "prikk: provisional word \"scaffold\"");
    assert_eq!(errors[1].detail, "prikk: provisional word \"initial\"");
}

#[test]
fn provisional_word_is_case_insensitive() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-store/Cargo.toml",
        "[package]\ndescription = \"Prikk storage crate SCAFFOLD.\"\n",
    );
    let mut errors = Vec::new();
    check_descriptions(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].detail,
        "prikk-store: provisional word \"scaffold\""
    );
}

#[test]
fn initial_is_a_whole_word_not_a_substring() {
    // Review of this handoff's v1 report: `initial` as a raw substring rejected legitimate
    // descriptions using `initialize`/`initialization`, vocabulary this project's own crates have
    // every reason to use. Whole-word matching must let this through.
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-hash/Cargo.toml",
        "[package]\ndescription = \"SHA-256 hash primitives used during repository initialization.\"\n",
    );
    let mut errors = Vec::new();
    check_descriptions(temporary.path(), &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn initial_as_a_standalone_word_still_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-hash/Cargo.toml",
        "[package]\ndescription = \"Initial hash primitives for Prikk.\"\n",
    );
    let mut errors = Vec::new();
    check_descriptions(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].detail, "prikk-hash: provisional word \"initial\"");
}

#[test]
fn missing_description_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-store/Cargo.toml",
        "[package]\n",
    );
    let mut errors = Vec::new();
    check_descriptions(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].detail, "prikk-store: missing description");
}

#[test]
fn blank_description_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-store/Cargo.toml",
        "[package]\ndescription = \"   \"\n",
    );
    let mut errors = Vec::new();
    check_descriptions(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].detail, "prikk-store: missing description");
}
