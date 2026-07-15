//! Candidate, pointer-promotion, and truncation failpoint cases.

use std::io::Write;

use super::root_publication;
use crate::fsutil::{TestFailPoint, fail_once_for_test};
use crate::test_support::unique_temp_dir;
use crate::{FileObjectStore, ObjectReader, RefStore, RepositoryLayout, verify_repository};

#[test]
fn object_finalization_failures_precede_pointer_and_log_mutation() -> prikk_error::Result<()> {
    for (point, installed_after_error) in [
        (TestFailPoint::ImmutableFileSync, false),
        (TestFailPoint::ImmutableInstallSync, true),
    ] {
        let root = unique_temp_dir("dc38-object-finalization-retry");
        let layout = RepositoryLayout::init(root.clone())?;
        let publication = root_publication(&layout, "heads/main")?;
        let state_id = publication.ref_state.object_id();
        let store = RefStore::new(layout.clone());

        fail_once_for_test(point);
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
fn pointer_promotion_failures_retry_to_one_log_record() -> prikk_error::Result<()> {
    for point in [
        TestFailPoint::PromotionRename,
        TestFailPoint::PromotionSourceSync,
    ] {
        let root = unique_temp_dir("dc38-pointer-promotion-retry");
        let layout = RepositoryLayout::init(root.clone())?;
        let publication = root_publication(&layout, "heads/main")?;
        let store = RefStore::new(layout.clone());
        fail_once_for_test(point);
        assert!(store.publish(&publication).is_err());
        store.finish_interrupted_publication_for_test(&publication)?;
        store.finish_interrupted_publication_for_test(&publication)?;
        assert_eq!(store.replay_log("heads/main")?.records.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn partial_tail_truncate_failure_preserves_state_for_retry() -> prikk_error::Result<()> {
    let root = unique_temp_dir("dc38-partial-truncate-retry");
    let layout = RepositoryLayout::init(root.clone())?;
    let publication = root_publication(&layout, "heads/main")?;
    let store = RefStore::new(layout.clone());
    fail_once_for_test(TestFailPoint::AppendWrite);
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
