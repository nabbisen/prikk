#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::path::Path;

use super::check;

const MARKERS: &str = "<!-- open-work-index:start -->\n\
     - [`kept.md`](rfcs/proposed/kept.md) — RFC 900, kept\n\
     <!-- open-work-index:end -->\n";

fn write_roadmap(root: &Path, index_body: &str) {
    std::fs::write(root.join("ROADMAP.md"), index_body).unwrap();
}

fn write_proposed(root: &Path, name: &str) {
    let dir = root.join("rfcs/proposed");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(name), "# stub\n").unwrap();
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
fn a_proposed_file_not_in_the_index_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_roadmap(temporary.path(), MARKERS);
    write_proposed(temporary.path(), "kept.md");
    write_proposed(temporary.path(), "unindexed.md");
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].detail,
        "rfcs/proposed/unindexed.md is not named in ROADMAP.md's open-work index"
    );
}

#[test]
fn an_index_entry_naming_a_nonexistent_file_fails() {
    let temporary = tempfile::tempdir().unwrap();
    write_roadmap(temporary.path(), MARKERS);
    // `kept.md` is named in the index (MARKERS) but never written to rfcs/proposed/.
    std::fs::create_dir_all(temporary.path().join("rfcs/proposed")).unwrap();
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].detail,
        "ROADMAP.md's open-work index names rfcs/proposed/kept.md, which does not exist"
    );
}

#[test]
fn gitkeep_is_exempt_from_both_directions() {
    let temporary = tempfile::tempdir().unwrap();
    write_roadmap(temporary.path(), MARKERS);
    write_proposed(temporary.path(), "kept.md");
    write_proposed(temporary.path(), ".gitkeep");
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn missing_markers_fails_closed() {
    let temporary = tempfile::tempdir().unwrap();
    write_roadmap(temporary.path(), "# Roadmap\n\nno markers here\n");
    write_proposed(temporary.path(), "kept.md");
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].detail,
        "ROADMAP.md: missing <!-- open-work-index:start --> or <!-- open-work-index:end -->"
    );
}

#[test]
fn text_outside_the_markers_is_not_scanned() {
    let temporary = tempfile::tempdir().unwrap();
    write_roadmap(
        temporary.path(),
        &format!("- [`outside.md`](rfcs/proposed/outside.md) — not gated\n{MARKERS}"),
    );
    write_proposed(temporary.path(), "kept.md");
    let mut errors = Vec::new();
    check(temporary.path(), &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
}
