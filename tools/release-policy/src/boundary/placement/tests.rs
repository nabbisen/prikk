#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use super::{PRODUCTS, check};

fn write_baseline(root: &Path) {
    for (_, manifest_path) in PRODUCTS {
        write_manifest(root, manifest_path, "[dependencies]\n");
    }
}

fn write_manifest(root: &Path, relative: &str, body: &str) {
    let full = root.join(relative);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, body).unwrap();
}

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
fn disallowed_third_party_in_product_dependencies_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-hash/Cargo.toml",
        "[dependencies]\nsha2 = \"0.10\"\n",
    );
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].detail, "prikk-hash:sha2");
}

#[test]
fn dev_dependency_sink_stays_open() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-store/Cargo.toml",
        "[dependencies]\n[dev-dependencies]\nproptest = \"1\"\n",
    );
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn allowlisted_third_party_in_its_own_crate_passes() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-crypto/Cargo.toml",
        "[dependencies]\ned25519-dalek = \"2\"\n",
    );
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn right_dependency_wrong_crate_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-cli/Cargo.toml",
        "[dependencies]\ned25519-dalek = \"2\"\n",
    );
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].detail, "prikk:ed25519-dalek");
}

#[test]
fn workspace_internal_dependency_passes_anywhere() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-store/Cargo.toml",
        "[dependencies]\nprikk-object = { workspace = true }\n",
    );
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn disallowed_third_party_in_build_dependencies_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-object/Cargo.toml",
        "[dependencies]\n[build-dependencies]\nsha2 = \"0.10\"\n",
    );
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].detail, "prikk-object:sha2");
}

#[test]
fn disallowed_third_party_under_target_dependencies_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-store/Cargo.toml",
        "[dependencies]\n[target.'cfg(unix)'.dependencies]\nsha2 = \"0.10\"\n",
    );
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].detail, "prikk-store:sha2");
}

#[test]
fn renamed_dependency_under_allowlisted_key_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-store/Cargo.toml",
        "[dependencies]\ngetrandom = { package = \"proptest\", version = \"1\" }\n",
    );
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].detail, "prikk-store:getrandom");
}

#[test]
fn unreadable_manifest_fails_closed() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    std::fs::remove_file(temporary.path().join("crates/prikk-error/Cargo.toml")).unwrap();
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].detail, "prikk-error: manifest unreadable");
}

#[test]
fn unparseable_manifest_fails_closed() {
    let temporary = tempfile::tempdir().unwrap();
    write_baseline(temporary.path());
    write_manifest(
        temporary.path(),
        "crates/prikk-error/Cargo.toml",
        "[dependencies\n",
    );
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].detail, "prikk-error: manifest unparseable");
}
