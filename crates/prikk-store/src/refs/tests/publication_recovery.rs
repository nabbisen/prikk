//! DC-38 pointer-first publication and bounded recovery evidence.

mod candidate_cleanup;
mod compatibility;
mod failpoints;
mod partial_tail_refusal;
mod state_matrix;

use std::io::Write;

use super::super::log;
use crate::fsutil::{TestFailPoint, fail_after_for_test, fail_once_for_test};
use crate::test_support::{
    signed_empty_block_envelope, signed_patch_envelope, signed_ref_state_envelope,
    signed_ref_update_envelope, unique_temp_dir,
};
use crate::{
    ActiveSession, DEFAULT_ACTIVE_PATCH_LIMIT, DoctorRepairOptions, FileObjectStore, ObjectWriter,
    RefPublication, RefStore, RepositoryLayout, add_trusted_maintainer, doctor_repository,
    repair_repository, verify_repository,
};

fn root_publication(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> prikk_error::Result<RefPublication> {
    let block = signed_empty_block_envelope();
    let target = FileObjectStore::new(layout.clone()).write_object(&block)?;
    let ref_state = signed_ref_state_envelope(ref_name, None, target, 1);
    let ref_state_id = ref_state.object_id();
    Ok(RefPublication {
        ref_name: ref_name.to_string(),
        expected_previous_ref_state_id: None,
        ref_update: signed_ref_update_envelope(ref_name, None, ref_state_id, target, 1),
        ref_state,
    })
}

fn next_publication(
    layout: &RepositoryLayout,
    previous: &RefPublication,
) -> prikk_error::Result<RefPublication> {
    let previous_id = previous.ref_state.object_id();
    let block = signed_empty_block_envelope();
    let target = FileObjectStore::new(layout.clone()).write_object(&block)?;
    let ref_state = signed_ref_state_envelope("heads/main", Some(previous_id), target, 2);
    let ref_state_id = ref_state.object_id();
    Ok(RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: Some(previous_id),
        ref_update: signed_ref_update_envelope(
            "heads/main",
            Some(previous_id),
            ref_state_id,
            target,
            2,
        ),
        ref_state,
    })
}

fn assert_blocking_issue(layout: &RepositoryLayout, code: &str) -> prikk_error::Result<()> {
    let report = verify_repository(layout)?;
    assert!(report.has_blocking_ref_publication_issues());
    assert!(
        report
            .ref_publication_issues
            .iter()
            .any(|issue| issue.code == code)
    );
    assert!(!doctor_repository(layout).is_healthy());
    Ok(())
}

