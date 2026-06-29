//! Active-session tests.

use crate::{ActiveLock, ActiveSession, RepositoryLayout, Wal};

use crate::test_support::{signed_patch_envelope, unique_temp_dir};

#[test]
fn active_lock_rejects_second_writer() {
    let root = unique_temp_dir("lock");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let first = ActiveLock::acquire(layout.default_active_lock_path());
        assert!(first.is_ok());
        let second = ActiveLock::acquire(layout.default_active_lock_path());
        assert!(second.is_err());
        drop(first);
        let third = ActiveLock::acquire(layout.default_active_lock_path());
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
        let result = session.append_patch(&envelope);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.wal_sequence, 1);
        }
        let wal = Wal::new(layout.default_queue_wal_path());
        let replay = wal.replay();
        assert!(replay.is_ok());
        if let Ok(replay) = replay {
            assert_eq!(replay.records.len(), 1);
            assert_eq!(
                replay.records.first().map(|record| &record.envelope),
                Some(&envelope)
            );
        }
    }
    let _ = std::fs::remove_dir_all(root);
}
