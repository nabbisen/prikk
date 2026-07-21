use std::fs;
use std::path::Path;

use serde_json::json;

use super::super::verify;
use crate::error::{Error, Result};
use crate::json;
use crate::schema::SchemaProfile;

pub(super) fn run(root: &Path, errors: &mut Vec<String>) -> Result<()> {
    let schema_bytes = fs::read(root.join("release/oracle/oracle-manifest-v1.schema.json"))?;
    let schema = json::parse(&schema_bytes)
        .map_err(|error| Error::new(format!("self-test pack schema: {error}")))?;
    let pack_schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": "#/$defs/vectorPack",
        "$defs": schema.get("$defs").cloned().ok_or_else(|| Error::new("missing $defs"))?,
    });
    let profile = SchemaProfile::compile(&pack_schema)?;
    let entry = |name: &str, content: &str| {
        json!({
            "entry_id": format!(
                "release/oracle/vectors/signer-challenge/{name}/challenge.txt"
            ),
            "content": content,
        })
    };
    let invalid = [
        ("malformed", b"{".to_vec()),
        (
            "duplicate-name",
            br#"{"schema_version":"oracle-vector-pack-v1","entries":[],"entries":[]}"#.to_vec(),
        ),
        (
            "nested-duplicate-name",
            br#"{"schema_version":"oracle-vector-pack-v1","entries":[{"entry_id":"release/oracle/vectors/signer-challenge/a/challenge.txt","content":"a","content":"b"}]}"#.to_vec(),
        ),
        (
            "bom",
            [
                b"\xef\xbb\xbf".as_slice(),
                serde_json::to_vec(&json!({
                    "schema_version": "oracle-vector-pack-v1",
                    "entries": [entry("a", "value")],
                }))?
                .as_slice(),
            ]
            .concat(),
        ),
        (
            "unknown-field",
            serde_json::to_vec(&json!({
                "schema_version": "oracle-vector-pack-v1",
                "entries": [entry("a", "value")],
                "unexpected": true,
            }))?,
        ),
        (
            "missing-field",
            br#"{"schema_version":"oracle-vector-pack-v1"}"#.to_vec(),
        ),
        (
            "unsupported-version",
            serde_json::to_vec(&json!({"schema_version": "wrong", "entries": []}))?,
        ),
        (
            "duplicate-entry",
            serde_json::to_vec(&json!({
                "schema_version": "oracle-vector-pack-v1",
                "entries": [entry("a", "one"), entry("a", "two")],
            }))?,
        ),
        (
            "unsorted-entry",
            serde_json::to_vec(&json!({
                "schema_version": "oracle-vector-pack-v1",
                "entries": [entry("b", "one"), entry("a", "two")],
            }))?,
        ),
        (
            "lone-high-surrogate",
            br#"{"schema_version":"oracle-vector-pack-v1","entries":[{"entry_id":"release/oracle/vectors/signer-challenge/a/challenge.txt","content":"\ud800"}]}"#.to_vec(),
        ),
        (
            "lone-low-surrogate",
            br#"{"schema_version":"oracle-vector-pack-v1","entries":[{"entry_id":"release/oracle/vectors/signer-challenge/a/challenge.txt","content":"\udc00"}]}"#.to_vec(),
        ),
        (
            "unknown-entry-field",
            serde_json::to_vec(&json!({
                "schema_version": "oracle-vector-pack-v1",
                "entries": [{
                    "entry_id": "release/oracle/vectors/signer-challenge/a/challenge.txt",
                    "content": "x",
                    "unexpected": true
                }]
            }))?,
        ),
        (
            "missing-entry-field",
            serde_json::to_vec(&json!({
                "schema_version": "oracle-vector-pack-v1",
                "entries": [{
                    "entry_id": "release/oracle/vectors/signer-challenge/a/challenge.txt"
                }]
            }))?,
        ),
        (
            "wrong-suite",
            serde_json::to_vec(&json!({
                "schema_version": "oracle-vector-pack-v1",
                "entries": [{
                    "entry_id": "release/oracle/vectors/release-state/a/context.json",
                    "content": "x",
                }],
            }))?,
        ),
        (
            "dot-entry",
            serde_json::to_vec(&json!({
                "schema_version": "oracle-vector-pack-v1",
                "entries": [{
                    "entry_id": "release/oracle/vectors/signer-challenge/./a/challenge.txt",
                    "content": "x",
                }],
            }))?,
        ),
        (
            "dot-dot-entry",
            serde_json::to_vec(&json!({
                "schema_version": "oracle-vector-pack-v1",
                "entries": [{
                    "entry_id": "release/oracle/vectors/signer-challenge/../a/challenge.txt",
                    "content": "x",
                }],
            }))?,
        ),
    ];
    for (name, bytes) in invalid {
        if verify::parse_pack("signer-challenge", &bytes, &profile).is_ok() {
            errors.push(format!("self-test:pack-{name}-not-rejected"));
        }
    }
    let pair = br#"{"schema_version":"oracle-vector-pack-v1","entries":[{"entry_id":"release/oracle/vectors/signer-challenge/a/challenge.txt","content":"\ud83d\ude00"}]}"#;
    let parsed = verify::parse_pack("signer-challenge", pair, &profile)?;
    if parsed.values().next().map(Vec::as_slice) != Some("😀".as_bytes()) {
        errors.push("self-test:pack-surrogate-pair-not-preserved".to_owned());
    }
    for (index, scalar) in ["é", "a\r\nb", "\r", "\n", "no-final-lf", "e\u{301}", "\0\t"]
        .into_iter()
        .enumerate()
    {
        let bytes = serde_json::to_vec(&json!({
            "schema_version": "oracle-vector-pack-v1",
            "entries": [entry("a", scalar)],
        }))?;
        let parsed = verify::parse_pack("signer-challenge", &bytes, &profile)?;
        let escaped = format!(
            "{{\"schema_version\":\"oracle-vector-pack-v1\",\"entries\":[{{\"entry_id\":\"release/oracle/vectors/signer-challenge/a/challenge.txt\",\"content\":\"{}\"}}]}}",
            escaped_json_content(scalar)
        );
        let escaped_parsed = verify::parse_pack("signer-challenge", escaped.as_bytes(), &profile)?;
        if parsed.values().next().map(Vec::as_slice) != Some(scalar.as_bytes())
            || escaped_parsed.values().next().map(Vec::as_slice) != Some(scalar.as_bytes())
        {
            errors.push(format!("self-test:pack-scalar-preservation:{index}"));
        }
    }
    if "é".as_bytes() == "e\u{301}".as_bytes() {
        errors.push("self-test:pack-normalization-distinction-lost".to_owned());
    }
    Ok(())
}

fn escaped_json_content(value: &str) -> String {
    value
        .encode_utf16()
        .map(|unit| format!("\\u{unit:04x}"))
        .collect()
}
