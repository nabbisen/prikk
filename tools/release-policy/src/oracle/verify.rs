mod cases;
mod identity;
mod packs;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use super::Oracle;
use super::coverage;
use super::model::{Manifest, ReasonMap};
use super::path::repository_file;
use crate::error::{Error, Result};
use crate::json;
use crate::schema::SchemaProfile;
use identity::verify_file;

const MANIFEST_PATH: &str = "release/oracle/oracle-manifest-v1.json";
const MANIFEST_SCHEMA_PATH: &str = "release/oracle/oracle-manifest-v1.schema.json";
const OBSERVATIONS_PATH: &str = "release/oracle/python-observations-v1.json";
pub(super) fn load(root: &Path) -> Result<Oracle> {
    let manifest_value = read_json(root, MANIFEST_PATH)?;
    load_manifest(root, manifest_value)
}

pub(super) fn load_manifest(root: &Path, manifest_value: Value) -> Result<Oracle> {
    let manifest_schema = read_json(root, MANIFEST_SCHEMA_PATH)?;
    let profile = SchemaProfile::compile(&manifest_schema)
        .map_err(|error| error.context("manifest-contract:schema-profile"))?;
    if !profile.is_valid(&manifest_value) {
        return Err(Error::new(format!(
            "manifest-contract:schema:{}",
            profile.errors(&manifest_value).join("|")
        )));
    }
    let manifest: Manifest = serde_json::from_value(manifest_value)?;
    verify_identity_fields(&manifest)?;
    verify_file(root, &manifest.normative_schema)?;
    if manifest.normative_schema.path != "release/schemas/release-evidence-v1.schema.json" {
        return Err(Error::new("manifest-contract:normative-schema-path"));
    }
    let payloads = packs::load(root, &manifest, &manifest_schema)?;
    let inputs = cases::load(root, &manifest, &payloads)?;
    verify_reason_map(root, &manifest)?;
    verify_observations(root, &manifest)?;
    coverage::verify(root, &manifest, &inputs)?;
    Ok(Oracle {
        root: root.to_path_buf(),
        manifest,
        inputs,
    })
}

fn verify_identity_fields(manifest: &Manifest) -> Result<()> {
    if manifest.schema_version != "oracle-manifest-v1"
        || manifest.python_baseline_commit != "12c137d"
        || manifest.profile_contract_commit != "ea427df"
        || manifest.observation_adapter_commit != "6be65af"
        || manifest.reason_taxonomy_version != 1
    {
        return Err(Error::new("manifest-contract:identity"));
    }
    Ok(())
}

pub(super) fn parse_pack(
    pack_id: &str,
    bytes: &[u8],
    profile: &SchemaProfile,
) -> Result<BTreeMap<String, Vec<u8>>> {
    packs::parse(pack_id, bytes, profile)
}

fn verify_reason_map(root: &Path, manifest: &Manifest) -> Result<()> {
    let bytes = verify_file(root, &manifest.reason_map)?;
    if manifest.reason_map.path != "release/oracle/reason-map-v1.json" {
        return Err(Error::new("manifest-contract:reason-map-path"));
    }
    let actual: ReasonMap = serde_json::from_value(
        json::parse(&bytes)
            .map_err(|error| Error::new(format!("manifest-contract:reason-map-json:{error}")))?,
    )?;
    let expected: ReasonMap = manifest
        .cases
        .iter()
        .filter(|case| case.expected.final_ != "valid")
        .map(|case| {
            (
                format!("{}:{}", case.suite_id, case.fixture_case_id),
                case.expected.primary_reason.clone(),
            )
        })
        .collect();
    if actual != expected {
        return Err(Error::new("manifest-contract:reason-map-exact"));
    }
    Ok(())
}

fn verify_observations(root: &Path, manifest: &Manifest) -> Result<()> {
    let value = read_json(root, OBSERVATIONS_PATH)?;
    let observations: FrozenObservationDocument = serde_json::from_value(value)?;
    if observations.schema_version != "python-policy-observations-v1"
        || observations.python_baseline_commit != "12c137d"
        || observations.profile_contract_commit != "ea427df"
    {
        return Err(Error::new("manifest-contract:observation-identity"));
    }
    let actual: BTreeMap<(&str, &str), _> = observations
        .cases
        .iter()
        .map(|case| ((case.suite_id.as_str(), case.case_id.as_str()), case))
        .collect();
    if actual.len() != manifest.cases.len() {
        return Err(Error::new("manifest-contract:observation-case-set"));
    }
    for case in &manifest.cases {
        let observed = actual
            .get(&(case.suite_id.as_str(), case.fixture_case_id.as_str()))
            .ok_or_else(|| Error::new("manifest-contract:observation-case-set"))?;
        if observed.final_ != case.expected.final_
            || observed.case_outcome != case.expected.case_outcome
            || observed
                .structural
                .as_ref()
                .is_some_and(|value| value != &case.expected.structural)
            || observed
                .semantic
                .as_ref()
                .is_some_and(|value| value != &case.expected.semantic)
        {
            return Err(Error::new("manifest-contract:observation-value"));
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenObservationDocument {
    schema_version: String,
    python_baseline_commit: String,
    profile_contract_commit: String,
    cases: Vec<FrozenObservation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenObservation {
    suite_id: String,
    case_id: String,
    #[serde(rename = "final")]
    final_: String,
    case_outcome: String,
    structural: Option<String>,
    semantic: Option<String>,
}

fn read_json(root: &Path, relative: &str) -> Result<Value> {
    let path = repository_file(root, relative)?;
    json::parse(&fs::read(path)?)
        .map_err(|error| Error::new(format!("manifest-contract:json:{relative}:{error}")))
}
