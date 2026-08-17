//! RFC 107 Stage 1: assemble a release's notes at publish time rather than restating facts in a
//! static file, which is what let the platform sentence in `.github/release-notes-template.md`
//! stay true for one release and false for the next two (`rfcs/accepted/107-release-distribution-\
//! surface.md` §0). Two pieces are derived, never hand-maintained:
//!
//! - **The version's own section**, extracted from `CHANGELOG.md`'s `## X.Y.Z — DATE` heading for
//!   the tag being released. A tag with no matching heading fails the release rather than
//!   publishing a page that says nothing (`RFC-107-stage-1-report-ruling-v1.md` §5).
//! - **The prebuilt-binary platform list**, derived from the `target:` field of every
//!   `*.build-info.txt` actually present in the publish job's `dist/` directory -- not from
//!   `release.yml`'s declared build matrix. **This is deliberate, not incidental**
//!   (`RFC-107-stage-1-report-ruling-v1.md` §2): the matrix is still a declaration, and a build job
//!   that silently produces nothing would leave it saying a target shipped when it did not.
//!   `dist/` is what is actually about to be published; deriving from it cannot make that same
//!   mistake one indirection removed. This is also why the grouping below classifies by operating
//!   system rather than assuming Linux-only: today's `dist/` holds only Linux artifacts, but a
//!   future `dist/` with macOS or Windows artifacts added by Stage 2 must produce a correct
//!   sentence without this module changing at all -- criterion 2's "impossible to reproduce, not
//!   merely fixed once."
//!
//! **The mutation-limit clause the previous static sentence carried is not reproduced here.** It
//! read *"repository mutation is Linux-only project-wide (DC-37), so this is not an
//! artifact-specific limitation"* -- true when written, false since 0.21.0, and carrying it forward
//! in derived form would still be defect 2 in gentler words on the release cut specifically to fix
//! it (`RFC-107-stage-1-report-ruling-v1.md` §4). The platform list states only what is true of the
//! artifacts themselves.

use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{Error, Result};

const RELEASE_AUTHORITY: &str = "## Release authority — read before relying on this release\n\
\n\
**This release does not pass the DC-35 signer-authority audit, and does not claim to.** The\n\
committed release-signer set (`release-signers.toml`) is empty and fail-closed, so no release\n\
currently satisfies that gate. A checksum published beside a binary on this page proves integrity of\n\
transport, not authority of origin. Verify what you obtain by content, not by release authority —\n\
see `prikk verify` and this project's\n\
[release-compatibility reference](https://nabbisen.github.io/prikk/reference/release-compatibility.html).";

/// Assemble the full notes body for `tag`, reading `CHANGELOG.md` under `root` and scanning
/// `dist_dir` for `*.build-info.txt` files. Fails if either source cannot support the claim being
/// made, rather than falling back to prose.
pub(crate) fn assemble(root: &Path, tag: &str, dist_dir: &Path) -> Result<String> {
    let changelog = std::fs::read_to_string(root.join("CHANGELOG.md"))?;
    let section = changelog_section(&changelog, tag).ok_or_else(|| {
        Error::new(format!(
            "no CHANGELOG.md entry for {tag} -- expected a `## {tag} — DATE` heading"
        ))
    })?;
    let platforms = platform_paragraph(dist_dir)?;
    Ok(format!("{section}\n\n{platforms}\n\n{RELEASE_AUTHORITY}\n"))
}

