//! Active-session tests.

#![allow(clippy::unwrap_used)]

use crate::{
    ActiveLock, ActiveRefMetadata, ActiveSession, DEFAULT_ACTIVE_PATCH_LIMIT, RepositoryLayout,
    Wal, finish_active_publication_cleanup, read_active_ref_metadata, remove_active_ref_metadata,
    write_active_ref_metadata,
};

use crate::fsutil::{TestFailPoint, fail_once_for_test};
use crate::test_support::{rollback_patch_envelope, signed_patch_envelope, unique_temp_dir};

mod format_transition;

#[test]
fn active_lock_rejects_second_writer() {
    let root = unique_temp_dir("lock");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let first = ActiveLock::acquire(&layout);
        assert!(first.is_ok());
        let second = ActiveLock::acquire(&layout);
        assert!(second.is_err());
        drop(first);
        let third = ActiveLock::acquire(&layout);
        assert!(third.is_ok());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn active_session_appends_signed_patch_under_lock() {
    let root = unique_temp_dir("active-session");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let session = ActiveSession::new(layout.clone());
        let envelope = signed_patch_envelope();
        let result = session.append_patch(&envelope, DEFAULT_ACTIVE_PATCH_LIMIT);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.wal_sequence, 1);
        }
        let wal = Wal::for_layout(&layout);
        let replay = wal.replay();
        assert!(replay.is_ok());
        if let Ok(replay) = replay {
            assert_eq!(replay.records.len(), 1);
            assert_eq!(
                replay.records.first().map(|record| &record.envelope),
                Some(&envelope)
            );
        }
        assert_eq!(
            read_active_ref_metadata(&layout).unwrap(),
            ActiveRefMetadata::Valid("heads/main".to_string())
        );
    }
    let _ = std::fs::remove_dir_all(root);
}

