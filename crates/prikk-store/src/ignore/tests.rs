//! Unit coverage for the pure matching/pruning logic. `IgnoreRules::load`'s own file-reading and
//! fail-closed behavior is covered end-to-end in `worktree_patch/tests.rs` and
//! `worktree_status/tests.rs`, driven against real repositories through the same call sites
//! production uses. The separator-safety invariant both call sites depend on
//! (`crate::path::pathbuf_to_slash_string`) is tested in `path/tests.rs`, next to the function
//! itself.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeSet;

use super::{IgnoreRules, should_skip_discovery};

fn rules(lines: &[&str]) -> IgnoreRules {
    IgnoreRules {
        prefixes: lines.iter().map(|line| (*line).to_string()).collect(),
    }
}

#[test]
fn a_rule_matches_its_own_exact_path() {
    let rules = rules(&["target"]);
    assert!(rules.is_ignored("target"));
}

#[test]
fn a_rule_matches_everything_nested_under_it() {
    let rules = rules(&["target"]);
    assert!(rules.is_ignored("target/debug/build"));
}

/// The over-matching bug a bare string prefix would have: `"target"` must not match a sibling whose
/// name merely starts with the same characters.
#[test]
fn a_rule_does_not_match_a_same_prefixed_sibling() {
    let rules = rules(&["target"]);
    assert!(!rules.is_ignored("target2"));
    assert!(!rules.is_ignored("targetfoo"));
    assert!(!rules.is_ignored("targetfoo/inner.txt"));
}

#[test]
fn an_unrelated_path_is_not_ignored() {
    let rules = rules(&["target"]);
    assert!(!rules.is_ignored("src/lib.rs"));
}

#[test]
fn a_nested_rule_matches_only_under_its_own_directory() {
    let rules = rules(&["docs/generated"]);
    assert!(rules.is_ignored("docs/generated"));
    assert!(rules.is_ignored("docs/generated/index.html"));
    assert!(!rules.is_ignored("docs/other"));
    assert!(!rules.is_ignored("docs"));
}

#[test]
fn should_skip_discovery_is_false_for_a_path_no_rule_covers() {
    let rules = rules(&["target"]);
    let tracked = BTreeSet::new();
    assert!(!should_skip_discovery(&rules, &tracked, "src/lib.rs"));
}

#[test]
fn should_skip_discovery_is_true_for_an_ignored_untracked_path() {
    let rules = rules(&["node_modules"]);
    let tracked = BTreeSet::new();
    assert!(should_skip_discovery(&rules, &tracked, "node_modules"));
    assert!(should_skip_discovery(
        &rules,
        &tracked,
        "node_modules/.bin/tool"
    ));
}

/// §4.4's constraint, at the unit level: a path already tracked must never be hidden by a rule that
/// now covers it, even though the rule textually matches.
#[test]
fn should_skip_discovery_is_false_for_an_already_tracked_exact_path() {
    let rules = rules(&["config.toml"]);
    let mut tracked = BTreeSet::new();
    tracked.insert("config.toml".to_string());
    assert!(!should_skip_discovery(&rules, &tracked, "config.toml"));
}

/// The directory-pruning case: an ignored directory must not be skipped wholesale if a tracked file
/// happens to live inside it -- otherwise that file would silently vanish from discovery too.
#[test]
fn should_skip_discovery_is_false_for_an_ignored_directory_with_a_tracked_descendant() {
    let rules = rules(&["vendor"]);
    let mut tracked = BTreeSet::new();
    tracked.insert("vendor/keep.rs".to_string());
    assert!(!should_skip_discovery(&rules, &tracked, "vendor"));
    // The tracked descendant itself is obviously never "skipped from discovery" either.
    assert!(!should_skip_discovery(&rules, &tracked, "vendor/keep.rs"));
    // A different, untracked file in that same ignored directory is still skipped.
    assert!(should_skip_discovery(
        &rules,
        &tracked,
        "vendor/generated.rs"
    ));
}

#[test]
fn a_tracked_path_under_a_different_top_level_directory_does_not_protect_an_ignored_one() {
    let rules = rules(&["build"]);
    let mut tracked = BTreeSet::new();
    tracked.insert("build-notes.md".to_string());
    // Component-aware matching: "build-notes.md" does not start with "build/", so it must not be
    // mistaken for a descendant of the "build" rule.
    assert!(should_skip_discovery(&rules, &tracked, "build"));
}
