use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::super::super::verify;
use super::super::candidate;
use crate::error::{Error, Result};
use crate::json;

const MANIFEST: &str = "release/oracle/oracle-manifest-v1.json";

pub(super) fn coordinated_pack_alias(
    root: &Path,
    original: &Value,
    segment: &str,
    errors: &mut Vec<String>,
) -> Result<()> {
    let candidate = candidate::create(root)?;
    let mut manifest = original.clone();
    let location = manifest
        .get_mut("cases")
        .and_then(Value::as_array_mut)
        .and_then(|cases| {
            cases
                .iter_mut()
                .filter_map(|case| case.get_mut("inputs").and_then(Value::as_array_mut))
                .flatten()
                .find_map(|input| {
                    let location = input.get_mut("location")?;
                    (string_field(location, "kind") == Some("packed")
                        && string_field(location, "pack_id") == Some("signer-challenge"))
                    .then_some(location)
                })
        })
        .ok_or_else(|| Error::new("signer challenge packed location absent"))?;
    let old_id = string_field(location, "entry_id")
        .ok_or_else(|| Error::new("packed entry id absent"))?
        .to_owned();
    let new_id = old_id.replacen(
        "signer-challenge/",
        &format!("signer-challenge/{segment}"),
        1,
    );
    set_field(location, "entry_id", json!(new_id))?;

    let pack_path = "release/oracle/packs/signer-challenge-v1.json";
    let mut pack = parse(&candidate.path().join(pack_path))?;
    let entry = pack
        .get_mut("entries")
        .and_then(Value::as_array_mut)
        .and_then(|entries| {
            entries
                .iter_mut()
                .find(|entry| string_field(entry, "entry_id") == Some(old_id.as_str()))
        })
        .ok_or_else(|| Error::new("packed entry absent"))?;
    set_field(entry, "entry_id", json!(new_id))?;
    let mut bytes = serde_json::to_vec_pretty(&pack)?;
    bytes.push(b'\n');
    fs::write(candidate.path().join(pack_path), &bytes)?;
    let identity = manifest
        .get_mut("packs")
        .and_then(Value::as_array_mut)
        .and_then(|packs| {
            packs
                .iter_mut()
                .find(|pack| string_field(pack, "pack_id") == Some("signer-challenge"))
        })
        .ok_or_else(|| Error::new("pack identity absent"))?;
    set_field(identity, "byte_length", json!(bytes.len()))?;
    set_field(
        identity,
        "sha256",
        json!(format!("{:x}", Sha256::digest(&bytes))),
    )?;
    fs::write(
        candidate.path().join(MANIFEST),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    if verify::load(candidate.path()).is_ok() {
        errors.push(format!(
            "self-test:coordinated-packed-{segment}-not-rejected"
        ));
    }
    Ok(())
}

fn parse(path: &Path) -> Result<Value> {
    json::parse(&fs::read(path)?).map_err(|error| Error::new(error.to_string()))
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn set_field(value: &mut Value, field: &str, replacement: Value) -> Result<()> {
    let target = value
        .get_mut(field)
        .ok_or_else(|| Error::new(format!("{field} absent")))?;
    *target = replacement;
    Ok(())
}
