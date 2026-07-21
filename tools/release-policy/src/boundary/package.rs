use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use super::{BoundaryError, PRODUCTS, push};
use crate::error::{Error, Result};
use crate::json;

pub(super) fn check(root: &Path, errors: &mut Vec<BoundaryError>) -> Result<()> {
    for (package, _) in PRODUCTS {
        let output = Command::new("cargo")
            .args([
                "package",
                "--locked",
                "--allow-dirty",
                "--list",
                "-p",
                package,
            ])
            .current_dir(root)
            .output()?;
        if !output.status.success() {
            push(
                errors,
                "package-contents",
                format!(
                    "{package}: cargo package failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            );
            continue;
        }
        for path in String::from_utf8_lossy(&output.stdout).lines() {
            if path.starts_with("release/oracle/") || path.starts_with("tools/release-policy/") {
                push(errors, "package-contents", format!("{package}:{path}"));
            }
        }
    }
    check_source_tree(root, errors);
    Ok(())
}

fn check_source_tree(root: &Path, errors: &mut Vec<BoundaryError>) {
    for path in [
        "tools/release-policy/Cargo.toml",
        "tools/release-policy/src/main.rs",
        "tools/release-policy/self-test-responsibility-map-v1.json",
        "release/schemas/release-evidence-v1.schema.json",
        "release/oracle/oracle-manifest-v1.json",
        "release/oracle/oracle-manifest-v1.schema.json",
        "release/oracle/coverage-inventory-v1.json",
        "release/oracle/python-observations-v1.json",
        "release/oracle/reason-map-v1.json",
        "release/release-policy-command-inventory-v1.json",
        "release/publication-command-inventory-v1.json",
        "release/oracle/packs/release-evidence-v1.json",
        "release/oracle/packs/release-state-v1.json",
        "release/oracle/packs/signer-challenge-v1.json",
    ] {
        if !root.join(path).is_file() {
            push(errors, "source-archive-contents", path.to_owned());
        }
    }
    let manifest_path = root.join("release/oracle/oracle-manifest-v1.json");
    let manifest = fs::read(&manifest_path)
        .map_err(Error::from)
        .and_then(|bytes| {
            json::parse(&bytes)
                .map_err(|error| Error::new(format!("source manifest JSON: {error}")))
        });
    match manifest {
        Ok(manifest) => check_direct_inputs(root, &manifest, errors),
        Err(error) => push(
            errors,
            "source-archive-contents",
            format!("oracle manifest: {error}"),
        ),
    }
}

fn check_direct_inputs(root: &Path, manifest: &Value, errors: &mut Vec<BoundaryError>) {
    let Some(cases) = manifest.get("cases").and_then(Value::as_array) else {
        push(
            errors,
            "source-archive-contents",
            "oracle manifest cases".to_owned(),
        );
        return;
    };
    for path in cases
        .iter()
        .filter_map(|case| case.get("inputs").and_then(Value::as_array))
        .flatten()
        .filter_map(|input| input.get("location"))
        .filter(|location| location.get("kind").and_then(Value::as_str) == Some("direct"))
        .filter_map(|location| location.get("path").and_then(Value::as_str))
    {
        if !root.join(path).is_file() {
            push(errors, "source-archive-contents", path.to_owned());
        }
    }
}
