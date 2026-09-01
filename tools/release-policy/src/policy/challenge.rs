use std::collections::BTreeMap;

use serde_json::Value;

use super::signer::fingerprint;
use crate::time::parse_utc_second;

const FIELD_NAMES: [&str; 7] = [
    "repository",
    "primary_fingerprint",
    "role",
    "authority_revision",
    "nonce",
    "issued_at",
    "expires_at",
];

pub(super) fn validate(context: &Value, challenge: &[u8]) -> Option<&'static str> {
    let Some(object) = context.as_object() else {
        return Some("challenge-grammar-or-binding");
    };
    if object.len() != 5
        || ![
            "id",
            "observed_at",
            "observed_primary_fingerprint",
            "verifier_result",
            "expected_authority_revision",
        ]
        .iter()
        .all(|name| object.contains_key(*name))
    {
        return Some("challenge-grammar-or-binding");
    }
    let Ok(text) = std::str::from_utf8(challenge) else {
        return Some("challenge-grammar-or-binding");
    };
    if !text.is_ascii() || !text.ends_with('\n') || text.contains('\r') || text.ends_with("\n\n") {
        return Some("challenge-grammar-or-binding");
    }
    let lines: Vec<&str> = text[..text.len() - 1].split('\n').collect();
    if lines.len() != 8 || lines.first() != Some(&"prikk-release-signer-proof-v1") {
        return Some("challenge-grammar-or-binding");
    }
    let mut fields = BTreeMap::new();
    for (name, line) in FIELD_NAMES.iter().zip(lines.iter().skip(1)) {
        let prefix = format!("{name}=");
        let Some(value) = line.strip_prefix(&prefix) else {
            return Some("challenge-grammar-or-binding");
        };
        fields.insert(*name, value);
    }
    if fields.get("repository") != Some(&"https://github.com/prikk-vcs/prikk")
        || fields.get("role") != Some(&"official-release")
        || !fields
            .get("primary_fingerprint")
            .is_some_and(|value| fingerprint(value))
        || !fields
            .get("authority_revision")
            .is_some_and(|value| git_id(value))
        || !fields.get("nonce").is_some_and(|value| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        })
    {
        return Some("challenge-grammar-or-binding");
    }
    let issued = fields
        .get("issued_at")
        .and_then(|value| parse_utc_second(value));
    let expires = fields
        .get("expires_at")
        .and_then(|value| parse_utc_second(value));
    let observed = object
        .get("observed_at")
        .and_then(Value::as_str)
        .and_then(parse_utc_second);
    let (Some(issued), Some(expires), Some(observed)) = (issued, expires, observed) else {
        return Some("challenge-time-window");
    };
    if issued >= expires
        || expires - issued > 24 * 60 * 60
        || issued - observed > 5 * 60
        || observed >= expires
    {
        return Some("challenge-time-window");
    }
    if object
        .get("expected_authority_revision")
        .and_then(Value::as_str)
        != fields.get("authority_revision").copied()
        || object
            .get("observed_primary_fingerprint")
            .and_then(Value::as_str)
            != fields.get("primary_fingerprint").copied()
        || object
            .get("verifier_result")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        return Some("challenge-grammar-or-binding");
    }
    None
}

fn git_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
#[path = "challenge/tests.rs"]
mod tests;
