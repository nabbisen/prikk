use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::json;

const PATH: &str = "tools/release-policy/self-test-responsibility-map-v1.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Mapping {
    schema_version: String,
    responsibilities: Vec<Responsibility>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Responsibility {
    python_check: String,
    rust_check: String,
}

pub(super) fn verify(root: &Path, errors: &mut Vec<String>) -> Result<()> {
    let value = json::parse(&fs::read(root.join(PATH))?)
        .map_err(|error| Error::new(format!("responsibility map JSON: {error}")))?;
    let mapping: Mapping = serde_json::from_value(value)?;
    if mapping.schema_version != "oracle-self-test-responsibility-map-v1"
        || mapping.responsibilities.len() != 50
    {
        errors.push("self-test:responsibility-map-identity".to_owned());
    }
    let python: BTreeSet<&str> = mapping
        .responsibilities
        .iter()
        .map(|item| item.python_check.as_str())
        .collect();
    let rust: BTreeSet<&str> = mapping
        .responsibilities
        .iter()
        .map(|item| item.rust_check.as_str())
        .collect();
    if python.len() != mapping.responsibilities.len()
        || rust.len() != mapping.responsibilities.len()
        || mapping
            .responsibilities
            .iter()
            .any(|item| item.python_check.is_empty() || item.rust_check.is_empty())
    {
        errors.push("self-test:responsibility-map-one-to-one".to_owned());
    }
    Ok(())
}