// DC-66: a non-empty active WAL now queues a distinct patch rather than refusing it — this test
// previously asserted the pre-DC-66 reject behavior; updated per the RFC's raised cap (1 -> N).
// Ref-ownership-mismatch coverage for a non-empty queue lives at the production `worktree_patch`
// layer (`node_authoring.rs`'s `author_inner`), which this narrow, currently uncalled-in-production
// helper mirrors but does not itself need to re-cover.
#[test]
fn active_session_append_queues_distinct_patch_onto_non_empty_wal() {
    let root = unique_temp_dir("active-session-nonempty");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    let session = ActiveSession::new(layout.clone());
    let first = session
        .append_patch(&signed_patch_envelope(), DEFAULT_ACTIVE_PATCH_LIMIT)
        .unwrap();
    assert_eq!(first.wal_sequence, 1);

    let second = session
        .append_patch(&rollback_patch_envelope(), DEFAULT_ACTIVE_PATCH_LIMIT)
        .unwrap();
    assert_eq!(second.wal_sequence, 2);

    let replay = Wal::for_layout(&layout).replay().unwrap();
    assert_eq!(replay.records.len(), 2);
    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Valid("heads/main".to_string())
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn active_session_append_rejects_trailing_partial_wal() {
    let root = unique_temp_dir("active-session-partial");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    std::fs::write(layout.default_queue_wal_path(), b"partial").unwrap();
    let session = ActiveSession::new(layout.clone());

    let err = session
        .append_patch(&signed_patch_envelope(), DEFAULT_ACTIVE_PATCH_LIMIT)
        .unwrap_err();
    assert!(
        err.to_string().contains("trailing partial bytes"),
        "unexpected error: {err}"
    );
    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Missing
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn active_session_append_does_not_overwrite_other_ref_metadata() {
    let root = unique_temp_dir("active-session-owned");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    write_active_ref_metadata(&layout, "heads/topic").unwrap();
    Wal::for_layout(&layout)
        .append_patch(&signed_patch_envelope())
        .unwrap();
    let session = ActiveSession::new(layout.clone());

    let err = session
        .append_patch(&signed_patch_envelope(), DEFAULT_ACTIVE_PATCH_LIMIT)
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("active WAL is owned by heads/topic"),
        "unexpected error: {err}"
    );
    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Valid("heads/topic".to_string())
    );
    let replay = Wal::for_layout(&layout).replay().unwrap();
    assert_eq!(replay.records.len(), 1);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn active_ref_metadata_round_trips_and_removes() {
    let root = unique_temp_dir("active-ref");
    let layout = RepositoryLayout::init(root.clone()).unwrap();

    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Missing
    );
    assert_eq!(
        write_active_ref_metadata(&layout, "heads/topic").unwrap(),
        "heads/topic"
    );
    assert_eq!(
        std::fs::read(layout.default_active_ref_name_path()).unwrap(),
        b"heads/topic"
    );
    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Valid("heads/topic".to_string())
    );
    assert!(remove_active_ref_metadata(&layout).unwrap());
    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Missing
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn active_ref_metadata_reports_malformed_content() {
    let root = unique_temp_dir("active-ref-bad");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    std::fs::write(layout.default_active_ref_name_path(), b"heads//bad").unwrap();

    assert!(matches!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Invalid(_)
    ));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn active_metadata_write_and_removal_failures_pin_retained_state() {
    let root = unique_temp_dir("active-ref-failpoints");
    let layout = RepositoryLayout::init(root.clone()).unwrap();

    fail_once_for_test(TestFailPoint::MutableParentSync);
    assert!(write_active_ref_metadata(&layout, "heads/topic").is_err());
    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Valid("heads/topic".to_string())
    );
    fail_once_for_test(TestFailPoint::CleanupDirectorySync);
    assert!(remove_active_ref_metadata(&layout).is_err());
    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Missing
    );
    fail_once_for_test(TestFailPoint::CleanupDirectorySync);
    assert!(remove_active_ref_metadata(&layout).is_err());
    assert!(!remove_active_ref_metadata(&layout).unwrap());

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn active_publication_cleanup_failures_preserve_retryable_states() {
    for point in [
        TestFailPoint::Truncate,
        TestFailPoint::Unlink,
        TestFailPoint::CleanupDirectorySync,
    ] {
        let root = unique_temp_dir("active-publication-cleanup-failpoint");
        let layout = RepositoryLayout::init(root.clone()).unwrap();
        write_active_ref_metadata(&layout, "heads/main").unwrap();
        Wal::for_layout(&layout)
            .append_patch(&signed_patch_envelope())
            .unwrap();
        let wal_before = std::fs::read(layout.default_queue_wal_path()).unwrap();
        let metadata_before = std::fs::read(layout.default_active_ref_name_path()).unwrap();
        let active_lock = ActiveLock::acquire(&layout).unwrap();

        fail_once_for_test(point);
        assert!(finish_active_publication_cleanup(&layout, &active_lock).is_err());
        match point {
            TestFailPoint::Truncate => {
                assert_eq!(
                    std::fs::read(layout.default_queue_wal_path()).unwrap(),
                    wal_before
                );
                assert_eq!(
                    std::fs::read(layout.default_active_ref_name_path()).unwrap(),
                    metadata_before
                );
            }
            TestFailPoint::Unlink => {
                assert!(
                    std::fs::read(layout.default_queue_wal_path())
                        .unwrap()
                        .is_empty()
                );
                assert_eq!(
                    std::fs::read(layout.default_active_ref_name_path()).unwrap(),
                    metadata_before
                );
            }
            TestFailPoint::CleanupDirectorySync => {
                assert!(
                    std::fs::read(layout.default_queue_wal_path())
                        .unwrap()
                        .is_empty()
                );
                assert!(!layout.default_active_ref_name_path().exists());
            }
            _ => unreachable!(),
        }

        finish_active_publication_cleanup(&layout, &active_lock).unwrap();
        assert!(
            std::fs::read(layout.default_queue_wal_path())
                .unwrap()
                .is_empty()
        );
        assert!(!layout.default_active_ref_name_path().exists());
        drop(active_lock);
        let _ = std::fs::remove_dir_all(root);
    }
}

#[test]
fn active_metadata_read_and_write_remain_on_retained_repository_root() {
    let root = unique_temp_dir("active-ref-root-replacement");
    let layout = RepositoryLayout::init(root.clone()).unwrap();
    write_active_ref_metadata(&layout, "heads/original").unwrap();
    let displaced = root.join(".prikk-displaced");
    std::fs::rename(layout.prikk_dir(), &displaced).unwrap();
    std::fs::create_dir_all(root.join(".prikk/active/default")).unwrap();
    std::fs::write(
        root.join(".prikk/active/default/ref-name"),
        b"heads/replacement",
    )
    .unwrap();

    assert_eq!(
        read_active_ref_metadata(&layout).unwrap(),
        ActiveRefMetadata::Valid("heads/original".to_string())
    );
    write_active_ref_metadata(&layout, "heads/updated").unwrap();
    assert_eq!(
        std::fs::read(displaced.join("active/default/ref-name")).unwrap(),
        b"heads/updated"
    );
    assert_eq!(
        std::fs::read(root.join(".prikk/active/default/ref-name")).unwrap(),
        b"heads/replacement"
    );

    let _ = std::fs::remove_dir_all(root);
}