/// The version token is bounded on both sides -- `"## "` prefix, `" — "` suffix -- so a tag can
/// only match a heading that actually has both, never a legacy `## 0.1.0 PR-030`-style heading
/// (no em dash, no date) and never a prefix of a longer version string. Section content runs from
/// the line after the matched heading to the line before the next `## ` heading of any shape, or
/// end of file, with trailing blank lines trimmed.
fn changelog_section(changelog: &str, tag: &str) -> Option<String> {
    let mut lines = changelog.lines();
    for line in lines.by_ref() {
        let Some(rest) = line.strip_prefix("## ") else {
            continue;
        };
        let Some((version, _date)) = rest.split_once(" — ") else {
            continue;
        };
        if version != tag {
            continue;
        }
        let mut section = String::new();
        for line in lines.by_ref() {
            if line.starts_with("## ") {
                break;
            }
            section.push_str(line);
            section.push('\n');
        }
        return Some(section.trim().to_owned());
    }
    None
}

fn platform_paragraph(dist_dir: &Path) -> Result<String> {
    let mut targets = build_info_targets(dist_dir)?;
    targets.sort();
    targets.dedup();
    if targets.is_empty() {
        return Err(Error::new(format!(
            "{}: no *.build-info.txt files found -- nothing to describe as published",
            dist_dir.display()
        )));
    }
    let mut by_os: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for target in &targets {
        let (os, arch) = classify_target(target)?;
        let architectures = by_os.entry(os).or_default();
        if !architectures.contains(&arch) {
            architectures.push(arch);
        }
    }
    let summary = if let [(os, architectures)] = by_os.iter().collect::<Vec<_>>().as_slice() {
        format!("{os} only ({})", join_architectures(architectures))
    } else {
        by_os
            .iter()
            .map(|(os, architectures)| format!("{os} ({})", join_architectures(architectures)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    Ok(format!(
        "## Prebuilt binaries\n\
         \n\
         {summary}. Each archive contains the `prikk` binary, `LICENSE`, and a sibling `.sha256`\n\
         checksum plus `.build-info.txt` recording the exact toolchain and command used to build\n\
         it — reproduce with:\n\
         \n\
         ```sh\n\
         git checkout <tag> && cargo build -p prikk --release --target <triple> --locked\n\
         ```\n\
         \n\
         `cargo install prikk` remains the toolchain-based install path; these binaries are an\n\
         additional option, not a replacement."
    ))
}

fn join_architectures(architectures: &[String]) -> String {
    architectures
        .iter()
        .map(|architecture| format!("`{architecture}`"))
        .collect::<Vec<_>>()
        .join("/")
}

fn build_info_targets(dist_dir: &Path) -> Result<Vec<String>> {
    let read_dir = std::fs::read_dir(dist_dir)
        .map_err(|error| Error::new(format!("{}: {error}", dist_dir.display())))?;
    let mut targets = Vec::new();
    for entry in read_dir {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.ends_with(".build-info.txt") {
            continue;
        }
        let contents = std::fs::read_to_string(entry.path())?;
        let target = contents
            .lines()
            .find_map(|line| line.strip_prefix("target: "))
            .ok_or_else(|| Error::new(format!("{}: no `target:` line", entry.path().display())))?;
        targets.push(target.to_owned());
    }
    Ok(targets)
}

/// Every target triple prikk builds for is `ARCH-VENDOR-OS[-ABI]`; classifying by the vendor/OS
/// segment rather than an exact-triple lookup table means a new ABI variant of an already-supported
/// OS (an msvc/gnu Windows switch, for instance) still classifies correctly without this function
/// changing. An unrecognized triple fails closed rather than being silently mislabeled or dropped.
fn classify_target(target: &str) -> Result<(&'static str, String)> {
    let os = if target.contains("-unknown-linux-") {
        "Linux"
    } else if target.contains("-apple-darwin") {
        "macOS"
    } else if target.contains("-pc-windows-") {
        "Windows"
    } else {
        return Err(Error::new(format!("{target}: unrecognized target triple")));
    };
    let architecture = target
        .split('-')
        .next()
        .filter(|segment| !segment.is_empty())
        .ok_or_else(|| Error::new(format!("{target}: empty target triple")))?
        .to_owned();
    Ok((os, architecture))
}

#[cfg(test)]
#[path = "release_notes/tests.rs"]
mod tests;
