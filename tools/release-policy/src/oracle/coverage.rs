use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde_json::Value;

use super::model::{
    Case, CountEntry, CoverageInventory, Manifest, ReasonCount, SubjectEntry, TransitionEntry,
};
use super::path::repository_file;
use crate::error::{Error, Result};
use crate::json;

// RFC 119 track A: "authority" and "challenge" were dropped (from 11 subjects to 9) when
// `signer-authority`/`signer-authority-live`/`signer-challenge` were parked -- those were the only
// suites that ever populated either subject (confirmed by re-deriving membership against the full
// 154-case set before parking: `authority` had 11 members, `challenge` had 16, both exclusively
// from the parked suites; every other subject kept at least one member). A subject with zero
// members after parking would fail `derive`'s own "manifest-contract:coverage-empty-subject" check
// below, so this list must track exactly what the surviving cases can populate.
//
// RFC 119 track B: "release-state" and "governance"/"hold" dropped in the same round (9 -> 6).
// `release-state`'s whole 23-case suite was removed (NEVER, superseded by the
// proposal-authorize-execute procedure) -- the only populator of the "release-state" subject.
// Separately, 16 of `release-evidence`'s 73 cases were parked: the ones exercising its embedded
// DC-35 signer-governance sub-object. Every case with "governance" in its name, and every case
// with "hold" in its name, was among those 16 (re-derived by simulating `subject_membership`
// against the surviving 57-case set before editing this list) -- so both subjects would otherwise
// be empty.
const SUBJECTS: [&str; 6] = [
    "schema",
    "transition",
    "exact-byte",
    "tag",
    "completion",
    "sequence",
];

// RFC 119 track B: the `schema-evaluator` and `json-parser` tuples were removed outright -- both
// suites were removed (NEVER), so listing their case names here would claim a regression is still
// protected when nothing runs it any more. Eleven `release-evidence` names were removed for the
// same reason: they name cases that were parked (LATER), not deleted, but a parked case does not
// run either, so it cannot honestly appear as a currently-protected regression.
const REPAIR_REGRESSIONS: [(&str, &[&str]); 1] = [(
    "release-evidence",
    &[
        "raw_predecessor_digest_mismatch",
        "full_schema_boolean_version_rejected",
        "tag_verification_signer_primary_fingerprint_immutable",
        "tag_verification_authority_path_immutable",
        "tag_verification_authority_blob_id_immutable",
        "tag_verification_verifier_result_immutable",
        "tag_verification_status_immutable",
        "sequence_zero_attempt_growth",
        "snapshot_object_bytes_mismatch",
        "pending_verified_without_details",
        "partial_verified_without_details",
        "pending_not_observed_with_detail",
        "pending_failed_without_authority",
        "partial_failed_without_authority",
        "pending_failed_with_authority_and_result",
    ],
)];

pub(super) fn verify(
    root: &Path,
    manifest: &Manifest,
    inputs: &BTreeMap<(String, String, String), Vec<u8>>,
) -> Result<()> {
    let bytes = fs::read(repository_file(
        root,
        "release/oracle/coverage-inventory-v1.json",
    )?)?;
    let actual: CoverageInventory = serde_json::from_value(
        json::parse(&bytes)
            .map_err(|error| Error::new(format!("manifest-contract:coverage-json:{error}")))?,
    )?;
    let expected = derive(manifest, inputs)?;
    if actual != expected {
        return Err(Error::new("manifest-contract:coverage-exact"));
    }
    Ok(())
}

