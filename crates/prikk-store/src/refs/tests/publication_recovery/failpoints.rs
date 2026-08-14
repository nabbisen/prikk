//! Candidate, pointer-promotion, and truncation failpoint cases.

use std::io::Write;

use super::root_publication;
use crate::fsutil::{TestFailPoint, fail_after_for_test, fail_once_for_test};
use crate::test_support::unique_temp_dir;
use crate::{FileObjectStore, ObjectReader, RefStore, RepositoryLayout, verify_repository};

#[test]
fn object_finalization_failures_precede_pointer_and_log_mutation() -> prikk_error::Result<()> {
    // RFC 102 Stage 3: the ref-state object write no longer goes through the old immutable-install
    // primitive (`ImmutableFileSync`/`ImmutableInstallSync`) at all -- it durably appends to its
    // container, then to the index, both through the same `RequiredFileSync` point the ref lock's
    // own creation already uses once. Skip 1 to land on the container append (the object is not
    // even durably present); skip 2 to land on the index append instead (the container record is
    // already durable, and the index write's own bytes are already on disk -- only its own sync is
    // interrupted -- so the object is visible to a same-process read exactly as the old
    // ImmutableInstallSync case's post-install cleanup-sync failure left it visible).
    for (skip, installed_after_error) in [(1, false), (2, true)] {
        let root = unique_temp_dir("dc38-object-finalization-retry");
        let layout = RepositoryLayout::init(root.clone())?;
        let publication = root_publication(&layout, "heads/main")?;
        let state_id = publication.ref_state.object_id();
        let store = RefStore::new(layout.clone());

        fail_after_for_test(TestFailPoint::RequiredFileSync, skip);
        assert!(store.publish(&publication).is_err());
        assert_eq!(
            FileObjectStore::new(layout.clone())
                .read_object(state_id)?
                .is_some(),
            installed_after_error
        );
        assert_eq!(store.read_current_ref_state_id("heads/main")?, None);
        assert_eq!(store.replay_log("heads/main")?.records.len(), 0);

        store.publish(&publication)?;
        assert_eq!(
            store.read_current_ref_state_id("heads/main")?,
            Some(state_id)
        );
        assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn candidate_atomic_write_failures_are_cleaned_by_retry() -> prikk_error::Result<()> {
    for point in [TestFailPoint::MutableFileSync, TestFailPoint::MutableRename] {
        let root = unique_temp_dir("dc38-candidate-atomic-retry");
        let layout = RepositoryLayout::init(root.clone())?;
        let publication = root_publication(&layout, "heads/main")?;
        let store = RefStore::new(layout.clone());
        fail_once_for_test(point);
        assert!(store.publish(&publication).is_err());
        assert!(
            verify_repository(&layout)?
                .ref_publication_issues
                .iter()
                .any(|issue| issue.code == "PRIKK-VERIFY-REF-CANDIDATE-DEBRIS")
        );
        store.finish_interrupted_publication_for_test(&publication)?;
        assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
        assert!(
            verify_repository(&layout)?
                .ref_publication_issues
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn pointer_rename_failure_leaves_unmoved_pointer_with_candidate_debris() -> prikk_error::Result<()>
{
    let root = unique_temp_dir("dc38-pointer-rename-retry");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    let store = RefStore::new(layout.clone());
    fail_once_for_test(TestFailPoint::PromotionRename);
    assert!(store.publish(&publication).is_err());
    assert_eq!(store.read_current_ref_state_id("heads/main")?, None);
    assert!(
        verify_repository(&layout)?
            .ref_publication_issues
            .iter()
            .any(|issue| issue.code == "PRIKK-VERIFY-REF-CANDIDATE-DEBRIS" && !issue.blocking)
    );
    store.finish_interrupted_publication_for_test(&publication)?;
    store.finish_interrupted_publication_for_test(&publication)?;
    assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
    assert_eq!(
        store.read_current_ref_state_id("heads/main")?,
        Some(publication.ref_state.object_id())
    );
    assert!(
        verify_repository(&layout)?
            .ref_publication_issues
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn pointer_source_sync_failure_leaves_committed_pointer_ahead_of_log() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-pointer-source-sync-retry");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    let state_id = publication.ref_state.object_id();
    let store = RefStore::new(layout.clone());
    fail_once_for_test(TestFailPoint::PromotionSourceSync);
    assert!(store.publish(&publication).is_err());
    assert_eq!(
        store.read_current_ref_state_id("heads/main")?,
        Some(state_id)
    );
    assert_eq!(store.replay_log("heads/main")?.records.len(), 0);
    assert!(
        verify_repository(&layout)?
            .ref_publication_issues
            .iter()
            .any(|issue| issue.code == "PRIKK-VERIFY-REF-DIVERGENCE" && issue.blocking)
    );
    store.finish_interrupted_publication_for_test(&publication)?;
    store.finish_interrupted_publication_for_test(&publication)?;
    assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
    assert_eq!(
        store.read_current_ref_state_id("heads/main")?,
        Some(state_id)
    );
    assert!(!verify_repository(&layout)?.has_blocking_ref_publication_issues());
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn partial_tail_truncate_failure_preserves_state_for_retry() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-partial-truncate-retry");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    let store = RefStore::new(layout.clone());
    // RFC 102 Stage 3: skip 2 to land the torn write on the log's own append, past the ref-state
    // object's own container and index appends (see the sibling test above for the full count).
    fail_after_for_test(TestFailPoint::AppendWrite, 2);
    assert!(store.publish(&publication).is_err());
    std::fs::OpenOptions::new()
        .append(true)
        .open(layout.ref_log_path("heads/main"))?
        .write_all(b"PREF")?;
    fail_once_for_test(TestFailPoint::Truncate);
    assert!(
        store
            .finish_interrupted_publication_for_test(&publication)
            .is_err()
    );
    assert_ne!(store.replay_log("heads/main")?.trailing_partial_bytes, 0);
    store.finish_interrupted_publication_for_test(&publication)?;
    assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
