//! Extractor controls (handoff §6, items 1-5; item 6 lives in `profile/tests.rs` against the
//! committed prikk profile). Every fixture here is small, hand-written, and committed test
//! material -- never a corpus (handoff §2).

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use super::*;

fn context() -> ExtractionContext {
    ExtractionContext {
        source_repository: "fixture (self-test)".to_string(),
        revision: "0000000000000000000000000000000000000000".to_string(),
        extraction_commands: vec![
            "git log --pretty=format:'@@%H' --name-status --no-merges -n 5".to_string(),
        ],
        extraction_date: "2026-01-01".to_string(),
        rename_detection: true,
        builder_inputs: BuilderInputs {
            generator_seed: 1,
            author_key_id: "fixture-author".to_string(),
            author_seed_hex: "11".repeat(32),
            maintainer_key_id: "fixture-maintainer".to_string(),
            maintainer_seed_hex: "22".repeat(32),
            known_nondeterminism_risks: Vec::new(),
        },
    }
}

/// Hand-built log fixture with a deliberately uneven files-changed-per-commit shape (handoff §6
/// control 3: "include at least one commit touching many files and several touching one, so a
/// mean-only bug is visible"), all four `--name-status` categories the handoff names (control 4),
/// and a mix of paths touched once, twice, and four times (for the touches-per-path histogram).
/// Contains distinctive path strings (`marker-five.txt` etc.) so control 2 can assert none of them
/// survive into the profile.
const LOG_FIXTURE: &str = "\
@@aaa0000000000000000000000000000000000000
A\tmarker-five.txt
A\tmarker-six.txt
A\tmarker-seven.txt
A\tmarker-eight.txt
A\tmarker-nine.txt

@@aaa1111111111111111111111111111111111111
A\tsrc/one.txt
A\tsrc/two.txt
A\tsrc/three.txt

@@aaa2222222222222222222222222222222222222
M\tsrc/one.txt

@@aaa3333333333333333333333333333333333333
M\tsrc/one.txt
D\tsrc/two.txt
R100\tsrc/three.txt\tsrc/four.txt

@@aaa4444444444444444444444444444444444444
M\tsrc/one.txt
";

/// Hand-built ls-tree fixture: two files of the same size, one of a different size, one tree
/// entry and one submodule (`commit`) entry -- both of which must be skipped, not refused, per
/// this module's own doc. Also carries a distinctive path string for control 2.
const LS_TREE_FIXTURE: &str = "\
100644 blob 1111111111111111111111111111111111111111 100\tmarker-a.txt
100644 blob 2222222222222222222222222222222222222222 250\tmarker-b.txt
100644 blob 3333333333333333333333333333333333333333 100\tmarker-c.txt
040000 tree 4444444444444444444444444444444444444444 -\tmarker-subdir
160000 commit 5555555555555555555555555555555555555555 -\tmarker-submodule
";

fn extract() -> Profile {
    extract_profile(LOG_FIXTURE, LS_TREE_FIXTURE, context()).expect("fixture must extract cleanly")
}

/// Control 1: same input text twice -> byte-identical profile. Trivially true for a pure
/// implementation, which is the point -- it also catches a `HashMap`-iteration-order bug reaching
/// the output (this crate uses `BTreeMap` throughout for exactly this reason).
#[test]
fn control1_extraction_is_deterministic() {
    let first = extract();
    let second = extract();
    assert_eq!(first, second);
    let first_toml = toml::to_string_pretty(&first).unwrap();
    let second_toml = toml::to_string_pretty(&second).unwrap();
    assert_eq!(first_toml, second_toml);
}

