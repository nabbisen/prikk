//! RFC 120: the open-work index gate. Before this gate existed, `ROADMAP.md` referenced none of
//! the four newest `rfcs/proposed/*.md` RFCs -- measured directly (`grep -c`), not assumed --
//! because the fact "which RFCs are proposed" was **partitioned** across places with nothing
//! asserting the set was complete. Unlike `parent_patch_ids` or the object-taxonomy table (RFC
//! 118's own defect class), nothing here was even transcribed, so nothing was stale to catch --
//! the answer was simply incomplete, and no existing gate could have said so.
//!
//! Binds two directions, the same shape `object_type_table_binding_gate.rs` uses for the
//! object-taxonomy table:
//! - every real file in `rfcs/proposed/` (other than `.gitkeep`) is named in `ROADMAP.md`'s
//!   open-work index section;
//! - every entry the index section names is a real file in `rfcs/proposed/`.
//!
//! **Checks presence only, scoped to the HTML-comment-marked block** (matching
//! `object_type_table_binding_gate.rs`/`trust_gated_operations_binding_gate.rs`) -- RFC 120 §3 is
//! explicit that a gate binds existence, not truth, and this one must not pretend to check an
//! entry's one-line description for accuracy, only that the entry exists at all.
//!
//! **Deliberately excludes `MILESTONES.md` and `rfcs/accepted/`** (RFC 120 §6 Q2/Q3, ruled, not
//! oversight): `MILESTONES.md`'s "State today" column is free prose with no marker this gate could
//! read without interpreting it, and `rfcs/accepted/` holds thirteen files dominated by finished
//! work -- widening the gated set to include it would teach readers to ignore the index.

use std::path::Path;

use super::{BoundaryError, push};

const ROADMAP_PATH: &str = "ROADMAP.md";
const PROPOSED_DIR: &str = "rfcs/proposed";
const INDEX_START_MARKER: &str = "<!-- open-work-index:start -->";
const INDEX_END_MARKER: &str = "<!-- open-work-index:end -->";

pub(super) fn check(root: &Path, errors: &mut Vec<BoundaryError>) {
    let Ok(roadmap) = std::fs::read_to_string(root.join(ROADMAP_PATH)) else {
        push(
            errors,
            "open-work-index",
            format!("{ROADMAP_PATH}: unreadable"),
        );
        return;
    };
    let Some(index_text) = bound_index_text(&roadmap) else {
        push(
            errors,
            "open-work-index",
            format!("{ROADMAP_PATH}: missing {INDEX_START_MARKER} or {INDEX_END_MARKER}"),
        );
        return;
    };
    let indexed = indexed_filenames(index_text);

    let Ok(read_dir) = std::fs::read_dir(root.join(PROPOSED_DIR)) else {
        push(
            errors,
            "open-work-index",
            format!("{PROPOSED_DIR}: directory unreadable"),
        );
        return;
    };
    let mut real_files = Vec::new();
    for entry in read_dir {
        let Ok(entry) = entry else {
            push(
                errors,
                "open-work-index",
                format!("{PROPOSED_DIR}: entry unreadable"),
            );
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            push(
                errors,
                "open-work-index",
                format!("{PROPOSED_DIR}: entry type unreadable"),
            );
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".gitkeep" {
            continue;
        }
        real_files.push(name);
    }

    for file in &real_files {
        if !indexed.iter().any(|name| name == file) {
            push(
                errors,
                "open-work-index",
                format!("{PROPOSED_DIR}/{file} is not named in {ROADMAP_PATH}'s open-work index"),
            );
        }
    }
    for name in &indexed {
        if !real_files.contains(name) {
            push(
                errors,
                "open-work-index",
                format!(
                    "{ROADMAP_PATH}'s open-work index names {PROPOSED_DIR}/{name}, which does not exist"
                ),
            );
        }
    }
}

/// The text strictly between the two HTML comment markers -- scoping every check to the declared
/// index, not the surrounding prose (which discusses the backlog tables, `MILESTONES.md`, and the
/// findings-without-a-file section, none of which this gate reads).
fn bound_index_text(text: &str) -> Option<&str> {
    let after_start = text
        .find(INDEX_START_MARKER)
        .map(|start| start + INDEX_START_MARKER.len())?;
    let end = text[after_start..]
        .find(INDEX_END_MARKER)
        .map(|relative| after_start + relative)?;
    Some(&text[after_start..end])
}

/// Every backtick-quoted `.md` filename the bound index text names, one per list item -- the
/// surrounding link text and RFC-number prose fall out of this parse on their own, the same way
/// `object_type_table_binding_gate.rs`'s row parse ignores everything outside its own two
/// backtick/bold markers.
fn indexed_filenames(text: &str) -> Vec<String> {
    text.lines().filter_map(row_filename).collect()
}

fn row_filename(line: &str) -> Option<String> {
    let after_tick = line.split('`').nth(1)?;
    after_tick.ends_with(".md").then(|| after_tick.to_owned())
}

#[cfg(test)]
#[path = "open_work_index/tests.rs"]
mod tests;
