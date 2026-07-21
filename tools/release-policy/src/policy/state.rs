use std::collections::BTreeSet;

use serde_json::Value;

use super::evidence;
use crate::error::{Error, Result};
use crate::schema::SchemaProfile;

pub(super) fn validate(context: &Value, schema: &SchemaProfile) -> Result<(bool, bool)> {
    let object = context
        .as_object()
        .ok_or_else(|| Error::new("release-state context must be an object"))?;
    if object.len() != 2
        || !object.contains_key("case")
        || !object.contains_key("governance_evidence")
    {
        return Ok((false, false));
    }
    let case = object
        .get("case")
        .and_then(Value::as_object)
        .ok_or_else(|| Error::new("release-state case must be an object"))?;
    let known: BTreeSet<&str> = [
        "id",
        "workspace",
        "latest",
        "candidate",
        "changelog",
        "rfc",
        "tag",
        "distribution",
        "internal_requirements",
        "missing_output",
        "authority_change",
        "release_hold",
        "dispute",
        "governance",
        "expected",
    ]
    .into_iter()
    .collect();
    if case.keys().any(|name| !known.contains(name.as_str())) {
        return Ok((false, false));
    }
    let Some(state) = classify(case) else {
        return Ok((false, false));
    };
    if string(case, "internal_requirements").unwrap_or("exact") != "exact"
        || (case
            .get("missing_output")
            .is_some_and(|value| !value.is_null())
            && string(case, "distribution") == Some("complete"))
    {
        return Ok((false, false));
    }
    let authority = string(case, "authority_change").unwrap_or("absent");
    let hold = string(case, "release_hold").unwrap_or("lifted");
    let dispute = string(case, "dispute").unwrap_or("none");
    let governance = governance_valid(case, object.get("governance_evidence"), schema);
    let valid = if state == "development" {
        let transaction = matches!(
            authority,
            "bootstrap" | "addition" | "replacement" | "removal-only" | "classification-only"
        );
        if authority != "absent" {
            transaction && governance == Some(true) && matches!(hold, "active" | "lifted")
        } else {
            governance.is_none() && hold == "lifted" && dispute == "none"
        }
    } else {
        authority == "absent" && hold == "lifted" && dispute == "none"
    };
    if !valid || (state == "development" && !matches!(dispute, "none" | "active")) {
        return Ok((false, false));
    }
    Ok((true, state == "private-finalization"))
}

fn governance_valid(
    case: &serde_json::Map<String, Value>,
    evidence: Option<&Value>,
    schema: &SchemaProfile,
) -> Option<bool> {
    let reference = case.get("governance").filter(|value| !value.is_null());
    let reference = reference?;
    let name = reference.get("hold_evidence").and_then(Value::as_str)?;
    let evidence = evidence?.as_object()?;
    if evidence.get("state").and_then(Value::as_str) != Some("present")
        || evidence.get("reference").and_then(Value::as_str) != Some(name)
    {
        return Some(false);
    }
    let document = evidence.get("document")?;
    if !schema.is_valid(document)
        || evidence::single_reason(document).is_some()
        || document.get("governance").is_none_or(Value::is_null)
    {
        return Some(false);
    }
    let governance = document.get("governance")?;
    if governance.get("transaction_type").and_then(Value::as_str)
        != string(case, "authority_change")
    {
        return Some(false);
    }
    let active = governance.get("hold_ended_at").is_some_and(Value::is_null);
    Some((string(case, "release_hold") == Some("active")) == active)
}

fn classify(case: &serde_json::Map<String, Value>) -> Option<&'static str> {
    let fields = (
        string(case, "workspace"),
        string(case, "latest"),
        nullable_string(case, "candidate"),
        string(case, "changelog"),
        string(case, "rfc"),
        string(case, "tag"),
        string(case, "distribution"),
    );
    match fields {
        (
            Some("last-release"),
            Some("last-release"),
            Some(None),
            Some("no-target-claim"),
            Some("proposed-or-accepted"),
            Some("absent-at-head"),
            Some("pending"),
        ) => Some("development"),
        (
            Some("target"),
            Some("last-release"),
            Some(Some("target")),
            Some("candidate"),
            Some("accepted"),
            Some("absent"),
            Some("pending"),
        ) => Some("release-candidate"),
        (
            Some("target"),
            Some("target"),
            Some(None),
            Some("final"),
            Some("done"),
            Some("local-only-matching"),
            Some("pending"),
        ) => Some("private-finalization"),
        (
            Some("target"),
            Some("target"),
            Some(None),
            Some("final"),
            Some("done"),
            Some("public-matching"),
            Some("pending" | "partial" | "complete"),
        ) => Some("released"),
        _ => None,
    }
}

fn string<'a>(case: &'a serde_json::Map<String, Value>, name: &str) -> Option<&'a str> {
    case.get(name).and_then(Value::as_str)
}

fn nullable_string<'a>(
    case: &'a serde_json::Map<String, Value>,
    name: &str,
) -> Option<Option<&'a str>> {
    let value = case.get(name)?;
    if value.is_null() {
        Some(None)
    } else {
        value.as_str().map(Some)
    }
}

#[cfg(test)]
#[path = "state/tests.rs"]
mod tests;