/// Control 2: RFC 139 §4's prohibition. A fixture containing distinctive path strings must
/// produce a profile containing none of them -- asserted on the actual strings, not on shape.
#[test]
fn control2_no_source_paths_survive_into_the_profile() {
    let profile = extract();
    let rendered = toml::to_string_pretty(&profile).unwrap();
    for marker in [
        "marker-five.txt",
        "marker-six.txt",
        "marker-seven.txt",
        "marker-eight.txt",
        "marker-nine.txt",
        "src/one.txt",
        "src/two.txt",
        "src/three.txt",
        "src/four.txt",
        "marker-a.txt",
        "marker-b.txt",
        "marker-c.txt",
        "marker-subdir",
        "marker-submodule",
    ] {
        assert!(
            !rendered.contains(marker),
            "profile must not contain the source path {marker:?}:\n{rendered}"
        );
    }
}

/// Control 3: the histograms are histograms, checked against hand-computed values, not a
/// plausible-looking mean. `LOG_FIXTURE` has 5 commits touching (5, 3, 1, 3, 1) files.
#[test]
fn control3_histograms_match_hand_computed_values() {
    let profile = extract();
    assert_eq!(profile.shape.commit_count, 5);

    let mut expected_files_changed = BTreeMap::new();
    expected_files_changed.insert("1".to_string(), 2); // commits touching 1 file: aaa2, aaa4
    expected_files_changed.insert("3".to_string(), 2); // commits touching 3 files: aaa1, aaa3
    expected_files_changed.insert("5".to_string(), 1); // commits touching 5 files: aaa0
    assert_eq!(
        profile.shape.files_changed_per_commit,
        expected_files_changed
    );

    // Touches: one.txt=4 (A,M,M,M), two.txt=2 (A,D), three.txt=2 (A,R-old), four.txt=1 (R-new),
    // and the five marker-* paths from aaa0 each touched exactly once.
    assert_eq!(profile.shape.distinct_paths, 9);
    let mut expected_path_touches = BTreeMap::new();
    expected_path_touches.insert("1".to_string(), 6); // four.txt + 5 marker-* paths
    expected_path_touches.insert("2".to_string(), 2); // two.txt, three.txt
    expected_path_touches.insert("4".to_string(), 1); // one.txt
    assert_eq!(profile.shape.path_touches, expected_path_touches);

    let mut expected_file_sizes = BTreeMap::new();
    expected_file_sizes.insert("100".to_string(), 2); // marker-a.txt, marker-c.txt
    expected_file_sizes.insert("250".to_string(), 1); // marker-b.txt
    assert_eq!(profile.shape.file_sizes, expected_file_sizes);
}

/// Control 4: the operation-kind mix is real. `LOG_FIXTURE` contains `A`, `M`, `D`, and `R` lines;
/// all four categories must be non-zero. An extractor that silently dropped `R` (the field
/// `--name-only` never had at all) would look correct against an `A`/`M`-only fixture -- this one
/// is not `A`/`M`-only.
#[test]
fn control4_operation_kind_mix_has_four_nonzero_categories() {
    let profile = extract();
    let mix = profile.shape.operation_kind_mix;
    assert_eq!(mix.added, 8, "5 marker-* + one/two/three from aaa1");
    assert_eq!(mix.modified, 3, "aaa2, aaa3, aaa4 each modify one.txt");
    assert_eq!(mix.deleted, 1, "aaa3 deletes two.txt");
    assert_eq!(mix.renamed, 1, "aaa3 renames three.txt to four.txt");
    assert_eq!(mix.copied, 0);
    assert_eq!(mix.type_changed, 0);
}

/// `C` (copy) and `T` (type-changed) are recognized too, not just the four `LOG_FIXTURE` exercises
/// above -- a minimal, separate fixture, since neither appears in ordinary prikk-authored history
/// (`CreateSymlink`/copy detection are not things `commit` produces) and forcing them into the
/// main fixture would make its hand-computed histograms harder to verify by eye.
#[test]
fn recognizes_copy_and_type_changed_status_letters() {
    let log = "\
@@bbb0000000000000000000000000000000000000
C100\tsrc/original.txt\tsrc/copy.txt
T\tsrc/became-a-symlink.txt
";
    let profile = extract_profile(log, "", context()).unwrap();
    assert_eq!(profile.shape.operation_kind_mix.copied, 1);
    assert_eq!(profile.shape.operation_kind_mix.type_changed, 1);
}

