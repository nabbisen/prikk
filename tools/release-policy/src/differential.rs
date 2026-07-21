use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};
use crate::json;
use crate::oracle::{Observation, ObservationDocument, Oracle};
use crate::policy;

#[derive(Debug, Serialize)]
pub(crate) struct DifferentialReport {
    schema_version: &'static str,
    pub(crate) valid: bool,
    case_count: usize,
    oracle_manifest_sha256: String,
    deliberate_disagreement_detected: bool,
    input_disagreement_detected: bool,
    errors: Vec<String>,
}

pub(crate) fn run(root: &Path, self_test: bool) -> Result<DifferentialReport> {
    let oracle = Oracle::load(root)?;
    let rust = policy::evaluate(&oracle)?;
    let python = python_observations(root)?;
    let mut errors = compare(&oracle, &python, &rust.observations, &rust.reasons);
    let deliberate_disagreement_detected = if self_test {
        let mut changed = rust.observations.clone();
        let target = changed
            .cases
            .iter_mut()
            .find(|case| case.final_ == "invalid")
            .ok_or_else(|| Error::new("no invalid case available for differential self-test"))?;
        target.final_ = "valid".to_owned();
        !compare(&oracle, &python, &changed, &rust.reasons).is_empty()
    } else {
        false
    };
    let input_disagreement_detected = if self_test {
        let mut changed = rust.observations.clone();
        let target = changed
            .cases
            .first_mut()
            .ok_or_else(|| Error::new("no case available for input differential self-test"))?;
        target.input_digest = "0".repeat(64);
        !compare(&oracle, &python, &changed, &rust.reasons).is_empty()
    } else {
        false
    };
    if self_test && !deliberate_disagreement_detected {
        errors.push("differential-self-test: disagreement was not detected".to_owned());
    }
    if self_test && !input_disagreement_detected {
        errors.push("differential-self-test: input disagreement was not detected".to_owned());
    }
    let manifest_bytes = std::fs::read(root.join("release/oracle/oracle-manifest-v1.json"))?;
    Ok(DifferentialReport {
        schema_version: "release-policy-differential-v1",
        valid: errors.is_empty(),
        case_count: rust.observations.cases.len(),
        oracle_manifest_sha256: format!("{:x}", Sha256::digest(manifest_bytes)),
        deliberate_disagreement_detected,
        input_disagreement_detected,
        errors,
    })
}

fn python_observations(root: &Path) -> Result<ObservationDocument> {
    let output = Command::new("python3")
        .args(["-B", "release/observe-policy.py"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(Error::new(format!(
            "Python observation command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let value = json::parse(&output.stdout)
        .map_err(|error| Error::new(format!("Python observation JSON: {error}")))?;
    Ok(serde_json::from_value(value)?)
}

fn compare(
    oracle: &Oracle,
    python: &ObservationDocument,
    rust: &ObservationDocument,
    reasons: &BTreeMap<(String, String), String>,
) -> Vec<String> {
    let mut errors = Vec::new();
    if python.schema_version != "python-policy-observations-v1"
        || python.python_baseline_commit != oracle.manifest.python_baseline_commit
        || python.profile_contract_commit != oracle.manifest.profile_contract_commit
        || rust.python_baseline_commit != oracle.manifest.python_baseline_commit
        || rust.profile_contract_commit != oracle.manifest.profile_contract_commit
    {
        errors.push("identity: observation identity mismatch".to_owned());
    }
    let python_cases = by_key(&python.cases);
    let rust_cases = by_key(&rust.cases);
    if python_cases.keys().ne(rust_cases.keys()) {
        errors.push("case-set: Python and Rust case sets differ".to_owned());
    }
    for case in &oracle.manifest.cases {
        let key = (case.suite_id.as_str(), case.fixture_case_id.as_str());
        let Some(left) = python_cases.get(&key) else {
            continue;
        };
        let Some(right) = rust_cases.get(&key) else {
            continue;
        };
        let expected_input_digest = case.input_digest();
        if left.input_digest != expected_input_digest {
            errors.push(format!("{}:{}:python-input-oracle: mismatch", key.0, key.1));
        }
        if right.input_digest != expected_input_digest {
            errors.push(format!("{}:{}:rust-input-oracle: mismatch", key.0, key.1));
        }
        for (name, left_value, right_value) in [
            (
                "input-digest",
                Some(left.input_digest.as_str()),
                Some(right.input_digest.as_str()),
            ),
            (
                "final",
                Some(left.final_.as_str()),
                Some(right.final_.as_str()),
            ),
            (
                "case-outcome",
                Some(left.case_outcome.as_str()),
                Some(right.case_outcome.as_str()),
            ),
            (
                "structural",
                left.structural.as_deref(),
                right.structural.as_deref(),
            ),
            (
                "semantic",
                left.semantic.as_deref(),
                right.semantic.as_deref(),
            ),
        ] {
            if left_value != right_value {
                errors.push(format!("{}:{}:{name}: mismatch", key.0, key.1));
            }
        }
        if reasons.get(&(key.0.to_owned(), key.1.to_owned())) != Some(&case.expected.primary_reason)
        {
            errors.push(format!("{}:{}:primary-reason: mismatch", key.0, key.1));
        }
    }
    errors
}

fn by_key(cases: &[Observation]) -> BTreeMap<(&str, &str), &Observation> {
    cases
        .iter()
        .map(|case| ((case.suite_id.as_str(), case.case_id.as_str()), case))
        .collect()
}

#[cfg(test)]
#[path = "differential/tests.rs"]
mod tests;
