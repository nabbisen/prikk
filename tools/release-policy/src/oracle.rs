mod coverage;
mod model;
mod path;
mod self_test;
mod verify;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};
pub(crate) use model::{Case, Manifest, Observation, ObservationDocument};

#[derive(Debug)]
pub(crate) struct Oracle {
    root: PathBuf,
    pub(crate) manifest: Manifest,
    inputs: BTreeMap<(String, String, String), Vec<u8>>,
}

impl Oracle {
    pub(crate) fn load(root: &Path) -> Result<Self> {
        verify::load(root)
    }

    pub(crate) fn input(&self, case: &Case, role: &str) -> Result<&[u8]> {
        self.inputs
            .get(&(case.suite_id.clone(), case.case_id.clone(), role.to_owned()))
            .map(Vec::as_slice)
            .ok_or_else(|| {
                Error::new(format!(
                    "manifest-contract:missing-role:{}:{}:{role}",
                    case.suite_id, case.case_id
                ))
            })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct VerificationReport {
    pub(crate) schema_version: &'static str,
    pub(crate) valid: bool,
    pub(crate) case_count: usize,
    pub(crate) errors: Vec<String>,
}

pub(crate) fn verify_repository(root: &Path, self_test: bool) -> Result<VerificationReport> {
    let oracle = Oracle::load(root)?;
    let mut errors = Vec::new();
    if self_test {
        errors.extend(self_test::run(root, &oracle)?);
    }
    Ok(VerificationReport {
        schema_version: "oracle-verification-result-v1",
        valid: errors.is_empty(),
        case_count: oracle.manifest.cases.len(),
        errors,
    })
}

#[cfg(test)]
#[path = "oracle/tests.rs"]
mod tests;
