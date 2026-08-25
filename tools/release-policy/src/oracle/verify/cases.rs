use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde_json::Value;

use super::super::model::{Case, Location, Manifest};
use super::super::path::{lexical, repository_file};
use super::identity::verify_bytes;
use crate::error::{Error, Result};
use crate::json;

pub(super) fn load(
    root: &Path,
    manifest: &Manifest,
    payloads: &BTreeMap<(String, String), Vec<u8>>,
) -> Result<BTreeMap<(String, String, String), Vec<u8>>> {
    let keys: Vec<(&str, &str)> = manifest
        .cases
        .iter()
        .map(|case| (case.suite_id.as_str(), case.case_id.as_str()))
        .collect();
    let mut sorted_keys = keys.clone();
    sorted_keys.sort_unstable();
    sorted_keys.dedup();
    if keys != sorted_keys {
        return Err(Error::new("manifest-contract:case-order-or-duplicate"));
    }
    let mut references: BTreeMap<(String, String), u64> = BTreeMap::new();
    let mut loaded = BTreeMap::new();
    for case in &manifest.cases {
        verify_shape(case)?;
        for input in &case.inputs {
            let (label, bytes) = match &input.location {
                Location::Direct { path } => {
                    (path.clone(), fs::read(repository_file(root, path)?)?)
                }
                Location::Packed { pack_id, entry_id } => {
                    if !lexical(entry_id) {
                        return Err(Error::new("manifest-contract:pack-entry-path"));
                    }
                    if pack_id != &case.suite_id || !packed_role_allowed(case, &input.role) {
                        return Err(Error::new("manifest-contract:packed-suite-role"));
                    }
                    let key = (pack_id.clone(), entry_id.clone());
                    *references.entry(key.clone()).or_default() += 1;
                    let bytes = payloads
                        .get(&key)
                        .cloned()
                        .ok_or_else(|| Error::new("manifest-contract:pack-entry-missing"))?;
                    (format!("{pack_id}:{entry_id}"), bytes)
                }
            };
            verify_bytes(&label, &bytes, input.byte_length, &input.sha256)?;
            loaded.insert(
                (
                    case.suite_id.clone(),
                    case.case_id.clone(),
                    input.role.clone(),
                ),
                bytes,
            );
        }
        verify_sequence(case, &loaded)?;
    }
    let expected: BTreeMap<(String, String), u64> =
        payloads.keys().cloned().map(|key| (key, 1)).collect();
    if references != expected {
        return Err(Error::new("manifest-contract:pack-entry-closure"));
    }
    Ok(loaded)
}

fn verify_shape(case: &Case) -> Result<()> {
    if case.case_id != case.fixture_case_id.replace('_', "-") {
        return Err(Error::new("manifest-contract:fixture-case-binding"));
    }
    let roles: Vec<&str> = case
        .inputs
        .iter()
        .map(|input| input.role.as_str())
        .collect();
    let mut expected = match case.suite_id.as_str() {
        "signer-authority-live" => vec!["authority", "expected-output"],
        "signer-challenge" => vec!["fixture-table", "challenge", "expected-output"],
        "release-evidence" => vec![
            "schema",
            "fixture-table",
            "current-snapshot",
            "expected-output",
        ],
        _ => vec!["fixture-table", "expected-output"],
    };
    if case.suite_id == "release-evidence" && case.sequence.is_some() {
        expected.insert(2, "prior-snapshot");
    }
    if roles != expected {
        return Err(Error::new("manifest-contract:input-roles"));
    }
    if !case
        .inputs
        .iter()
        .enumerate()
        .all(|(index, input)| input.ordinal == index as u64)
    {
        return Err(Error::new("manifest-contract:input-order"));
    }
    let locations: BTreeSet<String> = case
        .inputs
        .iter()
        .map(|input| input.location.key())
        .collect();
    if locations.len() != case.inputs.len() {
        return Err(Error::new("manifest-contract:input-location-duplicate"));
    }
    Ok(())
}

fn packed_role_allowed(case: &Case, role: &str) -> bool {
    match case.suite_id.as_str() {
        "signer-challenge" => matches!(role, "fixture-table" | "challenge"),
        "release-evidence" => matches!(
            role,
            "fixture-table" | "prior-snapshot" | "current-snapshot"
        ),
        _ => false,
    }
}

fn verify_sequence(
    case: &Case,
    loaded: &BTreeMap<(String, String, String), Vec<u8>>,
) -> Result<()> {
    let Some(sequence) = &case.sequence else {
        return Ok(());
    };
    if sequence.len() != 2 {
        return Err(Error::new("manifest-contract:sequence-size"));
    }
    let roles = ["prior-snapshot", "current-snapshot"];
    let mut prior_name: Option<String> = None;
    for (index, (member, role)) in sequence.iter().zip(roles).enumerate() {
        let input = case
            .inputs
            .iter()
            .find(|input| input.role == role)
            .ok_or_else(|| Error::new("manifest-contract:sequence-role"))?;
        if member.input_ordinal != input.ordinal
            || member.byte_length != input.byte_length
            || member.sha256 != input.sha256
        {
            return Err(Error::new("manifest-contract:sequence-identity"));
        }
        let bytes = loaded
            .get(&(case.suite_id.clone(), case.case_id.clone(), role.to_owned()))
            .ok_or_else(|| Error::new("manifest-contract:sequence-input"))?;
        let snapshot =
            json::parse(bytes).map_err(|_| Error::new("manifest-contract:sequence-json"))?;
        let version = snapshot
            .get("version")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::new("manifest-contract:sequence-version"))?;
        let number = snapshot
            .get("sequence")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::new("manifest-contract:sequence-number"))?;
        let name = format!("prikk-{version}-release-evidence-{number}.json");
        if member.current_name != name || member.predecessor_name != prior_name {
            return Err(Error::new(format!(
                "manifest-contract:sequence-name:{index}"
            )));
        }
        prior_name = Some(name);
    }
    Ok(())
}