fn derive(
    manifest: &Manifest,
    inputs: &BTreeMap<(String, String, String), Vec<u8>>,
) -> Result<CoverageInventory> {
    let mut suite_counts = BTreeMap::<String, u64>::new();
    let mut reason_counts = BTreeMap::<String, u64>::new();
    let mut subjects: BTreeMap<&str, Vec<String>> = SUBJECTS
        .into_iter()
        .map(|name| (name, Vec::new()))
        .collect();
    let mut transitions = Vec::new();
    for case in &manifest.cases {
        *suite_counts.entry(case.suite_id.clone()).or_default() += 1;
        *reason_counts
            .entry(case.expected.primary_reason.clone())
            .or_default() += 1;
        for subject in subject_membership(case) {
            subjects
                .get_mut(subject)
                .ok_or_else(|| Error::new("manifest-contract:coverage-subject"))?
                .push(case_key(case));
        }
        if case.suite_id == "release-evidence" && case.fixture_case_id.starts_with("transition_") {
            transitions.push(transition(case, inputs)?);
        }
    }
    for members in subjects.values_mut() {
        members.sort();
        if members.is_empty() {
            return Err(Error::new("manifest-contract:coverage-empty-subject"));
        }
    }
    transitions.sort_by(|left, right| (&left.from, &left.to).cmp(&(&right.from, &right.to)));
    let pairs: BTreeSet<(&str, &str)> = transitions
        .iter()
        .map(|entry| (entry.from.as_str(), entry.to.as_str()))
        .collect();
    let statuses = ["pending", "partial", "complete", "superseded"];
    let required: BTreeSet<(&str, &str)> = statuses
        .iter()
        .flat_map(|left| statuses.iter().map(move |right| (*left, *right)))
        .collect();
    if pairs != required || transitions.len() != 16 {
        return Err(Error::new("manifest-contract:coverage-transition-pairs"));
    }
    let mut regressions: Vec<String> = REPAIR_REGRESSIONS
        .iter()
        .flat_map(|(suite, cases)| {
            cases
                .iter()
                .map(move |case| format!("{suite}:{}", case.replace('_', "-")))
        })
        .collect();
    regressions.sort();
    Ok(CoverageInventory {
        schema_version: "oracle-coverage-v1".to_owned(),
        total_cases: manifest.cases.len() as u64,
        suites: suite_counts
            .into_iter()
            .map(|(suite_id, case_count)| CountEntry {
                suite_id,
                case_count,
            })
            .collect(),
        reason_counts: reason_counts
            .into_iter()
            .map(|(primary_reason, case_count)| ReasonCount {
                primary_reason,
                case_count,
            })
            .collect(),
        subjects: SUBJECTS
            .into_iter()
            .map(|subject| SubjectEntry {
                subject: subject.to_owned(),
                case_keys: subjects.remove(subject).unwrap_or_default(),
            })
            .collect(),
        transition_pairs: transitions,
        repair_regressions: regressions,
    })
}

fn subject_membership(case: &Case) -> Vec<&'static str> {
    let suite = case.suite_id.as_str();
    let fixture = case.fixture_case_id.as_str();
    let mut subjects = BTreeSet::new();
    if suite.starts_with("signer-authority") {
        subjects.insert("authority");
    }
    if suite == "signer-challenge" {
        subjects.extend(["challenge", "exact-byte"]);
    }
    if case.expected.structural == "invalid" {
        subjects.insert("schema");
    }
    if fixture.starts_with("transition_") {
        subjects.insert("transition");
    }
    if [
        "raw_",
        "digest",
        "object_bytes",
        "golden_bytes",
        "crlf",
        "final_lf",
    ]
    .iter()
    .any(|word| fixture.contains(word))
    {
        subjects.insert("exact-byte");
    }
    if fixture.contains("tag") {
        subjects.insert("tag");
    }
    if [
        "complete",
        "checksum",
        "archive",
        "crate",
        "pages",
        "release_page",
    ]
    .iter()
    .any(|word| fixture.contains(word))
    {
        subjects.insert("completion");
    }
    if case.sequence.is_some() {
        subjects.insert("sequence");
    }
    subjects.into_iter().collect()
}

fn transition(
    case: &Case,
    inputs: &BTreeMap<(String, String, String), Vec<u8>>,
) -> Result<TransitionEntry> {
    let status = |role: &str| -> Result<String> {
        let bytes = inputs
            .get(&(case.suite_id.clone(), case.case_id.clone(), role.to_owned()))
            .ok_or_else(|| Error::new("manifest-contract:coverage-transition-input"))?;
        let value = json::parse(bytes)
            .map_err(|_| Error::new("manifest-contract:coverage-transition-json"))?;
        value
            .get("overall_status")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| Error::new("manifest-contract:coverage-transition-status"))
    };
    let from = status("prior-snapshot")?;
    let to = status("current-snapshot")?;
    if case.fixture_case_id != format!("transition_{from}_to_{to}") {
        return Err(Error::new(
            "manifest-contract:coverage-transition-case-name",
        ));
    }
    Ok(TransitionEntry {
        case_key: case_key(case),
        from,
        to,
        expected_valid: case.expected.final_ == "valid",
    })
}

fn case_key(case: &Case) -> String {
    format!("{}:{}", case.suite_id, case.case_id)
}
