//! RFC 127: every released tag keeps exactly one matching `## X.Y.Z — DATE` heading in
//! `CHANGELOG.md`, checked in the standing gate set rather than only at release time.
//!
//! `release_notes::assemble` (`release_notes.rs`) reads only the section for the tag *being*
//! released — its guarantee is "the version being released has a heading," never "every
//! previously released version still does." That blind spot is exactly where the real defect
//! landed: `5964ad6` ("release: bump workspace to 0.24.0 and add the changelog entry") *replaced*
//! `## 0.23.0 — 2026-08-23` with `## 0.24.0 — ...` instead of inserting above it, and nothing
//! caught it because every release since has only ever read its own section.
//!
//! **Fails loudly on an empty tag list, per RFC 127 §3, rather than passing vacuously** — a gate
//! that cannot fail here would have let the actual defect through regardless of the check below.
//! This is why CI must fetch tags at checkout (`ci.yml`'s own `fetch-tags: true` on every job that
//! runs `cargo test --workspace`, which is what exercises this module): a checkout with no tags
//! must read as "this gate could not run," not as "there was nothing to check."

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use super::{BoundaryError, push};
use crate::error::Result;

/// **Ruled by the architect, RFC 127 §3.3, 2026-09-01: keep this list.** `0.0.1` (no heading of any
/// shape) and `0.1.1` (`## 0.1.1 Housekeeping` -- no ` — DATE` suffix) predate the
/// `## X.Y.Z — DATE` convention, which begins holding at `0.1.2`. The gate exists to catch a
/// regression *against* that convention; these two tags were never in its shape, so there is
/// nothing for the gate to regress on. This is this project's own idiom for exactly this shape
/// (`UNSAFE_EXEMPT_CRATES`, `DECLARED_UNDOCUMENTED`, `RFC114_ADMITTED_BUT_UNWRITTEN`), and entries
/// here are pre-convention tags only -- never a version range, never "everything before X".
///
/// **Self-guarding, per the ruling:** [`check`] verifies each entry here still genuinely lacks a
/// conforming heading, and fails if one does not -- an exemption that has become untrue must break
/// the build until someone deletes it, per `unsafe_boundary.rs`'s own standard: "a control the
/// controlled party can silently remove is a convention, not a control."
const LEGACY_TAGS_WITHOUT_DATED_HEADINGS: &[&str] = &["0.0.1", "0.1.1"];

/// A released tag's own shape (`prikk-release-tag-convention`): unprefixed `X.Y.Z`, three
/// dot-separated non-negative integers, nothing else — this project has never used another tag
/// shape. Anything not matching this is silently excluded from the gate, not failed on: a
/// non-version tag (should one ever exist — an RFC marker, say) is not a released version and has
/// no changelog heading to keep.
fn is_release_tag(name: &str) -> bool {
    let mut parts = name.split('.');
    let Some(major) = parts.next() else {
        return false;
    };
    let Some(minor) = parts.next() else {
        return false;
    };
    let Some(patch) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    [major, minor, patch].iter().all(|component| {
        !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
    })
}

/// Every `## X.Y.Z — DATE` heading's version token, in file order — the same bounded-both-sides
/// match (`"## "` prefix, `" — "` suffix) `release_notes::changelog_section` already uses, so this
/// gate can never disagree with what a real release actually reads. Duplicates are kept, not
/// deduplicated: a duplicate heading for one tag is itself a defect this gate must report, via the
/// count check in [`check`] below.
fn changelog_headings(changelog: &str) -> Vec<&str> {
    changelog
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .filter_map(|rest| rest.split_once(" — "))
        .map(|(version, _date)| version)
        .collect()
}

pub(super) fn check(root: &Path, errors: &mut Vec<BoundaryError>) -> Result<()> {
    let output = Command::new("git")
        .args(["tag", "--list"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        push(
            errors,
            "changelog-history",
            format!(
                "git tag --list failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        );
        return Ok(());
    }
    let tags: BTreeSet<&str> = std::str::from_utf8(&output.stdout)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|name| is_release_tag(name))
        .filter(|name| !LEGACY_TAGS_WITHOUT_DATED_HEADINGS.contains(name))
        .collect();

    if tags.is_empty() {
        push(
            errors,
            "changelog-history",
            "no release tags found in this checkout (`git tag --list` matched none shaped like \
             X.Y.Z) -- this gate cannot verify changelog coverage without tags; if this is CI, \
             confirm the checkout step fetches them (`fetch-tags: true`)"
                .to_string(),
        );
        return Ok(());
    }

    let changelog = std::fs::read_to_string(root.join("CHANGELOG.md"))?;
    let headings = changelog_headings(&changelog);

    for exempt_tag in LEGACY_TAGS_WITHOUT_DATED_HEADINGS {
        let count = headings
            .iter()
            .filter(|heading| **heading == *exempt_tag)
            .count();
        if count != 0 {
            push(
                errors,
                "changelog-history",
                format!(
                    "{exempt_tag}: exemption in LEGACY_TAGS_WITHOUT_DATED_HEADINGS is stale -- \
                     CHANGELOG.md now has {count} `## {exempt_tag} — DATE` heading(s); remove this \
                     tag from the exemption list"
                ),
            );
        }
    }

    for tag in &tags {
        let count = headings.iter().filter(|heading| **heading == *tag).count();
        if count != 1 {
            push(
                errors,
                "changelog-history",
                format!(
                    "{tag}: {count} `## {tag} — DATE` heading(s) in CHANGELOG.md (expected \
                     exactly 1)"
                ),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "changelog_history/tests.rs"]
mod tests;
