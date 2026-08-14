use prikk_error::Result;

use super::{PointerIndexEntry, append_ref_pointer_entry, lookup_ref_pointer};
use crate::layout::{RepositoryLayout, ref_name_key_bytes};
use crate::test_support::{sample_object_id, unique_temp_dir};

#[test]
fn write_then_lookup_round_trips() -> Result<()> {
    let root = unique_temp_dir("pointer-index-write-lookup");
    let layout = RepositoryLayout::init(root.clone())?;
    let key = ref_name_key_bytes("heads/main");
    let entry = PointerIndexEntry {
        ref_name_key: key,
        ref_name: "heads/main".to_string(),
        ref_state_id: sample_object_id("state"),
    };
    append_ref_pointer_entry(&layout, &entry)?;

    let found = lookup_ref_pointer(&layout, key)?;
    assert_eq!(found, Some(entry));
    assert_eq!(lookup_ref_pointer(&layout, ref_name_key_bytes("heads/other"))?, None);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// "Last entry wins" (Step 0 §13.4's own words, mirroring `index.rs::lookup_object_location`): a
/// second publish for the same ref supersedes the first at lookup time, without needing the first
/// entry removed or rewritten -- append-only, never overwritten in place.
#[test]
fn a_second_publish_supersedes_the_first_at_lookup() -> Result<()> {
    let root = unique_temp_dir("pointer-index-last-wins");
    let layout = RepositoryLayout::init(root.clone())?;
    let key = ref_name_key_bytes("heads/main");
    let first = PointerIndexEntry {
        ref_name_key: key,
        ref_name: "heads/main".to_string(),
        ref_state_id: sample_object_id("state-1"),
    };
    let second = PointerIndexEntry {
        ref_name_key: key,
        ref_name: "heads/main".to_string(),
        ref_state_id: sample_object_id("state-2"),
    };
    append_ref_pointer_entry(&layout, &first)?;
    append_ref_pointer_entry(&layout, &second)?;

    assert_eq!(lookup_ref_pointer(&layout, key)?, Some(second));

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