#[test]
fn candidate_failure_warns_and_retry_publishes_once() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-candidate-retry");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    let store = RefStore::new(layout.clone());
    fail_once_for_test(TestFailPoint::MutableParentSync);
    assert!(store.publish(&publication).is_err());
    let report = verify_repository(&layout)?;
    assert!(!report.has_blocking_ref_publication_issues());
    assert!(
        report
            .ref_publication_issues
            .iter()
            .any(|issue| issue.code == "PRIKK-VERIFY-REF-CANDIDATE-DEBRIS")
    );
    assert!(
        ActiveSession::new(layout.clone())
            .append_patch(&signed_patch_envelope(), DEFAULT_ACTIVE_PATCH_LIMIT)
            .is_err()
    );
    assert!(
        add_trusted_maintainer(
            &layout,
            "blocked-maintainer",
            "1111111111111111111111111111111111111111111111111111111111111111"
        )
        .is_err()
    );
    assert!(repair_repository(&layout, DoctorRepairOptions::truncate_wal_tail()).is_err());
    assert_eq!(
        store.finish_interrupted_publication_for_test(&publication)?,
        publication.ref_state.object_id()
    );
    assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn pointer_sync_failure_is_blocking_and_repeated_retry_appends_once() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-pointer-leading");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    let state_id = publication.ref_state.object_id();
    let store = RefStore::new(layout.clone());
    fail_once_for_test(TestFailPoint::PromotionDestinationSync);
    assert!(store.publish(&publication).is_err());
    assert_eq!(
        store.read_current_ref_state_id("heads/main")?,
        Some(state_id)
    );
    assert_eq!(store.replay_log("heads/main")?.records.len(), 0);
    assert_blocking_issue(&layout, "PRIKK-VERIFY-REF-DIVERGENCE")?;
    assert_eq!(
        store.finish_interrupted_publication_for_test(&publication)?,
        state_id
    );
    assert_eq!(
        store.finish_interrupted_publication_for_test(&publication)?,
        state_id
    );
    assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
    assert!(!verify_repository(&layout)?.has_blocking_ref_publication_issues());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn complete_record_sync_failure_retries_without_duplicate() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-complete-log-retry");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    let store = RefStore::new(layout.clone());
    fail_after_for_test(TestFailPoint::RequiredFileSync, 1);
    assert!(store.publish(&publication).is_err());
    assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
    assert!(!verify_repository(&layout)?.has_blocking_ref_publication_issues());
    store.finish_interrupted_publication_for_test(&publication)?;
    store.finish_interrupted_publication_for_test(&publication)?;
    assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn pointer_lead_with_partial_tail_is_truncated_then_completed() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-partial-log-retry");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    let store = RefStore::new(layout.clone());
    fail_once_for_test(TestFailPoint::AppendWrite);
    assert!(store.publish(&publication).is_err());
    std::fs::OpenOptions::new()
        .append(true)
        .open(layout.ref_log_path("heads/main"))?
        .write_all(b"PREF")?;
    assert_blocking_issue(&layout, "PRIKK-VERIFY-REF-DIVERGENCE")?;
    store.finish_interrupted_publication_for_test(&publication)?;
    assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
    assert_eq!(store.replay_log("heads/main")?.trailing_partial_bytes, 0);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn fully_framed_checksum_failure_is_never_truncated() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-checksum-refusal");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    let store = RefStore::new(layout.clone());
    store.publish(&publication)?;
    let path = layout.ref_log_path("heads/main");
    let mut bytes = std::fs::read(&path)?;
    let last = bytes.last_mut().ok_or_else(|| {
        prikk_error::PrikkError::Integrity("expected a complete ref-log record".to_string())
    })?;
    *last ^= 0xff;
    std::fs::write(path, bytes)?;

    // DC-95 Stage 2 Level 2: a single ref's own checksum corruption is now an item-level failure
    // (its log file's own read), not a whole-`Refs`-stage failure.
    assert!(verify_repository(&layout)?.has_item_failure());
    assert!(!doctor_repository(&layout).is_healthy());
    assert!(
        store
            .finish_interrupted_publication_for_test(&publication)
            .is_err()
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn format2_ahead_log_refuses_pointer_promotion() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-legacy-ahead");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    let mut objects = FileObjectStore::new(layout.clone());
    objects.write_object(&publication.ref_state)?;
    log::append_log_record(&layout, "heads/main", &publication.ref_update)?;
    let store = RefStore::new(layout.clone());
    assert_blocking_issue(&layout, "PRIKK-VERIFY-REF-DIVERGENCE")?;
    assert!(
        store
            .finish_interrupted_publication_for_test(&publication)
            .is_err()
    );
    assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
    assert_eq!(store.read_current_ref_state_id("heads/main")?, None);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn existing_ref_pointer_lead_finishes_but_format2_ahead_log_refuses() -> prikk_error::Result<()> {
    for legacy_ahead in [false, true] {
        let root = unique_temp_dir("dc38-existing-recovery");
        let layout = RepositoryLayout::init(root.clone())?;
        let first = root_publication(&layout, "heads/main")?;
        let store = RefStore::new(layout.clone());
        store.publish(&first)?;
        let second = next_publication(&layout, &first)?;
        FileObjectStore::new(layout.clone()).write_object(&second.ref_state)?;
        if legacy_ahead {
            log::append_log_record(&layout, "heads/main", &second.ref_update)?;
            assert_blocking_issue(&layout, "PRIKK-VERIFY-REF-DIVERGENCE")?;
        } else {
            fail_once_for_test(TestFailPoint::PromotionDestinationSync);
            assert!(store.publish(&second).is_err());
            assert_blocking_issue(&layout, "PRIKK-VERIFY-REF-DIVERGENCE")?;
        }
        if legacy_ahead {
            assert!(
                store
                    .finish_interrupted_publication_for_test(&second)
                    .is_err()
            );
            assert_eq!(
                store.read_current_ref_state_id("heads/main")?,
                Some(first.ref_state.object_id())
            );
        } else {
            store.finish_interrupted_publication_for_test(&second)?;
            store.finish_interrupted_publication_for_test(&second)?;
            assert_eq!(store.replay_log("heads/main")?.records.len(), 2);
            assert_eq!(
                store.read_current_ref_state_id("heads/main")?,
                Some(second.ref_state.object_id())
            );
        }
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn duplicate_and_greater_than_one_log_divergence_fail_closed() -> prikk_error::Result<()> {
    for duplicate in [false, true] {
        let root = unique_temp_dir("dc38-divergence");
        let layout = RepositoryLayout::init(root.clone())?;
        let first = root_publication(&layout, "heads/main")?;
        let mut objects = FileObjectStore::new(layout.clone());
        objects.write_object(&first.ref_state)?;
        log::append_log_record(&layout, "heads/main", &first.ref_update)?;
        if duplicate {
            let path = layout.ref_log_path("heads/main");
            let record = std::fs::read(&path)?;
            std::fs::OpenOptions::new()
                .append(true)
                .open(path)?
                .write_all(&record)?;
        } else {
            let second = next_publication(&layout, &first)?;
            objects.write_object(&second.ref_state)?;
            log::append_log_record(&layout, "heads/main", &second.ref_update)?;
        }
        // DC-95 Stage 2 Level 2: a single ref's own duplicate/divergent log record is now an
        // item-level failure (its log file's own read, or its own `classify_ref_state` call), not
        // a whole-`Refs`-stage failure.
        assert!(verify_repository(&layout)?.has_item_failure());
        assert!(!doctor_repository(&layout).is_healthy());
        assert!(
            RefStore::new(layout.clone())
                .finish_interrupted_publication_for_test(&first)
                .is_err()
        );
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn ref_log_sequence_gap_fails_closed() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-sequence-gap");
    let layout = RepositoryLayout::init(root.clone())?;
    let first = root_publication(&layout, "heads/main")?;
    let first_id = first.ref_state.object_id();
    let mut objects = FileObjectStore::new(layout.clone());
    objects.write_object(&first.ref_state)?;
    log::append_log_record(&layout, "heads/main", &first.ref_update)?;
    let target = objects.write_object(&signed_empty_block_envelope())?;
    let gap_state = signed_ref_state_envelope("heads/main", Some(first_id), target, 3);
    let gap_id = objects.write_object(&gap_state)?;
    let gap_update = signed_ref_update_envelope("heads/main", Some(first_id), gap_id, target, 3);
    log::append_log_record(&layout, "heads/main", &gap_update)?;

    // DC-95 Stage 2 Level 2: this ref's own sequence-gap defect is now an item-level failure (its
    // log file's own read), not a whole-`Refs`-stage failure.
    assert!(verify_repository(&layout)?.has_item_failure());
    assert!(!doctor_repository(&layout).is_healthy());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn doctor_missing_pointer_repair_is_refused() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-doctor-refusal");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    FileObjectStore::new(layout.clone()).write_object(&publication.ref_state)?;
    log::append_log_record(&layout, "heads/main", &publication.ref_update)?;
    let error = repair_repository(&layout, DoctorRepairOptions::reconstruct_main_ref())
        .err()
        .ok_or_else(|| {
            prikk_error::PrikkError::Integrity(
                "doctor missing-pointer repair unexpectedly succeeded".to_string(),
            )
        })?;
    assert!(error.to_string().contains("unsupported in 0.18.0"));
    assert_eq!(
        RefStore::new(layout.clone()).read_current_ref_state_id("heads/main")?,
        None
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
