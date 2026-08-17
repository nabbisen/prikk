#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use super::{assemble, changelog_section, classify_target, platform_paragraph};

const SAMPLE_CHANGELOG: &str = "\
# Changelog

## 0.22.1 — 2026-08-18

**Fixed a bug.** Details here.

### Fixed

- one thing


## 0.22.0 — 2026-08-17

**Windows catches up.** More text.

## 0.1.0 PR-030

Legacy heading, no em dash, no date.

## Earlier PRs

Legacy heading with no version at all.
";

#[test]
fn extracts_the_exact_matching_version_section() {
    let section = changelog_section(SAMPLE_CHANGELOG, "0.22.0").expect("section must be found");
    assert_eq!(section, "**Windows catches up.** More text.");
}

#[test]
fn stops_at_the_next_heading_and_trims_trailing_blank_lines() {
    let section = changelog_section(SAMPLE_CHANGELOG, "0.22.1").expect("section must be found");
    assert_eq!(
        section,
        "**Fixed a bug.** Details here.\n\n### Fixed\n\n- one thing"
    );
}

#[test]
fn does_not_match_a_legacy_heading_with_no_em_dash() {
    assert!(changelog_section(SAMPLE_CHANGELOG, "0.1.0").is_none());
}

#[test]
fn does_not_prefix_match_a_longer_version() {
    // "0.22" must not match the "0.22.0" or "0.22.1" headings.
    assert!(changelog_section(SAMPLE_CHANGELOG, "0.22").is_none());
}

#[test]
fn returns_none_for_a_tag_with_no_entry() {
    assert!(changelog_section(SAMPLE_CHANGELOG, "9.9.9").is_none());
}

fn write_build_info(dir: &Path, name: &str, target: &str) {
    std::fs::write(
        dir.join(format!("prikk-{name}.build-info.txt")),
        format!("target: {target}\ncommit: deadbeef\ntag: 0.22.1\nbuild: cargo build\n"),
    )
    .unwrap();
}

#[test]
fn a_single_os_renders_as_only() {
    let temporary = tempfile::tempdir().unwrap();
    write_build_info(temporary.path(), "x86_64", "x86_64-unknown-linux-gnu");
    write_build_info(temporary.path(), "aarch64", "aarch64-unknown-linux-gnu");
    let paragraph = platform_paragraph(temporary.path()).unwrap();
    // Architectures within an OS group are ordered by sorting the full target triple, not
    // hand-picked -- "aarch64-..." sorts before "x86_64-..." alphabetically.
    assert!(
        paragraph.starts_with("## Prebuilt binaries\n\nLinux only (`aarch64`/`x86_64`)."),
        "{paragraph}"
    );
}

#[test]
fn multiple_operating_systems_are_listed_without_only() {
    let temporary = tempfile::tempdir().unwrap();
    write_build_info(temporary.path(), "x86_64", "x86_64-unknown-linux-gnu");
    write_build_info(temporary.path(), "macos", "aarch64-apple-darwin");
    write_build_info(temporary.path(), "windows", "x86_64-pc-windows-msvc");
    let paragraph = platform_paragraph(temporary.path()).unwrap();
    assert!(
        paragraph.starts_with(
            "## Prebuilt binaries\n\nLinux (`x86_64`), Windows (`x86_64`), macOS (`aarch64`)."
        ),
        "{paragraph}"
    );
}

#[test]
fn an_unrecognized_triple_fails_closed() {
    let temporary = tempfile::tempdir().unwrap();
    // classify_target only looks at the vendor/OS segment, so an unfamiliar architecture on a
    // recognized OS (Linux) is not a good probe here -- it is deliberately accepted. What must
    // fail closed is an OS this project does not support at all.
    write_build_info(temporary.path(), "freebsd", "x86_64-unknown-freebsd");
    assert!(platform_paragraph(temporary.path()).is_err());
}

#[test]
fn no_build_info_files_fails_rather_than_publishing_nothing() {
    let temporary = tempfile::tempdir().unwrap();
    assert!(platform_paragraph(temporary.path()).is_err());
}

#[test]
fn classify_target_recognizes_all_three_supported_platforms() {
    assert_eq!(
        classify_target("x86_64-unknown-linux-gnu").unwrap(),
        ("Linux", "x86_64".to_owned())
    );
    assert_eq!(
        classify_target("aarch64-apple-darwin").unwrap(),
        ("macOS", "aarch64".to_owned())
    );
    assert_eq!(
        classify_target("x86_64-pc-windows-gnu").unwrap(),
        ("Windows", "x86_64".to_owned())
    );
}

#[test]
fn assemble_fails_the_release_when_the_tag_has_no_changelog_entry() {
    let temporary = tempfile::tempdir().unwrap();
    std::fs::write(temporary.path().join("CHANGELOG.md"), SAMPLE_CHANGELOG).unwrap();
    let dist = temporary.path().join("dist");
    std::fs::create_dir_all(&dist).unwrap();
    write_build_info(&dist, "x86_64", "x86_64-unknown-linux-gnu");

    let error = assemble(temporary.path(), "9.9.9", &dist).expect_err("no matching entry");
    assert!(error.to_string().contains("9.9.9"), "{error}");
}

#[test]
fn assemble_joins_the_changelog_section_platforms_and_release_authority() {
    let temporary = tempfile::tempdir().unwrap();
    std::fs::write(temporary.path().join("CHANGELOG.md"), SAMPLE_CHANGELOG).unwrap();
    let dist = temporary.path().join("dist");
    std::fs::create_dir_all(&dist).unwrap();
    write_build_info(&dist, "x86_64", "x86_64-unknown-linux-gnu");

    let notes = assemble(temporary.path(), "0.22.0", &dist).unwrap();
    assert!(
        notes.starts_with("**Windows catches up.** More text."),
        "{notes}"
    );
    assert!(notes.contains("## Prebuilt binaries"), "{notes}");
    assert!(notes.contains("Linux only (`x86_64`)."), "{notes}");
    assert!(!notes.contains("mutation"), "{notes}");
    assert!(
        notes.contains("## Release authority — read before relying on this release"),
        "{notes}"
    );
}

/// Runs the real assembly against the live repository's own `CHANGELOG.md` for the most recent
/// released tag, against a synthetic `dist/` shaped like today's actual two-target Linux matrix --
/// proof the derivation works end to end against real content, not only synthetic fixtures.
#[test]
fn real_changelog_produces_notes_for_the_current_release() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repository root");
    let temporary = tempfile::tempdir().unwrap();
    write_build_info(temporary.path(), "x86_64", "x86_64-unknown-linux-gnu");
    write_build_info(temporary.path(), "aarch64", "aarch64-unknown-linux-gnu");

    let notes = assemble(root, "0.22.0", temporary.path()).expect("0.22.0 has a changelog entry");
    assert!(notes.contains("## Prebuilt binaries"), "{notes}");
    assert!(
        notes.contains("Linux only (`aarch64`/`x86_64`)."),
        "{notes}"
    );
    assert!(notes.contains("## Release authority"), "{notes}");
}
