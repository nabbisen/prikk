//! Ref publication tests.

mod object_mismatch;
mod publication_recovery;

use prikk_object::ObjectType;

use crate::layout::ref_name_key_bytes;
use crate::{
    FileObjectStore, ObjectWriter, RefLock, RefPublication, RefStore, RepositoryLayout,
    verify_repository,
};

use crate::fsutil::{TestFailPoint, fail_after_for_test};
use crate::test_support::{
    sample_object_id, signed_empty_block_envelope, signed_ref_state_envelope,
    signed_ref_update_envelope, unique_temp_dir,
};

#[test]
fn ref_lock_rejects_second_writer() {
    let root = unique_temp_dir("ref-lock");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let first = RefLock::acquire(&layout, "heads/main");
        assert!(first.is_ok());
        let second = RefLock::acquire(&layout, "heads/main");
        assert!(second.is_err());
        drop(first);
        let third = RefLock::acquire(&layout, "heads/main");
        assert!(third.is_ok());
    }
    let _ = std::fs::remove_dir_all(root);
}

/// RFC 102 Stage 4 acceptance criterion 2 (handoff §4): "no durability-bearing write uses
/// `atomic_replace`." Demonstrated rather than merely inspected, the same technique `index/
/// tests.rs::container_and_index_writes_append_rather_than_replace` used for RFC 102 Stage 3's own
/// version of this criterion: `atomic_replace`/`write_file_atomically` would overwrite each file's
/// content outright on every write, so a second publish landing in the pointer index and the log
/// container as exactly the first publish's bytes with the second's appended after is only possible
/// if both files are genuinely appended to -- confirming by observed behavior what `append_ref_
/// pointer_entry`/`append_ref_container_record`'s own imports (`append_file_required`, never
/// `write_file_atomically`) already show by inspection. Covers both containers a publish touches,
/// since Step 0 §13.3's ruling ("an append-only record has no candidate value to stage") applies to
/// both identically -- `refs/tmp/`'s candidate mechanism is gone from this path entirely, not just
/// narrowed.
#[test]
fn publish_writes_append_rather_than_replace() -> prikk_error::Result<()> {
    let root = unique_temp_dir("ref-publish-append-not-replace");
    let layout = RepositoryLayout::init(root.clone())?;
    let pointer_index_path = layout.ref_pointer_index_path();
    let log_path = layout.ref_log_container_slot_path(crate::layout::ContainerSlot::A);
    let mut objects = FileObjectStore::new(layout.clone());
    let store = RefStore::new(layout.clone());

    let first_target = objects.write_object(&signed_empty_block_envelope())?;
    let first_state = signed_ref_state_envelope("heads/main", None, first_target, 1);
    let first_state_id = first_state.object_id();
    store.publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_update: signed_ref_update_envelope("heads/main", None, first_state_id, first_target, 1),
        ref_state: first_state,
    })?;
    let pointer_index_after_first = std::fs::read(&pointer_index_path)?;
    let log_after_first = std::fs::read(&log_path)?;
    assert!(!pointer_index_after_first.is_empty());
    assert!(!log_after_first.is_empty());

    let second_target = objects.write_object(&signed_empty_block_envelope())?;
    let second_state = signed_ref_state_envelope("heads/topic", None, second_target, 1);
    let second_state_id = second_state.object_id();
    store.publish(&RefPublication {
        ref_name: "heads/topic".to_string(),
        expected_previous_ref_state_id: None,
        ref_update: signed_ref_update_envelope(
            "heads/topic",
            None,
            second_state_id,
            second_target,
            1,
        ),
        ref_state: second_state,
    })?;
    let pointer_index_after_second = std::fs::read(&pointer_index_path)?;
    let log_after_second = std::fs::read(&log_path)?;

    assert!(pointer_index_after_second.len() > pointer_index_after_first.len());
    assert!(pointer_index_after_second.starts_with(&pointer_index_after_first));
    assert!(log_after_second.len() > log_after_first.len());
    assert!(log_after_second.starts_with(&log_after_first));

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// RFC 102 Stage 4 acceptance criterion 3 (handoff §4): "per-ref sequence order survives -- a log
/// whose records are correct but reordered must still be rejected." Two individually valid,
/// chain-consistent records (`state2.old_ref_state_id == state1.new_ref_state_id`, exactly what a
/// real `update_seq: 1` then `update_seq: 2` publish would produce) are appended to the shared
/// container in reversed physical order -- `state2`'s record lands first, `state1`'s second. Step 0
/// §13.1's own ruling is what this proves: `expected_seq` is computed from a record's position
/// *within its own ref's filtered subsequence*, not trusted from the record's own claimed `update_
/// seq` field or inferred purely from the chain link, so a chain that is internally consistent but
/// physically out of order is still caught. `RefStore::replay_log` (`container::replay_ref_
/// subsequence`) performs no sequence validation at all -- it is `verify_repository` (`refs/verify/
/// scan.rs::validate_log_replay`) that carries this check, confirmed by testing both and finding
/// only one catches it, not assumed from which module the ruling names.
#[test]
fn reordered_but_individually_valid_log_records_are_rejected() -> prikk_error::Result<()> {
    let root = unique_temp_dir("ref-log-reordered");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let target = objects.write_object(&signed_empty_block_envelope())?;

    let state1 = signed_ref_state_envelope("heads/main", None, target, 1);
    let state1_id = state1.object_id();
    objects.write_object(&state1)?;
    let update1 = signed_ref_update_envelope("heads/main", None, state1_id, target, 1);

    let state2 = signed_ref_state_envelope("heads/main", Some(state1_id), target, 2);
    let state2_id = state2.object_id();
    objects.write_object(&state2)?;
    let update2 = signed_ref_update_envelope("heads/main", Some(state1_id), state2_id, target, 2);

    // Physically reversed: update2 (the second transition) lands first in the container, update1
    // second -- both records are individually well-formed and chain-consistent with each other,
    // only their physical order is wrong.
    super::append_log_record_for_signature_test(&layout, "heads/main", &update2)?;
    super::append_log_record_for_signature_test(&layout, "heads/main", &update1)?;
    crate::refs::write_ref_pointer_candidate(&layout, "heads/main", state2_id)?;

    // `RefStore::replay_log` performs no sequence validation -- confirmed, not assumed: it returns
    // both records with no item failure, in their physical (reversed) order.
    let replay = RefStore::new(layout.clone()).replay_log("heads/main")?;
    assert!(!replay.has_item_failure());
    assert_eq!(replay.records.len(), 2);

    // `verify_repository` is what actually carries the per-ref sequence check.
    let report = verify_repository(&layout)?;
    assert!(report.has_item_failure());
    assert!(report.log_outcomes.iter().any(|outcome| {
        matches!(&outcome.status, crate::refs::RefFileStatus::Failed { message }
            if message.contains("ref-log chain or sequence diverges"))
    }));

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn ref_store_publishes_ref_state_and_log() {
    let root = unique_temp_dir("ref-store");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let block = signed_empty_block_envelope();
        let target = block.object_id();
        assert!(object_store.write_object(&block).is_ok());
        let store = RefStore::new(layout.clone());
        let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        let published = store.publish(&publication);
        assert_eq!(published, Ok(ref_state_id));
        assert_eq!(
            store.read_current_ref_state_id("heads/main"),
            Ok(Some(ref_state_id))
        );
        let log = store.replay_log("heads/main");
        assert!(log.is_ok());
        if let Ok(log) = log {
            assert_eq!(log.records.len(), 1);
            assert_eq!(log.trailing_partial_bytes, 0);
        }
        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.checked_refs, Some(1));
            assert_eq!(report.checked_ref_log_records, Some(1));
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn first_publication_retries_completed_log_sync_without_duplicate() -> prikk_error::Result<()> {
    for point in [
        TestFailPoint::RequiredFileSync,
        TestFailPoint::RequiredDirectorySync,
    ] {
        let root = unique_temp_dir("ref-publication-log-sync-retry");
        let layout = RepositoryLayout::init(root.clone())?;
        let mut object_store = FileObjectStore::new(layout.clone());
        let block = signed_empty_block_envelope();
        let target = object_store.write_object(&block)?;
        let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
        let ref_state_id = ref_state.object_id();
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_update: signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1),
            ref_state,
        };
        let store = RefStore::new(layout);

        // RFC 102 Stage 3: an object write now durably appends to both its container and the
        // index (two matching sync calls, not one), preceded by the ref lock's own creation (a
        // third). RFC 102 Stage 4 adds one more before the log append: the pointer-index append
        // (a fourth). Skip 4, not 3, to land the injected failure on the ref log's own completed
        // append.
        fail_after_for_test(point, 4);
        assert!(store.publish(&publication).is_err());
        assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
        assert_eq!(store.publish(&publication)?, ref_state_id);
        assert_eq!(store.replay_log("heads/main")?.records.len(), 1);

        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn ref_cas_log_and_publication_remain_on_retained_repository_root() -> prikk_error::Result<()> {
    let root = unique_temp_dir("ref-root-replacement");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut object_store = FileObjectStore::new(layout.clone());
    let first_block = signed_empty_block_envelope();
    let first_target = object_store.write_object(&first_block)?;
    let store = RefStore::new(layout.clone());
    let first_state = signed_ref_state_envelope("heads/main", None, first_target, 1);
    let first_state_id = first_state.object_id();
    let first_update =
        signed_ref_update_envelope("heads/main", None, first_state_id, first_target, 1);
    store.publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_state: first_state,
        ref_update: first_update,
    })?;

    let displaced = root.join(".prikk-displaced");
    std::fs::rename(layout.prikk_dir(), &displaced)?;
    std::fs::create_dir(root.join(".prikk"))?;
    let second_block = signed_empty_block_envelope();
    let second_target = object_store.write_object(&second_block)?;
    let second_state =
        signed_ref_state_envelope("heads/main", Some(first_state_id), second_target, 2);
    let second_state_id = second_state.object_id();
    let second_update = signed_ref_update_envelope(
        "heads/main",
        Some(first_state_id),
        second_state_id,
        second_target,
        2,
    );
    assert_eq!(
        store.publish(&RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: Some(first_state_id),
            ref_state: second_state,
            ref_update: second_update,
        }),
        Ok(second_state_id)
    );
    assert_eq!(
        store.read_current_ref_state_id("heads/main"),
        Ok(Some(second_state_id))
    );
    assert_eq!(store.replay_log("heads/main")?.records.len(), 2);
    assert!(std::fs::read_dir(root.join(".prikk"))?.next().is_none());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn ref_verification_returns_envelopes_from_retained_anchored_observation() -> prikk_error::Result<()>
{
    let root = unique_temp_dir("ref-verification-root-replacement");
    let layout = RepositoryLayout::init(root.clone())?;
    let block = signed_empty_block_envelope();
    let target = FileObjectStore::new(layout.clone()).write_object(&block)?;
    let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
    let ref_state_id = ref_state.object_id();
    let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1);
    RefStore::new(layout.clone()).publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_state,
        ref_update: ref_update.clone(),
    })?;

    std::fs::rename(layout.prikk_dir(), root.join(".prikk-displaced"))?;
    std::fs::create_dir_all(root.join(".prikk/refs/logs"))?;
    std::fs::write(root.join(".prikk/refs/logs/replacement.log"), b"malicious")?;

    let verification = super::verify_refs(&layout)?;
    assert_eq!(verification.ref_update_envelopes, vec![ref_update]);
    assert_eq!(verification.log_record_count, 1);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn ref_store_rejects_cas_mismatch() {
    let root = unique_temp_dir("ref-cas");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let store = RefStore::new(layout);
        let target = sample_object_id("target-block");
        let bogus_previous = Some(sample_object_id("bogus-previous"));
        let ref_state = signed_ref_state_envelope("heads/main", bogus_previous, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update =
            signed_ref_update_envelope("heads/main", bogus_previous, ref_state_id, target, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: bogus_previous,
            ref_state,
            ref_update,
        };
        assert!(store.publish(&publication).is_err());
        assert_eq!(store.read_current_ref_state_id("heads/main"), Ok(None));
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ref_store_rejects_unborn_publication_when_log_has_history() {
    let root = unique_temp_dir("ref-unborn-log-history");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let block = signed_empty_block_envelope();
        let target = block.object_id();
        assert!(object_store.write_object(&block).is_ok());
        let store = RefStore::new(layout.clone());
        let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1);
        let first = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        assert!(store.publish(&first).is_ok());
        // Containers are append-only, so there is no direct "delete the pointer file" equivalent
        // to the pre-Stage-4 `std::fs::remove_file` this replaces.
        assert!(
            super::remove_pointer_entries_for_test(&layout, ref_name_key_bytes("heads/main"))
                .is_ok()
        );

        let second_target = sample_object_id("different-target");
        let second_ref_state = signed_ref_state_envelope("heads/main", None, second_target, 1);
        let second_ref_state_id = second_ref_state.object_id();
        let second_ref_update =
            signed_ref_update_envelope("heads/main", None, second_ref_state_id, second_target, 1);
        let second = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state: second_ref_state,
            ref_update: second_ref_update,
        };
        assert!(store.publish(&second).is_err());
        assert_eq!(store.read_current_ref_state_id("heads/main"), Ok(None));
        let log = store.replay_log("heads/main");
        assert!(log.is_ok());
        if let Ok(log) = log {
            assert_eq!(log.records.len(), 1);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ref_store_rejects_unborn_publication_when_log_has_trailing_partial() {
    let root = unique_temp_dir("ref-unborn-log-partial");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let block = signed_empty_block_envelope();
        let target = block.object_id();
        assert!(object_store.write_object(&block).is_ok());
        // Containers are append-only, so a raw torn write now targets the shared container
        // directly, attributed to "heads/main" via the frame's own header-carried `ref_name_key`
        // (design-v1.md §13.6) rather than a per-ref file.
        let torn_source = signed_ref_update_envelope(
            "heads/main",
            None,
            sample_object_id("torn-state"),
            sample_object_id("torn-target"),
            1,
        );
        let framed = super::encode_ref_container_record_for_test(
            ref_name_key_bytes("heads/main"),
            &torn_source,
        );
        assert!(framed.is_ok());
        if let Ok(framed) = framed {
            if let Some(torn) = framed.get(..framed.len().saturating_sub(3)) {
                assert!(
                    std::fs::write(
                        layout.ref_log_container_slot_path(crate::layout::ContainerSlot::A),
                        torn
                    )
                    .is_ok()
                );
            }
        }

        let store = RefStore::new(layout.clone());
        let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        assert!(store.publish(&publication).is_err());
        assert_eq!(store.read_current_ref_state_id("heads/main"), Ok(None));
        let log = store.replay_log("heads/main");
        assert!(log.is_ok());
        if let Ok(log) = log {
            assert_eq!(log.records.len(), 0);
            assert_ne!(log.trailing_partial_bytes, 0);
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ref_store_rejects_non_local_branch_publication() {
    let root = unique_temp_dir("ref-non-local-branch");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let block = signed_empty_block_envelope();
        let target = block.object_id();
        assert!(object_store.write_object(&block).is_ok());
        let store = RefStore::new(layout.clone());
        let ref_state = signed_ref_state_envelope("tags/v1", None, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("tags/v1", None, ref_state_id, target, 1);
        let publication = RefPublication {
            ref_name: "tags/v1".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };

        assert!(store.publish(&publication).is_err());
        assert!(!object_store.contains_object(ObjectType::RefState, ref_state_id));
        assert_eq!(store.read_current_ref_state_id("tags/v1"), Ok(None));
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn verify_repository_detects_missing_ref_state_object() {
    let root = unique_temp_dir("ref-missing-state");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let block = signed_empty_block_envelope();
        let target = block.object_id();
        assert!(object_store.write_object(&block).is_ok());
        let store = RefStore::new(layout.clone());
        let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        assert!(store.publish(&publication).is_ok());
        // Containers are append-only, so there is no direct "delete one object" equivalent to the
        // pre-Stage-3 `std::fs::remove_file` this replaces.
        assert!(crate::index::remove_index_entry_for_test(&layout, ref_state_id).is_ok());
        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            // DC-95 Stage 2 Level 2: this ref's own missing target object is now an item-level
            // failure (its pointer/log read), not a whole-`Refs`-stage failure.
            assert!(report.has_item_failure());
        }
    }
    let _ = std::fs::remove_dir_all(root);
}

/// DC-95 Stage 2 Level 2 (refs half) implementation review v1 §2, required: `ensure_no_incomplete_
/// publication` -- the gate every mutation entry point (`ActiveSession`, `append_rollback_draft`,
/// `doctor_repository`'s repair path, `add_trusted_maintainer`, `commit_worktree_changes_signed` via
/// `node_authoring.rs`) calls before proceeding -- previously relied on `verify_refs` returning `Err`
/// for effectively any ref defect. Item containment means `verify_refs` now returns `Ok` even when a
/// specific ref's own item failed; `ensure_no_incomplete_publication`'s own `has_item_failure()`
/// check (`refs.rs`) is what still makes it refuse. **Removing that check from `refs.rs` and running
/// the whole workspace suite found zero failures** -- proving the check was previously wired but
/// entirely untested; this closes that gap. The assertion is the refusal itself, matching the
/// standard `repair_repository_still_refuses_when_the_refs_stage_fails` (`doctor/tests.rs`) already
/// set for the repair path -- the mutation path needed the same proof and did not have it.
///
/// Same fixture as `verify_repository_detects_missing_ref_state_object` above: a published ref
/// whose RefState object is then deleted, a genuine item-level defect (this ref's own pointer read
/// fails), not a structural one.
#[test]
fn ensure_no_incomplete_publication_refuses_when_a_ref_item_fails() {
    let root = unique_temp_dir("ref-mutation-gate-item-failure");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let block = signed_empty_block_envelope();
        let target = block.object_id();
        assert!(object_store.write_object(&block).is_ok());
        let store = RefStore::new(layout.clone());
        let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        assert!(store.publish(&publication).is_ok());
        // Containers are append-only, so there is no direct "delete one object" equivalent to the
        // pre-Stage-3 `std::fs::remove_file` this replaces.
        assert!(crate::index::remove_index_entry_for_test(&layout, ref_state_id).is_ok());

        // Confirm the premise first: `verify_refs` itself no longer returns `Err` for this fixture
        // (item containment), so `ensure_no_incomplete_publication`'s own refusal cannot be coming
        // from that path -- it must be the `has_item_failure()` check.
        assert!(super::verify_refs(&layout).is_ok());
        assert!(super::ensure_no_incomplete_publication(&layout).is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

/// RFC 102 Stage 4 acceptance criterion 5 (handoff §4): "`ensure_no_incomplete_publication` still
/// refuses on a damaged ref container." `ensure_no_incomplete_publication_refuses_when_a_ref_item_
/// fails` above already proves the gate's own `has_item_failure()` wiring using a missing-object
/// defect; this proves the same gate refuses when the *container itself* is damaged (a checksum-
/// corrupted log-container record), not just when it is coherent but references something missing --
/// the container-corruption half of "damaged ref container," not merely a name for the same fixture.
#[test]
fn ensure_no_incomplete_publication_refuses_on_a_corrupted_log_container_record()
-> prikk_error::Result<()> {
    let root = unique_temp_dir("ref-mutation-gate-container-corruption");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let target = objects.write_object(&signed_empty_block_envelope())?;
    let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
    let ref_state_id = ref_state.object_id();
    let publication = RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_update: signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1),
        ref_state,
    };
    RefStore::new(layout.clone()).publish(&publication)?;

    // `heads/main` is the only ref in this fixture, so the whole container is exactly its own
    // subsequence; flip the last byte to corrupt the checksum of its one record.
    let path = layout.ref_log_container_slot_path(crate::layout::ContainerSlot::A);
    let mut bytes = std::fs::read(&path)?;
    let last = bytes
        .last_mut()
        .ok_or_else(|| prikk_error::PrikkError::Integrity("expected a record".to_string()))?;
    *last ^= 0xff;
    std::fs::write(path, bytes)?;

    assert!(super::ensure_no_incomplete_publication(&layout).is_err());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn ref_store_refuses_unsigned_missing_pointer_reconstruction() {
    let root = unique_temp_dir("ref-reconstruct");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let block = signed_empty_block_envelope();
        let target = block.object_id();
        assert!(object_store.write_object(&block).is_ok());
        let store = RefStore::new(layout.clone());
        let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
        let ref_state_id = ref_state.object_id();
        let ref_update = signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_state,
            ref_update,
        };
        assert!(store.publish(&publication).is_ok());
        assert!(
            super::remove_pointer_entries_for_test(&layout, ref_name_key_bytes("heads/main"))
                .is_ok()
        );
        assert_eq!(store.read_current_ref_state_id("heads/main"), Ok(None));
        let candidate = store.recoverable_missing_ref("heads/main");
        assert!(candidate.is_ok());
        assert_eq!(
            candidate.ok().flatten().map(|value| value.ref_state_id),
            Some(ref_state_id)
        );
        let repair = store.reconstruct_missing_ref_from_log("heads/main");
        assert!(repair.is_err());
        assert_eq!(store.read_current_ref_state_id("heads/main"), Ok(None));
        let report = verify_repository(&layout);
        assert!(report.is_ok());
        assert!(report.is_ok_and(|report| report.has_blocking_ref_publication_issues()));
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ref_log_ahead_of_pointer_is_recoverable() {
    let root = unique_temp_dir("ref-log-ahead");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let mut object_store = FileObjectStore::new(layout.clone());
        let block = signed_empty_block_envelope();
        let target = block.object_id();
        assert!(object_store.write_object(&block).is_ok());
        let ref_state = signed_ref_state_envelope("heads/topic", None, target, 1);
        let ref_state_id = ref_state.object_id();
        assert!(object_store.write_object(&ref_state).is_ok());
        let ref_update = signed_ref_update_envelope("heads/topic", None, ref_state_id, target, 1);
        assert!(
            super::append_ref_container_record(
                &layout,
                ref_name_key_bytes("heads/topic"),
                &ref_update
            )
            .is_ok()
        );

        let store = RefStore::new(layout.clone());
        assert_eq!(store.read_current_ref_state_id("heads/topic"), Ok(None));
        let candidate = store.recoverable_missing_ref("heads/topic");
        assert!(candidate.is_ok());
        if let Ok(Some(candidate)) = candidate {
            assert_eq!(candidate.ref_state_id, ref_state_id);
            assert_eq!(candidate.target_object_id, target);
            assert_eq!(candidate.update_seq, 1);
        } else {
            panic!("expected heads/topic to be recoverable from log");
        }

        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert_eq!(report.checked_refs, Some(0));
            assert_eq!(report.checked_ref_log_records, Some(1));
        }

        let repair = store.reconstruct_missing_ref_from_log("heads/topic");
        assert!(repair.is_err());
        assert_eq!(store.read_current_ref_state_id("heads/topic"), Ok(None));
    }
    let _ = std::fs::remove_dir_all(root);
}
