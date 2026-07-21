use std::collections::BTreeSet;

use serde_json::Value;

use crate::time::parse_utc_second;

use super::super::signer::fingerprints_valid;

pub(super) fn reason(governance: Option<&Value>) -> Option<&'static str> {
    let governance = governance?;
    if governance.is_null() {
        return None;
    }
    let Some(object) = governance.as_object() else {
        return Some("governance-transition-or-proof");
    };
    let Some(old) = string_array(object.get("old_authorized_fingerprints")) else {
        return Some("governance-transition-or-proof");
    };
    let Some(new) = string_array(object.get("new_authorized_fingerprints")) else {
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
    let derived = transaction_type(&old, &new, &introduced, &removed);
    let transaction = object.get("transaction_type").and_then(Value::as_str);
    if transaction != derived {
        return Some("governance-transition-or-proof");
    }
    let old_blob = object.get("old_authority_blob_id");
    let new_blob = object.get("new_authority_blob_id");
    if (transaction == Some("classification-only") && old_blob != new_blob)
        || (transaction != Some("classification-only") && old_blob == new_blob)
    {
        return Some("governance-transition-or-proof");
    }
    if !proof_valid(object.get("authority_proof"), &introduced) {
        return Some("governance-transition-or-proof");
    }
    if !approvals_valid(object.get("approvals")) {
        return Some("governance-review-or-hold");
    }
    let started = object
        .get("hold_started_at")
        .and_then(Value::as_str)
        .and_then(parse_utc_second);
    let Some(started) = started else {
        return Some("governance-review-or-hold");
    };
    let ended = object.get("hold_ended_at");
    let lift = object.get("hold_lift");
    if ended.is_none_or(Value::is_null) {
        if lift.is_some_and(|value| !value.is_null()) {
            return Some("governance-review-or-hold");
        }
        return None;
    }
    let Some(ended) = ended.and_then(Value::as_str).and_then(parse_utc_second) else {
        return Some("governance-review-or-hold");
    };
    if lift.is_none_or(Value::is_null) || ended - started < 72 * 60 * 60 {
        return Some("governance-review-or-hold");
    }
    if transaction == Some("classification-only")
        && object.get("classification").is_none_or(Value::is_null)
    {
        return Some("governance-transition-or-proof");
    }
    None
}

pub(super) fn hold_active(governance: Option<&Value>) -> bool {
    governance
        .filter(|value| !value.is_null())
        .and_then(Value::as_object)
        .and_then(|object| object.get("hold_ended_at"))
        .is_some_and(Value::is_null)
}

pub(super) fn progression_reason(old: Option<&Value>, new: Option<&Value>) -> Option<&'static str> {
    let old = old.filter(|value| !value.is_null())?;
    let Some(new) = new.filter(|value| !value.is_null()) else {
        return Some("governance-transition-or-proof");
    };
    let (Some(old), Some(new)) = (old.as_object(), new.as_object()) else {
        return Some("governance-transition-or-proof");
    };
    for (field, old_value) in old {
        let fillable = matches!(
            field.as_str(),
            "classification" | "hold_ended_at" | "hold_lift"
        );
        let Some(new_value) = new.get(field) else {
            return Some("governance-transition-or-proof");
        };
        if (!fillable || !old_value.is_null()) && new_value != old_value {
            return Some("governance-transition-or-proof");
        }
    }
    None
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

fn string_array(value: Option<&Value>) -> Option<Vec<&str>> {
    value?
        .as_array()?
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
}

fn proof_valid(value: Option<&Value>, introduced: &[&str]) -> bool {
    let Some(proof) = value.and_then(Value::as_object) else {
        return false;
    };
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
    let actual: Option<Vec<&str>> = records
        .iter()
        .map(|record| record.get("primary_fingerprint").and_then(Value::as_str))
        .collect();
    actual.as_deref() == Some(introduced)
}

fn approvals_valid(value: Option<&Value>) -> bool {
    let Some(approvals) = value.and_then(Value::as_array) else {
        return false;
    };
    if approvals.len() != 2 {
        return false;
    }
    let people: Option<BTreeSet<&str>> = approvals
        .iter()
        .map(|approval| approval.get("person").and_then(Value::as_str))
        .collect();
    let roles: Option<BTreeSet<&str>> = approvals
        .iter()
        .map(|approval| approval.get("role").and_then(Value::as_str))
        .collect();
    people.is_some_and(|items| items.len() == 2)
        && roles
            == Some(BTreeSet::from([
                "architect-security",
                "maintainer-administrator",
            ]))
}

#[cfg(test)]
#[path = "governance/tests.rs"]
mod tests;
