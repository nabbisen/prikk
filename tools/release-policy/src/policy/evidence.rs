mod governance;
mod sequence;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::json;
use crate::oracle::{Case, Oracle};
use crate::schema::SchemaProfile;
use crate::time::parse_utc_second;

const CRATE_ORDER: [(&str, u64); 7] = [
    ("prikk-error", 1),
    ("prikk-hash", 1),
    ("prikk-crypto", 2),
    ("prikk-object", 2),
    ("prikk-replay", 3),
    ("prikk-store", 4),
    ("prikk", 5),
];

pub(super) fn evaluate<'a>(
    oracle: &Oracle,
    case: &Case,
    schema: &SchemaProfile,
) -> Result<(&'a str, Option<&'a str>, Option<&'a str>, &'a str)> {
    let parsed = json::parse(oracle.input(case, "fixture-table")?)
        .map_err(|error| Error::new(format!("release evidence parsed input: {error}")))?;
    let current = parsed
        .get("current")
        .ok_or_else(|| Error::new("release evidence case missing current document"))?;
    let prior = parsed.get("prior").filter(|value| !value.is_null());
    let structural = schema.is_valid(current) && prior.is_none_or(|value| schema.is_valid(value));
    if !structural {
        return Ok((
            "invalid",
            Some("invalid"),
            Some("not-run"),
            "schema-instance",
        ));
    }
    let mut reasons = Vec::new();
    if let Some(reason) = single_reason(current) {
        reasons.push(reason);
    }
    if let Some(prior) = prior {
        if let Some(reason) = single_reason(prior) {
            reasons.push(reason);
        }
        if let Some(reason) = sequence::reason(
            prior,
            current,
            oracle.input(case, "prior-snapshot")?,
            oracle.input(case, "current-snapshot")?,
        ) {
            reasons.push(reason);
        }
    }
    reasons.sort_by_key(|reason| reason_rank(reason));
    let reason = reasons.first().copied().unwrap_or("none");
    Ok((
        if reasons.is_empty() {
            "valid"
        } else {
            "invalid"
        },
        Some("valid"),
        Some(if reasons.is_empty() {
            "valid"
        } else {
            "invalid"
        }),
        reason,
    ))
}

pub(super) fn single_reason(snapshot: &Value) -> Option<&'static str> {
    if let Some(reason) = governance::reason(snapshot.get("governance")) {
        return Some(reason);
    }
    if snapshot.get("overall_status").and_then(Value::as_str) == Some("complete")
        && governance::hold_active(snapshot.get("governance"))
    {
        return Some("governance-review-or-hold");
    }
    if tag_or_artifact_invalid(snapshot) {
        return Some("evidence-tag-or-artifact");
    }
    if attempts_invalid(snapshot) {
        return Some("evidence-transition-or-attempt-prefix");
    }
    if snapshot.get("overall_status").and_then(Value::as_str) == Some("complete")
        && !complete_valid(snapshot)
    {
        return Some("evidence-tag-or-artifact");
    }
    None
}

fn tag_or_artifact_invalid(snapshot: &Value) -> bool {
    let Some(version) = snapshot.get("version").and_then(Value::as_str) else {
        return true;
    };
    let Some(tag) = snapshot.get("tag") else {
        return true;
    };
    if tag.get("name").and_then(Value::as_str) != Some(version)
        || !tag_verification_valid(tag.get("release_tag_verification"))
    {
        return true;
    }
    let Some(archive) = snapshot.get("archive") else {
        return true;
    };
    let expected_archive = format!("prikk-v{version}.tar.gz");
    if archive.get("name").and_then(Value::as_str) != Some(expected_archive.as_str())
        || archive.get("checksum_name").and_then(Value::as_str)
            != Some(format!("{expected_archive}.sha256").as_str())
    {
        return true;
    }
    let Some(crates) = value_array(snapshot, "crates") else {
        return true;
    };
    if crates.len() != CRATE_ORDER.len() {
        return true;
    }
    crates.iter().zip(CRATE_ORDER).any(|(item, (name, level))| {
        item.get("name").and_then(Value::as_str) != Some(name)
            || item.get("publish_level").and_then(Value::as_u64) != Some(level)
            || item.get("version").and_then(Value::as_str) != Some(version)
            || item
                .get("exact_internal_requirements")
                .and_then(Value::as_bool)
                != Some(true)
            || !crate_checksum_state_valid(item)
    })
}

