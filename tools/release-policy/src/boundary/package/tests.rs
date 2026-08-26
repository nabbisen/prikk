#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use super::{PRODUCTS, check_descriptions, check_readmes};

fn write_baseline(root: &Path) {
    for (name, manifest_path) in PRODUCTS {
        write_manifest(
            root,
            manifest_path,
            &format!("[package]\ndescription = \"A component named {name}.\"\n"),
        );
    }
}

/// Baseline for `check_readmes`: every crate gets a manifest with both `description` and
/// `readme = "README.md"`, plus a matching README next to it that says something else entirely.
fn write_baseline_with_readmes(root: &Path) {
    for (name, manifest_path) in PRODUCTS {
        write_manifest(
            root,
            manifest_path,
            &format!(
                "[package]\ndescription = \"A component named {name}.\"\nreadme = \"README.md\"\n"
            ),
        );
        let readme_path = readme_path_for(manifest_path);
        write_manifest(
            root,
            &readme_path,
            &format!("# {name}\n\nInternal component; see the `prikk` CLI.\n"),
        );
    }
}

fn readme_path_for(manifest_path: &str) -> String {
    format!(
        "{}/README.md",
        Path::new(manifest_path).parent().unwrap().display()
    )
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

#[test]
fn real_tree_readmes_pass_unchanged() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repository root");
    let mut errors = Vec::new();
    check_readmes(root, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn provisional_word_in_readme_fails() {
    // Crate-README-currency handoff v1 §7 control 1.
    let temporary = tempfile::tempdir().unwrap();
    write_baseline_with_readmes(temporary.path());
    write_manifest(
        temporary.path(),
        &readme_path_for("crates/prikk-store/Cargo.toml"),
        "# prikk-store\n\nPrikk storage crate scaffold.\n",
    );
    let mut errors = Vec::new();
    check_readmes(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].category, "package-readme");
    assert_eq!(
        errors[0].detail,
        "prikk-store: provisional word \"scaffold\" in readme"
    );
}

#[test]
fn missing_readme_named_by_the_manifest_fails() {
    // Crate-README-currency handoff v1 §7 control 2.
    let temporary = tempfile::tempdir().unwrap();
    write_baseline_with_readmes(temporary.path());
    std::fs::remove_file(
        temporary
            .path()
            .join(readme_path_for("crates/prikk-error/Cargo.toml")),
    )
    .unwrap();
    let mut errors = Vec::new();
    check_readmes(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].category, "package-readme");
    assert_eq!(
        errors[0].detail,
        "prikk-error: readme unreadable (README.md)"
    );
}

#[test]
fn readme_duplicating_its_own_description_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline_with_readmes(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-object/Cargo.toml",
        "[package]\ndescription = \"Object identity and canonical payload types for Prikk.\"\nreadme = \"README.md\"\n",
    );
    write_manifest(
        temporary.path(),
        &readme_path_for("crates/prikk-object/Cargo.toml"),
        "# prikk-object\n\nObject identity and canonical payload types for Prikk.\n",
    );
    let mut errors = Vec::new();
    check_readmes(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].category, "package-readme-duplication");
    assert_eq!(
        errors[0].detail,
        "prikk-object: readme repeats description sentence \"object identity and canonical payload types for prikk\""
    );
}

#[test]
fn prikk_is_exempt_from_the_duplication_check() {
    // `prikk`'s readme is the workspace root README, and its description is deliberately sourced
    // from that document's own sentence -- the opposite direction from the seven library crates'
    // defect. This must never fail the duplication check.
    let temporary = tempfile::tempdir().unwrap();
    write_baseline_with_readmes(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-cli/Cargo.toml",
        "[package]\ndescription = \"A standalone distributed version control system.\"\nreadme = \"README.md\"\n",
    );
    write_manifest(
        temporary.path(),
        &readme_path_for("crates/prikk-cli/Cargo.toml"),
        "# prikk\n\n**Prikk is a standalone distributed version control system.**\n",
    );
    let mut errors = Vec::new();
    check_readmes(temporary.path(), &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}
