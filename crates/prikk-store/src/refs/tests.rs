//! Ref publication tests.

mod object_mismatch;
mod publication_recovery;

use prikk_object::ObjectType;

use super::log;
use crate::{
    FileObjectStore, ObjectWriter, RefLock, RefPublication, RefStore, RepositoryLayout,
    verify_repository,
};

use crate::fsutil::{TestFailPoint, fail_after_for_test, fail_once_for_test};
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

#[test]
fn ref_log_first_append_failure_is_retryable() -> prikk_error::Result<()> {
    let root = unique_temp_dir("ref-log-append-failure");
    let layout = RepositoryLayout::init(root.clone())?;
    let target = sample_object_id("target");
    let state = sample_object_id("state");
    let envelope = signed_ref_update_envelope("heads/main", None, state, target, 1);

    fail_once_for_test(TestFailPoint::AppendWrite);
    assert!(log::append_log_record(&layout, "heads/main", &envelope).is_err());
    assert_eq!(log::replay_log(&layout, "heads/main")?.records.len(), 0);
    assert!(log::append_log_record(&layout, "heads/main", &envelope).is_ok());
    assert_eq!(log::replay_log(&layout, "heads/main")?.records.len(), 1);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn existing_ref_log_append_failure_is_retryable() -> prikk_error::Result<()> {
    let root = unique_temp_dir("ref-log-existing-append-failure");
    let layout = RepositoryLayout::init(root.clone())?;
    let target = sample_object_id("target");
    let first_state = sample_object_id("state-1");
    let second_state = sample_object_id("state-2");
    let first = signed_ref_update_envelope("heads/main", None, first_state, target, 1);
    let second =
        signed_ref_update_envelope("heads/main", Some(first_state), second_state, target, 2);
    assert!(log::append_log_record(&layout, "heads/main", &first).is_ok());

    fail_once_for_test(TestFailPoint::AppendWrite);
    assert!(log::append_log_record(&layout, "heads/main", &second).is_err());
    assert_eq!(log::replay_log(&layout, "heads/main")?.records.len(), 1);
    assert!(log::append_log_record(&layout, "heads/main", &second).is_ok());
    assert_eq!(log::replay_log(&layout, "heads/main")?.records.len(), 2);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn ref_log_file_and_first_directory_sync_failures_retry_without_duplication()
-> prikk_error::Result<()> {
    for point in [
        TestFailPoint::RequiredFileSync,
        TestFailPoint::RequiredDirectorySync,
    ] {
        let root = unique_temp_dir("ref-log-sync-failure");
        let layout = RepositoryLayout::init(root.clone())?;
        let envelope = signed_ref_update_envelope(
            "heads/main",
            None,
            sample_object_id("state"),
            sample_object_id("target"),
            1,
        );
        fail_once_for_test(point);
        assert!(log::append_log_record(&layout, "heads/main", &envelope).is_err());
        assert_eq!(log::replay_log(&layout, "heads/main")?.records.len(), 1);
        assert!(log::append_log_record(&layout, "heads/main", &envelope).is_ok());
        assert_eq!(log::replay_log(&layout, "heads/main")?.records.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }
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

        fail_after_for_test(point, 1);
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
        assert!(std::fs::remove_file(layout.ref_pointer_path("heads/main")).is_ok());

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
        assert!(std::fs::write(layout.ref_log_path("heads/main"), b"partial").is_ok());

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
        assert!(
            !layout
                .object_path(ObjectType::RefState, ref_state_id)
                .exists()
        );
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
        let ref_state_path = layout.object_path(ObjectType::RefState, ref_state_id);
        assert!(std::fs::remove_file(ref_state_path).is_ok());
        let report = verify_repository(&layout);
        assert!(report.is_ok());
        if let Ok(report) = report {
            assert!(report.has_stage_failure());
        }
    }
    let _ = std::fs::remove_dir_all(root);
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
        assert!(std::fs::remove_file(layout.ref_pointer_path("heads/main")).is_ok());
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
        assert!(log::append_log_record(&layout, "heads/topic", &ref_update).is_ok());

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
