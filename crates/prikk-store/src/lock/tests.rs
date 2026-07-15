use std::fs;

use crate::fsutil::{TestFailPoint, fail_once_for_test};
use crate::test_support::unique_temp_dir;
use crate::{ActiveLock, RepositoryLayout};

#[test]
fn failed_lock_directory_sync_retains_stale_lock() {
    let root = unique_temp_dir("lock-directory-sync-failure");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        let path = layout.default_active_lock_path();
        fail_once_for_test(TestFailPoint::RequiredDirectorySync);
        assert!(ActiveLock::acquire(&layout).is_err());
        assert!(path.is_file());
        assert!(ActiveLock::acquire(&layout).is_err());
        let _ = fs::remove_file(path);
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn failed_lock_file_sync_retains_stale_lock() -> prikk_error::Result<()> {
    let root = unique_temp_dir("lock-file-sync-failure");
    let layout = RepositoryLayout::init(root.clone())?;
    let path = layout.default_active_lock_path();
    fail_once_for_test(TestFailPoint::RequiredFileSync);
    assert!(ActiveLock::acquire(&layout).is_err());
    assert!(path.is_file());
    assert!(ActiveLock::acquire(&layout).is_err());
    let _ = fs::remove_file(path);
    let _ = fs::remove_dir_all(root);
    Ok(())
}