fn tag_verification_valid(value: Option<&Value>) -> bool {
    let Some(value) = value else {
        return false;
    };
    let status = value.get("status").and_then(Value::as_str);
    let details = [
        "signer_primary_fingerprint",
        "authority_path",
        "authority_blob_id",
        "verifier_result",
    ];
    match status {
        Some("not-observed") => details
            .iter()
            .all(|field| value.get(field).is_some_and(Value::is_null)),
        Some("verified") => details
            .iter()
            .all(|field| value.get(field).is_some_and(|item| !item.is_null())),
        Some("failed") => ["authority_path", "authority_blob_id", "verifier_result"]
            .iter()
            .all(|field| value.get(field).is_some_and(|item| !item.is_null())),
        _ => false,
    }
}

fn crate_checksum_state_valid(value: &Value) -> bool {
    let checksums = [
        value.get("staged_sha256"),
        value.get("registry_checksum"),
        value.get("fetched_sha256"),
    ];
    match value.get("checksum_equality").and_then(Value::as_str) {
        Some("match") => {
            checksums
                .iter()
                .all(|item| item.is_some_and(|value| !value.is_null()))
                && checksums[0] == checksums[1]
                && checksums[1] == checksums[2]
        }
        Some("mismatch") => {
            checksums
                .iter()
                .all(|item| item.is_some_and(|value| !value.is_null()))
                && !(checksums[0] == checksums[1] && checksums[1] == checksums[2])
        }
        Some("not-observed") => true,
        _ => false,
    }
}

fn attempts_invalid(snapshot: &Value) -> bool {
    let Some(attempts) = value_array(snapshot, "attempts") else {
        return true;
    };
    let mut previous = None;
    for (index, attempt) in attempts.iter().enumerate() {
        if attempt.get("sequence").and_then(Value::as_u64) != Some(index as u64 + 1) {
            return true;
        }
        let Some(time) = attempt
            .get("time")
            .and_then(Value::as_str)
            .and_then(parse_utc_second)
        else {
            return true;
        };
        if previous.is_some_and(|value| value > time) {
            return true;
        }
        previous = Some(time);
    }
    false
}

fn complete_valid(snapshot: &Value) -> bool {
    let verification = snapshot
        .get("tag")
        .and_then(|tag| tag.get("release_tag_verification"));
    if verification
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
        != Some("verified")
    {
        return false;
    }
    let Some(archive) = snapshot.get("archive") else {
        return false;
    };
    if archive.get("archive_attached").and_then(Value::as_bool) != Some(true)
        || archive.get("checksum_attached").and_then(Value::as_bool) != Some(true)
        || archive.get("checksum_grammar").and_then(Value::as_str) != Some("valid")
        || archive.get("archive_root").and_then(Value::as_str) != Some("valid")
    {
        return false;
    }
    if value_array(snapshot, "crates").is_none_or(|crates| {
        crates.iter().any(|item| {
            !crate_checksum_state_valid(item)
                || item.get("checksum_equality").and_then(Value::as_str) != Some("match")
                || item.get("published").and_then(Value::as_bool) != Some(true)
                || item.get("registry_visible").and_then(Value::as_bool) != Some(true)
        })
    }) {
        return false;
    }
    let page = snapshot
        .get("release_page")
        .and_then(|value| value.get("status"));
    if page.and_then(Value::as_str) != Some("published") {
        return false;
    }
    let pages = snapshot.get("pages");
    match pages
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
    {
        Some("deployed") => {
            pages.and_then(|value| value.get("deployed_commit"))
                == snapshot
                    .get("tag")
                    .and_then(|value| value.get("peeled_commit"))
        }
        Some("inapplicable") => pages
            .and_then(|value| value.get("inapplicable_ruling"))
            .and_then(Value::as_str)
            .is_some_and(|ruling| !ruling.is_empty()),
        _ => false,
    }
}

pub(super) fn value_array<'a>(value: &'a Value, field: &str) -> Option<&'a [Value]> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
}

fn reason_rank(reason: &str) -> usize {
    [
        "manifest-contract",
        "input-identity",
        "json-syntax-or-duplicate-name",
        "schema-profile-or-compilation",
        "schema-instance",
        "authority-grammar",
        "challenge-grammar-or-binding",
        "challenge-time-window",
        "governance-transition-or-proof",
        "governance-review-or-hold",
        "release-state",
        "evidence-byte-identity-or-link",
        "evidence-transition-or-attempt-prefix",
        "evidence-tag-or-artifact",
        "evidence-completion",
        "none",
    ]
    .iter()
    .position(|item| *item == reason)
    .unwrap_or(usize::MAX)
}

#[cfg(test)]
#[path = "evidence/tests.rs"]
mod tests;
