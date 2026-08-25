//! MSRV rise-policy gate (`rfcs/handoffs/msrv-policy/msrv-rise-policy-and-gate-handoff-v1.md`).
//!
//! `Cargo.toml`'s `workspace.package.rust-version` is the one declaration; everything else listed
//! in [`MARKERS`] is a transcription of it, and nothing before this gate checked that the copies
//! agreed with the original. Two spellings are both correct and deliberately not unified: the
//! manifest holds a two-component version (`"1.85"`), every toolchain pin and prose sentence holds
//! a three-component one (`1.85.0`). Comparison here is by parsed `(major, minor, patch)` triple,
//! with an absent patch component defaulting to `0`, never by string equality.
//!
//! Only the six live sites the handoff names are checked. Historical documents (`MILESTONES.md`,
//! `rfcs/README.md`, `rfcs/IMPLEMENTATION-STATUS.md`, everything under `rfcs/done/` and
//! `rfcs/handoffs/`) record what was true when they were written and must never be bound to the
//! current authority — `rfcs/EXECUTION-ORDER.md` is the one exception living under `rfcs/`, because
//! its §6 rule 9 is a gate command every increment is required to run verbatim, today, not a record
//! of the past.

use std::path::Path;

use regex::Regex;

use crate::error::{Error, Result};

const CARGO_TOML: &str = "Cargo.toml";
const CI_WORKFLOW: &str = ".github/workflows/ci.yml";
const DEVELOPMENT_GUIDE: &str = "docs/src/contributing/development.md";
const RELEASE_COMPATIBILITY: &str = "docs/src/reference/release-compatibility.md";
const EXECUTION_ORDER: &str = "rfcs/EXECUTION-ORDER.md";

struct Marker {
    file: &'static str,
    name: &'static str,
    pattern: &'static str,
}

/// One entry per live transcription site (handoff §2). Each pattern carries exactly one capturing
/// group: the version token to check against the authority.
const MARKERS: &[Marker] = &[
    Marker {
        file: CI_WORKFLOW,
        name: "toolchain-pin",
        pattern: r"dtolnay/rust-toolchain@(\d+\.\d+\.\d+)",
    },
    Marker {
        file: CI_WORKFLOW,
        name: "job-name",
        pattern: r"(?m)^\s*name:\s*msrv-(\d+\.\d+\.\d+)\s*$",
    },
    Marker {
        file: DEVELOPMENT_GUIDE,
        name: "prose",
        pattern: r"declares Rust (\d+(?:\.\d+){1,2}) as its minimum supported version",
    },
    Marker {
        file: DEVELOPMENT_GUIDE,
        name: "gate-command",
        pattern: r"cargo \+(\d+\.\d+\.\d+)",
    },
    Marker {
        file: RELEASE_COMPATIBILITY,
        name: "prose",
        pattern: r"declared minimum Rust version is exactly (\d+(?:\.\d+){1,2})",
    },
    Marker {
        file: RELEASE_COMPATIBILITY,
        name: "gate-command",
        pattern: r"cargo \+(\d+\.\d+\.\d+)",
    },
    Marker {
        file: EXECUTION_ORDER,
        name: "gate-command",
        pattern: r"cargo \+(\d+\.\d+\.\d+)",
    },
];

pub(super) fn check(root: &Path, errors: &mut Vec<String>) -> Result<()> {
    let authority = read_authority(root)?;
    for marker in MARKERS {
        let text = std::fs::read_to_string(root.join(marker.file))?;
        let regex = Regex::new(marker.pattern).map_err(|error| {
            Error::new(format!("msrv marker pattern {:?}: {error}", marker.name))
        })?;
        let mut found_any = false;
        for capture in regex.captures_iter(&text) {
            let Some(group) = capture.get(1) else {
                continue;
            };
            found_any = true;
            let token = group.as_str();
            match parse_version(token) {
                Some(version) if version == authority => {}
                _ => errors.push(format!(
                    "msrv-transcription-mismatch:{}:{}:{token}",
                    marker.file, marker.name
                )),
            }
        }
        if !found_any {
            errors.push(format!(
                "msrv-transcription-missing:{}:{}",
                marker.file, marker.name
            ));
        }
    }
    Ok(())
}

fn read_authority(root: &Path) -> Result<(u64, u64, u64)> {
    let text = std::fs::read_to_string(root.join(CARGO_TOML))?;
    let manifest: toml::Value = toml::from_str(&text)
        .map_err(|error| Error::new(format!("{CARGO_TOML} parse: {error}")))?;
    let raw = manifest
        .get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("package"))
        .and_then(toml::Value::as_table)
        .and_then(|table| table.get("rust-version"))
        .and_then(toml::Value::as_str)
        .ok_or_else(|| {
            Error::new(format!(
                "{CARGO_TOML}: workspace.package.rust-version missing"
            ))
        })?;
    parse_version(raw).ok_or_else(|| {
        Error::new(format!(
            "{CARGO_TOML}: rust-version {raw:?} is not a valid version"
        ))
    })
}

/// Parses `"1.85"` or `"1.85.0"` into `(1, 85, 0)`. Rejects anything with more than three
/// components or a non-numeric component; a missing patch component defaults to `0`.
fn parse_version(raw: &str) -> Option<(u64, u64, u64)> {
    let mut parts = raw.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = match parts.next() {
        Some(value) => value.parse().ok()?,
        None => 0,
    };
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

#[cfg(test)]
#[path = "msrv/tests.rs"]
mod tests;
