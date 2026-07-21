use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthorityDocument {
    schema_version: u64,
    authorized_primary_fingerprints: Vec<String>,
}

pub(super) fn authority_valid(bytes: &[u8]) -> bool {
    let Ok(document) = std::str::from_utf8(bytes) else {
        return false;
    };
    let Ok(value) = toml::from_str::<AuthorityDocument>(document) else {
        return false;
    };
    if value.schema_version != 1 {
        return false;
    }
    let fingerprints: Vec<&str> = value
        .authorized_primary_fingerprints
        .iter()
        .map(String::as_str)
        .collect();
    fingerprints_valid(&fingerprints)
}

pub(super) fn transaction_reason(case: &Value) -> Option<&'static str> {
    let Some(object) = case.as_object() else {
        return Some("governance-transition-or-proof");
    };
    let required = [
        "id",
        "old",
        "new",
        "declared_type",
        "proof",
        "approvals",
        "expected",
    ];
    if object.len() != required.len() || !required.iter().all(|name| object.contains_key(*name)) {
        return Some("governance-transition-or-proof");
    }
    let Some(old) = string_array(object.get("old")) else {
        return Some("governance-transition-or-proof");
    };
    let Some(new) = string_array(object.get("new")) else {
        return Some("governance-transition-or-proof");
    };
    if !fingerprints_valid(&old) || !fingerprints_valid(&new) {
        return Some("governance-transition-or-proof");
    }
    let introduced: Vec<&str> = new
        .iter()
        .copied()
        .filter(|item| !old.contains(item))
        .collect();
    let removed: Vec<&str> = old
        .iter()
        .copied()
        .filter(|item| !new.contains(item))
        .collect();
    let actual = transaction_type(&old, &new, &introduced, &removed);
    if actual != object.get("declared_type").and_then(Value::as_str) {
        return Some("governance-transition-or-proof");
    }
    if !approvals_valid(object.get("approvals")) {
        return Some("governance-review-or-hold");
    }
    if !proof_valid(object.get("proof"), &introduced) {
        return Some("governance-transition-or-proof");
    }
    None
}

pub(super) fn fingerprints_valid(values: &[&str]) -> bool {
    let sorted: BTreeSet<&str> = values.iter().copied().collect();
    values.iter().all(|value| fingerprint(value))
        && sorted.len() == values.len()
        && sorted.into_iter().eq(values.iter().copied())
}

pub(super) fn fingerprint(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
}

fn string_array(value: Option<&Value>) -> Option<Vec<&str>> {
    value?
        .as_array()?
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
}

fn transaction_type<'a>(
    old: &[&str],
    new: &[&str],
    introduced: &[&str],
    removed: &[&str],
) -> Option<&'a str> {
    if old.is_empty() && !introduced.is_empty() && removed.is_empty() {
        Some("bootstrap")
    } else if !introduced.is_empty() && removed.is_empty() {
        Some("addition")
    } else if !introduced.is_empty() && !removed.is_empty() {
        Some("replacement")
    } else if introduced.is_empty() && !removed.is_empty() {
        Some("removal-only")
    } else if old == new {
        Some("classification-only")
    } else {
        None
    }
}

fn approvals_valid(value: Option<&Value>) -> bool {
    let Some(approvals) = value.and_then(Value::as_array) else {
        return false;
    };
    if approvals.len() != 2 {
        return false;
    }
    let mut people = BTreeSet::new();
    let mut roles = BTreeSet::new();
    for approval in approvals {
        let Some(object) = approval.as_object() else {
            return false;
        };
        if object.len() != 2 {
            return false;
        }
        let Some(person) = object.get("person").and_then(Value::as_str) else {
            return false;
        };
        let Some(role) = object.get("role").and_then(Value::as_str) else {
            return false;
        };
        if person.is_empty() || person.starts_with("automation-") {
            return false;
        }
        people.insert(person);
        roles.insert(role);
    }
    people.len() == 2 && roles == BTreeSet::from(["architect-security", "maintainer-administrator"])
}

fn proof_valid(value: Option<&Value>, introduced: &[&str]) -> bool {
    let Some(proof) = value.and_then(Value::as_object) else {
        return false;
    };
    if proof.len() != 3 {
        return false;
    }
    let Some(records) = proof.get("introduced_signers").and_then(Value::as_array) else {
        return false;
    };
    if introduced.is_empty() {
        return proof.get("state").and_then(Value::as_str) == Some("not-applicable")
            && proof
                .get("reason")
                .and_then(Value::as_str)
                .is_some_and(|reason| !reason.is_empty())
            && records.is_empty();
    }
    if proof.get("state").and_then(Value::as_str) != Some("verified")
        || !proof.get("reason").is_some_and(Value::is_null)
    {
        return false;
    }
    let fingerprints: Option<Vec<&str>> = records
        .iter()
        .map(|record| {
            let object = record.as_object()?;
            if object.len() != 2
                || object
                    .get("verifier_result")
                    .and_then(Value::as_str)
                    .is_none_or(str::is_empty)
            {
                return None;
            }
            object.get("primary_fingerprint").and_then(Value::as_str)
        })
        .collect();
    fingerprints.is_some_and(|values| values == introduced)
}

#[cfg(test)]
#[path = "signer/tests.rs"]
mod tests;
