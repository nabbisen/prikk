//! Data-model-lifecycle-`ProjectGenesis`-currency handoff v1 §4 — the object-taxonomy table
//! binding gate.
//!
//! `docs/src/reference/data-model-lifecycle.md`'s object taxonomy table is a hand-maintained
//! transcription of [`ObjectType`]. It went stale for two releases, undetected, after the
//! repository-identity settlement deleted `ProjectGenesis` from the enum but not from this table.
//! [`ObjectType::ALL`] is the single declared source (RFC 118 stage 6); this `#[test]` binds the
//! table to it bidirectionally, the same shape RFC 118 stage 3's trust-gated-operations gate uses:
//!
//! - every [`ObjectType::ALL`] entry's code and name is a row in the table;
//! - every row in the table is a real `ObjectType` at that code.
//!
//! Deliberately binds only the **Code** and **Type** columns, not **Role** or **Stored in** —
//! those are authored prose describing what an object is *for*, not derivable from the enum, and
//! this gate must not pretend otherwise. **It proves the table's code/name inventory agrees with
//! the enum. It does not, and cannot, prove the Role column stays accurate.**

#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::id::ObjectType;

const DATA_MODEL_LIFECYCLE_PATH: &str = "docs/src/reference/data-model-lifecycle.md";
const TABLE_START_MARKER: &str = "<!-- object-taxonomy-table:start -->";
const TABLE_END_MARKER: &str = "<!-- object-taxonomy-table:end -->";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("prikk-object's manifest dir has a workspace root two levels up")
        .to_path_buf()
}

fn read_data_model_lifecycle() -> String {
    let root = repo_root();
    fs::read_to_string(root.join(DATA_MODEL_LIFECYCLE_PATH))
        .unwrap_or_else(|err| panic!("{DATA_MODEL_LIFECYCLE_PATH} must read: {err}"))
}

/// The text strictly between the two HTML comment markers -- scoping every check to the declared
/// table, not the whole page (which discusses many other objects and concepts).
fn bound_table_text(text: &str) -> &str {
    let after_start = text
        .find(TABLE_START_MARKER)
        .map(|start| start + TABLE_START_MARKER.len())
        .unwrap_or_else(|| panic!("{DATA_MODEL_LIFECYCLE_PATH} is missing {TABLE_START_MARKER}"));
    let end = text[after_start..]
        .find(TABLE_END_MARKER)
        .map(|rel| after_start + rel)
        .unwrap_or_else(|| panic!("{DATA_MODEL_LIFECYCLE_PATH} is missing {TABLE_END_MARKER}"));
    &text[after_start..end]
}

/// Every `(code, name)` pair the bound table's data rows declare -- the header and separator
/// rows carry neither a backtick-quoted `0x..` code nor a `**Bold**` name, so they fall out of
/// this parse on their own; nothing needs to skip them explicitly.
fn table_rows(text: &str) -> Vec<(u16, &str)> {
    text.lines().filter_map(row_code_and_name).collect()
}

fn row_code_and_name(line: &str) -> Option<(u16, &str)> {
    let after_tick = line.split('`').nth(1)?;
    let code = u16::from_str_radix(after_tick.strip_prefix("0x")?, 16).ok()?;
    let name_start = line.find("**")? + 2;
    let name_end = name_start + line[name_start..].find("**")?;
    Some((code, &line[name_start..name_end]))
}

#[test]
fn every_object_type_is_named_in_the_data_model_lifecycle_table() {
    let text = read_data_model_lifecycle();
    let rows = table_rows(bound_table_text(&text));
    for object_type in ObjectType::ALL {
        let name = format!("{object_type:?}");
        assert!(
            rows.contains(&(object_type.code(), name.as_str())),
            "ObjectType::{name} (code {:#04x}) is not named in {DATA_MODEL_LIFECYCLE_PATH}'s \
             object taxonomy table",
            object_type.code()
        );
    }
}

#[test]
fn every_data_model_lifecycle_table_row_is_a_real_object_type() {
    let text = read_data_model_lifecycle();
    for (code, name) in table_rows(bound_table_text(&text)) {
        assert!(
            ObjectType::ALL
                .iter()
                .any(|object_type| object_type.code() == code
                    && format!("{object_type:?}") == name),
            "{DATA_MODEL_LIFECYCLE_PATH} names `{name}` at code {code:#04x}, but no ObjectType \
             variant matches both"
        );
    }
}
