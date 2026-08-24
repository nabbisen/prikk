use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::super::model::{Manifest, Pack};
use super::super::path::lexical;
use super::identity::verify_file;
use crate::error::{Error, Result};
use crate::json;
use crate::schema::SchemaProfile;

// RFC 119 track A: `signer-challenge` was here until parking removed every case that referenced
// it, orphaning all 32 of its pack entries -- `load`'s own closure check below (every packed entry
// referenced by exactly one case) would fail with the pack still registered and no case left to
// reference it. Moved to `release/oracle/parked-packs/signer-challenge-v1.json`, not deleted; see
// `release/oracle/parked-cases-v1.json`'s `revival_condition` for what would bring it back.
const PACKS: [(&str, &str); 2] = [
    (
        "release-evidence",
        "release/oracle/packs/release-evidence-v1.json",
    ),
    (
        "release-state",
        "release/oracle/packs/release-state-v1.json",
    ),
];

pub(super) fn load(
    root: &Path,
    manifest: &Manifest,
    manifest_schema: &Value,
) -> Result<BTreeMap<(String, String), Vec<u8>>> {
    let pack_dir = root.join("release/oracle/packs");
    let physical: BTreeSet<String> = fs::read_dir(&pack_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| {
            format!(
                "release/oracle/packs/{}",
                entry.file_name().to_string_lossy()
            )
        })
        .collect();
    let expected_paths: BTreeSet<String> =
        PACKS.iter().map(|(_, path)| (*path).to_owned()).collect();
    if physical != expected_paths {
        return Err(Error::new("manifest-contract:pack-file-set"));
    }
    let registry: Vec<(&str, &str)> = manifest
        .packs
        .iter()
        .map(|pack| (pack.pack_id.as_str(), pack.path.as_str()))
        .collect();
    if registry != PACKS {
        return Err(Error::new("manifest-contract:pack-registry"));
    }
    let definitions = manifest_schema
        .get("$defs")
        .cloned()
        .ok_or_else(|| Error::new("manifest-contract:missing-definitions"))?;
    let pack_schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": "#/$defs/vectorPack",
        "$defs": definitions,
    });
    let profile = SchemaProfile::compile(&pack_schema)
        .map_err(|error| error.context("manifest-contract:pack-schema-profile"))?;
    let mut payloads = BTreeMap::new();
    for identity in &manifest.packs {
        for (entry_id, content) in
            parse(&identity.pack_id, &verify_file(root, identity)?, &profile)?
        {
            payloads.insert((identity.pack_id.clone(), entry_id), content);
        }
    }
    Ok(payloads)
}

pub(super) fn parse(
    pack_id: &str,
    bytes: &[u8],
    profile: &SchemaProfile,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let value = json::parse(bytes)
        .map_err(|error| Error::new(format!("manifest-contract:pack-json:{error}")))?;
    if !profile.is_valid(&value) {
        return Err(Error::new(format!(
            "manifest-contract:pack-schema:{pack_id}"
        )));
    }
    let pack: Pack = serde_json::from_value(value)?;
    if pack.schema_version != "oracle-vector-pack-v1" {
        return Err(Error::new("manifest-contract:pack-version"));
    }
    let ids: Vec<&str> = pack
        .entries
        .iter()
        .map(|entry| entry.entry_id.as_str())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    if ids != sorted {
        return Err(Error::new("manifest-contract:pack-entry-order"));
    }
    let prefix = format!("release/oracle/vectors/{pack_id}/");
    let mut payloads = BTreeMap::new();
    for entry in pack.entries {
        if !lexical(&entry.entry_id) || !entry.entry_id.starts_with(&prefix) {
            return Err(Error::new("manifest-contract:pack-entry-path"));
        }
        payloads.insert(entry.entry_id, entry.content.into_bytes());
    }
    Ok(payloads)
}
