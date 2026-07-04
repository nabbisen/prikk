//! Ref publication tests.

use prikk_object::ObjectType;

use super::log;
use crate::{
    FileObjectStore, ObjectWriter, RefLock, RefPublication, RefStore, RepositoryLayout,
    verify_repository,
};

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
        let first = RefLock::acquire(layout.ref_lock_path("heads/main"));
        assert!(first.is_ok());
        let second = RefLock::acquire(layout.ref_lock_path("heads/main"));
        assert!(second.is_err());
        drop(first);
        let third = RefLock::acquire(layout.ref_lock_path("heads/main"));
        assert!(third.is_ok());
    }
    let _ = std::fs::remove_dir_all(root);
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
            assert_eq!(report.checked_refs, 1);
            assert_eq!(report.checked_ref_log_records, 1);
        }
    }
    let _ = std::fs::remove_dir_all(root);
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

        let second_ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
        let second_ref_state_id = second_ref_state.object_id();
        let second_ref_update =
            signed_ref_update_envelope("heads/main", None, second_ref_state_id, target, 1);
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
        assert!(verify_repository(&layout).is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ref_store_reconstructs_missing_pointer_from_log() {
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
        assert!(repair.is_ok());
        if let Ok(repair) = repair {
            assert!(repair.wrote_pointer);
            assert_eq!(repair.ref_state_id, ref_state_id);
        }
        assert_eq!(
            store.read_current_ref_state_id("heads/main"),
            Ok(Some(ref_state_id))
        );
        assert!(verify_repository(&layout).is_ok());
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
            assert_eq!(report.checked_refs, 0);
            assert_eq!(report.checked_ref_log_records, 1);
        }

        let repair = store.reconstruct_missing_ref_from_log("heads/topic");
        assert!(repair.is_ok());
        if let Ok(repair) = repair {
            assert!(repair.wrote_pointer);
        }
        assert_eq!(
            store.read_current_ref_state_id("heads/topic"),
            Ok(Some(ref_state_id))
        );
    }
    let _ = std::fs::remove_dir_all(root);
}