/// Control 5: malformed input is refused, not absorbed -- three shapes, each naming its own line.
#[test]
fn control5_malformed_log_input_is_refused_naming_the_line() {
    // Truncated: the header line carries no hash at all.
    let truncated = extract_profile("@@\nA\tfile.txt\n", "", context());
    assert_eq!(
        truncated,
        Err(ExtractError::Log {
            line: 1,
            message: "commit header carries no commit hash".to_string(),
        })
    );

    // A commit header with no file lines before the next header.
    let empty_commit = extract_profile(
        "@@aaa0000000000000000000000000000000000000\n@@aaa1111111111111111111111111111111111111\nA\tfile.txt\n",
        "",
        context(),
    );
    assert_eq!(
        empty_commit,
        Err(ExtractError::Log {
            line: 1,
            message: "commit header has no file lines before the next header".to_string(),
        })
    );

    // A commit header with no file lines before end of input.
    let empty_at_eof = extract_profile(
        "@@aaa0000000000000000000000000000000000000\n",
        "",
        context(),
    );
    assert_eq!(
        empty_at_eof,
        Err(ExtractError::Log {
            line: 1,
            message: "commit header has no file lines before end of input".to_string(),
        })
    );

    // An unrecognized status letter.
    let bad_status = extract_profile(
        "@@aaa0000000000000000000000000000000000000\nX\tfile.txt\n",
        "",
        context(),
    );
    assert_eq!(
        bad_status,
        Err(ExtractError::Log {
            line: 2,
            message: "unrecognized status letter 'X'".to_string(),
        })
    );

    // A file line before any commit header at all.
    let orphan_file_line = extract_profile("A\tfile.txt\n", "", context());
    assert_eq!(
        orphan_file_line,
        Err(ExtractError::Log {
            line: 1,
            message: "file line appears before any commit header".to_string(),
        })
    );
}

/// Control 5, the ls-tree half: a malformed size field and an unrecognized object type are both
/// refused, naming their own line.
#[test]
fn control5_malformed_ls_tree_input_is_refused_naming_the_line() {
    let bad_size = extract_profile(
        "@@aaa0000000000000000000000000000000000000\nA\tfile.txt\n",
        "100644 blob 1111111111111111111111111111111111111111 not-a-number\tfile.txt\n",
        context(),
    );
    assert_eq!(
        bad_size,
        Err(ExtractError::LsTree {
            line: 1,
            message: "size field \"not-a-number\" is not a non-negative integer".to_string(),
        })
    );

    let bad_type = extract_profile(
        "@@aaa0000000000000000000000000000000000000\nA\tfile.txt\n",
        "100644 symlink 1111111111111111111111111111111111111111 4\tfile.txt\n",
        context(),
    );
    assert_eq!(
        bad_type,
        Err(ExtractError::LsTree {
            line: 1,
            message: "unrecognized object type \"symlink\"".to_string(),
        })
    );

    let missing_tab = extract_profile(
        "@@aaa0000000000000000000000000000000000000\nA\tfile.txt\n",
        "100644 blob 1111111111111111111111111111111111111111 4 file.txt\n",
        context(),
    );
    assert_eq!(
        missing_tab,
        Err(ExtractError::LsTree {
            line: 1,
            message: "no tab separating metadata from path".to_string(),
        })
    );
}

/// `ExtractError`'s `Display` names the source and the line, for a human reading a failed run.
#[test]
fn extract_error_display_names_source_and_line() {
    let log_error = ExtractError::Log {
        line: 3,
        message: "example".to_string(),
    };
    assert_eq!(log_error.to_string(), "log text, line 3: example");
    let ls_tree_error = ExtractError::LsTree {
        line: 7,
        message: "example".to_string(),
    };
    assert_eq!(ls_tree_error.to_string(), "ls-tree text, line 7: example");
}
