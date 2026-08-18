#![allow(clippy::indexing_slicing)]

use prikk_error::Result;

use super::{
    AuthorKeyEntry, AuthorKeyRecordStatus, decode_author_key_records, encode_author_key_record,
    lookup_author_key_entries, record_author_key_material,
};
use crate::layout::RepositoryLayout;
use crate::test_support::unique_temp_dir;

fn sample_public_key(seed: u8) -> [u8; 32] {
    [seed; 32]
}

#[test]
fn a_single_key_entry_round_trips_through_decode() -> Result<()> {
    let entry = AuthorKeyEntry {
        key_id: "alice".to_string(),
        public_key: sample_public_key(1),
    };
    let bytes = encode_author_key_record(&entry)?;
    let replay = decode_author_key_records(&bytes)?;
    assert_eq!(replay.entries, vec![entry]);
    assert_eq!(replay.trailing_partial_bytes, 0);
    assert!(
        replay
            .record_outcomes
            .iter()
            .all(|outcome| matches!(outcome.status, AuthorKeyRecordStatus::Evaluated))
    );
    Ok(())
}

#[test]
fn trailing_partial_key_bytes_are_tolerated_not_treated_as_corruption() -> Result<()> {
    let entry = AuthorKeyEntry {
        key_id: "alice".to_string(),
        public_key: sample_public_key(1),
    };
    let mut bytes = encode_author_key_record(&entry)?;
    bytes.extend_from_slice(&[0_u8; 10]);
    let replay = decode_author_key_records(&bytes)?;
    assert_eq!(replay.entries, vec![entry]);
    assert_eq!(replay.trailing_partial_bytes, 10);
    Ok(())
}

/// Decode-level isolation, matching `trust_index.rs`'s own key-material precedent: a damaged entry
/// is named at its own offset and the scan resyncs past it.
#[test]
fn isolates_a_damaged_key_entry_and_reads_sound_entries_around_it() -> Result<()> {
    let first = AuthorKeyEntry {
        key_id: "alice".to_string(),
        public_key: sample_public_key(1),
    };
    let damaged_source = AuthorKeyEntry {
        key_id: "bob".to_string(),
        public_key: sample_public_key(2),
    };
    let third = AuthorKeyEntry {
        key_id: "carol".to_string(),
        public_key: sample_public_key(3),
    };
    let mut bytes = encode_author_key_record(&first)?;
    let damaged_start = bytes.len();
    let damaged_record = encode_author_key_record(&damaged_source)?;
    let damaged_end = damaged_start + damaged_record.len();
    bytes.extend_from_slice(&damaged_record);
    bytes.extend_from_slice(&encode_author_key_record(&third)?);
    bytes[damaged_end - 1] ^= 0x01;

    let replay = decode_author_key_records(&bytes)?;
    let failed_offsets: Vec<_> = replay
        .record_outcomes
        .iter()
        .filter_map(|outcome| match &outcome.status {
            AuthorKeyRecordStatus::Failed { .. } => Some(outcome.offset),
            AuthorKeyRecordStatus::Evaluated => None,
        })
        .collect();
    assert_eq!(failed_offsets, vec![damaged_start]);
    assert_eq!(replay.entries, vec![first, third]);
    Ok(())
}

#[test]
fn a_repository_with_no_container_file_reads_as_empty_not_an_error() -> Result<()> {
    // A repository initialized before this container existed -- the module doc's own migration
    // note. `RepositoryLayout::init` now allocates the file, so simulate the pre-increment case by
    // removing it after init.
    let root = unique_temp_dir("author-key-index-missing-container");
    let layout = RepositoryLayout::init(root.clone())?;
    std::fs::remove_file(layout.author_key_container_path())?;
    assert_eq!(lookup_author_key_entries(&layout, "alice")?, Vec::new());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn record_author_key_material_is_idempotent_for_the_same_pair() -> Result<()> {
    let root = unique_temp_dir("author-key-index-idempotent");
    let layout = RepositoryLayout::init(root.clone())?;
    record_author_key_material(&layout, "alice", sample_public_key(1))?;
    record_author_key_material(&layout, "alice", sample_public_key(1))?;
    let entries = lookup_author_key_entries(&layout, "alice")?;
    assert_eq!(entries.len(), 1, "{entries:?}");
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// The module doc's own decision: a second, different key for an existing `key_id` is appended,
/// not refused -- Stage 1 makes no admission judgement about it.
#[test]
fn record_author_key_material_appends_rather_than_rejects_a_conflicting_key() -> Result<()> {
    let root = unique_temp_dir("author-key-index-conflict-appends");
    let layout = RepositoryLayout::init(root.clone())?;
    record_author_key_material(&layout, "alice", sample_public_key(1))?;
    record_author_key_material(&layout, "alice", sample_public_key(2))?;
    let entries = lookup_author_key_entries(&layout, "alice")?;
    assert_eq!(
        entries,
        vec![
            AuthorKeyEntry {
                key_id: "alice".to_string(),
                public_key: sample_public_key(1),
            },
            AuthorKeyEntry {
                key_id: "alice".to_string(),
                public_key: sample_public_key(2),
            },
        ]
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn lookup_only_returns_entries_for_the_requested_key_id() -> Result<()> {
    let root = unique_temp_dir("author-key-index-scoped-lookup");
    let layout = RepositoryLayout::init(root.clone())?;
    record_author_key_material(&layout, "alice", sample_public_key(1))?;
    record_author_key_material(&layout, "bob", sample_public_key(2))?;
    assert_eq!(
        lookup_author_key_entries(&layout, "alice")?,
        vec![AuthorKeyEntry {
            key_id: "alice".to_string(),
            public_key: sample_public_key(1),
        }]
    );
    assert_eq!(lookup_author_key_entries(&layout, "carol")?, Vec::new());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
