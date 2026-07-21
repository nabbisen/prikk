mod alias;

use std::path::Path;

use serde_json::{Value, json};

use super::super::verify;
use crate::error::{Error, Result};

pub(super) fn run(root: &Path, original: &Value, errors: &mut Vec<String>) -> Result<()> {
    for (name, location) in [
        (
            "location-both",
            json!({"kind":"direct","path":"release-signers.toml","pack_id":"x","entry_id":"x"}),
        ),
        ("location-neither", json!({"kind":"direct"})),
        (
            "location-mixed",
            json!({"kind":"packed","path":"x","pack_id":"x","entry_id":"x"}),
        ),
        (
            "location-wrong-kind",
            json!({"kind":"unknown","path":"release-signers.toml"}),
        ),
        (
            "location-traversal",
            json!({"kind":"direct","path":"../outside"}),
        ),
    ] {
        reject(root, original, errors, name, |manifest| {
            *direct_location_mut(manifest)? = location;
            Ok(())
        })?;
    }
    reject(root, original, errors, "direct-dot-alias", |manifest| {
        let location = direct_location_mut(manifest)?;
        let path = string_field(location, "path")?;
        set_field(location, "path", json!(path.replacen('/', "/./", 1)))?;
        Ok(())
    })?;
    reject(root, original, errors, "registry-dot-alias", |manifest| {
        let pack = first_array_item_mut(manifest, "packs")?;
        let path = string_field(pack, "path")?;
        set_field(pack, "path", json!(path.replacen('/', "/./", 1)))?;
        Ok(())
    })?;
    for (name, mutation) in [
        (
            "pack-absent",
            PackMutation::Path("release/oracle/packs/absent.json"),
        ),
        ("pack-traversal", PackMutation::Path("../pack.json")),
        ("pack-length", PackMutation::Length),
        ("pack-hash", PackMutation::Hash),
    ] {
        reject(root, original, errors, name, |manifest| {
            let pack = first_array_item_mut(manifest, "packs")?;
            match mutation {
                PackMutation::Path(path) => set_field(pack, "path", json!(path))?,
                PackMutation::Length => {
                    let length = pack
                        .get("byte_length")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| Error::new("pack length absent"))?;
                    set_field(pack, "byte_length", json!(length + 1))?;
                }
                PackMutation::Hash => set_field(pack, "sha256", json!("0".repeat(64)))?,
            }
            Ok(())
        })?;
    }
    reject(root, original, errors, "pack-omission", |manifest| {
        array_field_mut(manifest, "packs")?.pop();
        Ok(())
    })?;
    reject(root, original, errors, "pack-extra", |manifest| {
        let packs = array_field_mut(manifest, "packs")?;
        let extra = packs
            .first()
            .cloned()
            .ok_or_else(|| Error::new("pack absent"))?;
        packs.push(extra);
        Ok(())
    })?;
    reject(root, original, errors, "entry-absent", |manifest| {
        set_field(
            packed_location_mut(manifest)?,
            "entry_id",
            json!("release/oracle/vectors/release-state/absent/context.json"),
        )
    })?;
    reject(root, original, errors, "entry-length", |manifest| {
        let input = packed_input_mut(manifest)?;
        let length = input
            .get("byte_length")
            .and_then(Value::as_u64)
            .ok_or_else(|| Error::new("entry length absent"))?;
        set_field(input, "byte_length", json!(length + 1))
    })?;
    reject(root, original, errors, "entry-hash", |manifest| {
        set_field(packed_input_mut(manifest)?, "sha256", json!("0".repeat(64)))
    })?;
    reject(
        root,
        original,
        errors,
        "entry-one-to-one-closure",
        duplicate_packed_binding,
    )?;
    reject(
        root,
        original,
        errors,
        "sequence-reversed-members",
        |manifest| {
            sequence_case_mut(manifest)?
                .get_mut("sequence")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| Error::new("sequence not array"))?
                .reverse();
            Ok(())
        },
    )?;
    reject(
        root,
        original,
        errors,
        "sequence-reused-input",
        reuse_sequence_input,
    )?;
    reject(
        root,
        original,
        errors,
        "sequence-reversed-inputs",
        reverse_sequence_inputs,
    )?;
    for segment in ["./", "../"] {
        alias::coordinated_pack_alias(root, original, segment, errors)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum PackMutation {
    Path(&'static str),
    Length,
    Hash,
}

fn reject(
    root: &Path,
    original: &Value,
    errors: &mut Vec<String>,
    name: &str,
    mutation: impl FnOnce(&mut Value) -> Result<()>,
) -> Result<()> {
    let mut manifest = original.clone();
    mutation(&mut manifest)?;
    if verify::load_manifest(root, manifest).is_ok() {
        errors.push(format!("self-test:{name}-not-rejected"));
    }
    Ok(())
}

fn duplicate_packed_binding(manifest: &mut Value) -> Result<()> {
    let cases = array_field_mut(manifest, "cases")?;
    let candidates: Vec<(String, Value)> = cases
        .iter()
        .filter_map(|case| case.get("inputs").and_then(Value::as_array))
        .flatten()
        .filter(|input| {
            nested_string_field(input, "location", "kind") == Some("packed")
                && string_field(input, "role").ok() == Some("fixture-table")
        })
        .filter_map(|input| {
            nested_string_field(input, "location", "entry_id")
                .map(|entry| (entry.to_owned(), input.clone()))
        })
        .collect();
    let (source_entry, source) = candidates
        .first()
        .map(|(entry, input)| (entry.clone(), input.clone()))
        .ok_or_else(|| Error::new("packed closure source absent"))?;
    let target = candidates
        .iter()
        .skip(1)
        .find(|(entry, _)| entry != &source_entry)
        .map(|(entry, _)| entry.clone())
        .ok_or_else(|| Error::new("packed closure target absent"))?;
    let input = cases
        .iter_mut()
        .filter_map(|case| case.get_mut("inputs").and_then(Value::as_array_mut))
        .flatten()
        .find(|input| nested_string_field(input, "location", "entry_id") == Some(target.as_str()))
        .ok_or_else(|| Error::new("packed closure target input absent"))?;
    copy_identity(input, &source)
}

fn reuse_sequence_input(manifest: &mut Value) -> Result<()> {
    let case = sequence_case_mut(manifest)?;
    let inputs = array_field_mut(case, "inputs")?;
    let prior = inputs
        .iter()
        .find(|input| string_field(input, "role").ok() == Some("prior-snapshot"))
        .cloned()
        .ok_or_else(|| Error::new("prior sequence input absent"))?;
    let current = inputs
        .iter_mut()
        .find(|input| string_field(input, "role").ok() == Some("current-snapshot"))
        .ok_or_else(|| Error::new("current sequence input absent"))?;
    copy_identity(current, &prior)
}

fn reverse_sequence_inputs(manifest: &mut Value) -> Result<()> {
    let case = sequence_case_mut(manifest)?;
    let inputs = array_field_mut(case, "inputs")?;
    let prior = inputs
        .iter()
        .find(|input| string_field(input, "role").ok() == Some("prior-snapshot"))
        .cloned()
        .ok_or_else(|| Error::new("prior sequence input absent"))?;
    let current = inputs
        .iter()
        .find(|input| string_field(input, "role").ok() == Some("current-snapshot"))
        .cloned()
        .ok_or_else(|| Error::new("current sequence input absent"))?;
    let prior_target = inputs
        .iter_mut()
        .find(|input| string_field(input, "role").ok() == Some("prior-snapshot"))
        .ok_or_else(|| Error::new("prior sequence target absent"))?;
    copy_identity(prior_target, &current)?;
    let current_target = inputs
        .iter_mut()
        .find(|input| string_field(input, "role").ok() == Some("current-snapshot"))
        .ok_or_else(|| Error::new("current sequence target absent"))?;
    copy_identity(current_target, &prior)
}

fn direct_location_mut(manifest: &mut Value) -> Result<&mut Value> {
    array_field_mut(manifest, "cases")?
        .iter_mut()
        .filter_map(|case| case.get_mut("inputs").and_then(Value::as_array_mut))
        .flatten()
        .find_map(|input| {
            let location = input.get_mut("location")?;
            (string_field(location, "kind").ok() == Some("direct")).then_some(location)
        })
        .ok_or_else(|| Error::new("direct location absent"))
}

fn packed_input_mut(manifest: &mut Value) -> Result<&mut Value> {
    array_field_mut(manifest, "cases")?
        .iter_mut()
        .filter_map(|case| case.get_mut("inputs").and_then(Value::as_array_mut))
        .flatten()
        .find(|input| nested_string_field(input, "location", "kind") == Some("packed"))
        .ok_or_else(|| Error::new("packed input absent"))
}

fn packed_location_mut(manifest: &mut Value) -> Result<&mut Value> {
    packed_input_mut(manifest)?
        .get_mut("location")
        .ok_or_else(|| Error::new("packed location absent"))
}

fn sequence_case_mut(manifest: &mut Value) -> Result<&mut Value> {
    array_field_mut(manifest, "cases")?
        .iter_mut()
        .find(|case| case.get("sequence").is_some())
        .ok_or_else(|| Error::new("sequence case absent"))
}

fn first_array_item_mut<'a>(value: &'a mut Value, field: &str) -> Result<&'a mut Value> {
    array_field_mut(value, field)?
        .first_mut()
        .ok_or_else(|| Error::new(format!("{field} empty")))
}

fn array_field_mut<'a>(value: &'a mut Value, field: &str) -> Result<&'a mut Vec<Value>> {
    value
        .get_mut(field)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| Error::new(format!("{field} is not an array")))
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| Error::new(format!("{field} is not a string")))
}

fn nested_string_field<'a>(value: &'a Value, object: &str, field: &str) -> Option<&'a str> {
    value
        .get(object)
        .and_then(|nested| nested.get(field))
        .and_then(Value::as_str)
}

fn set_field(value: &mut Value, field: &str, replacement: Value) -> Result<()> {
    let target = value
        .get_mut(field)
        .ok_or_else(|| Error::new(format!("{field} absent")))?;
    *target = replacement;
    Ok(())
}

fn copy_identity(target: &mut Value, source: &Value) -> Result<()> {
    for field in ["location", "byte_length", "sha256"] {
        let replacement = source
            .get(field)
            .cloned()
            .ok_or_else(|| Error::new(format!("{field} absent")))?;
        set_field(target, field, replacement)?;
    }
    Ok(())
}
