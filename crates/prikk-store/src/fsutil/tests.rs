use std::fs;
use std::path::Path;

mod directory;

use crate::RepositoryLayout;
use crate::test_support::unique_temp_dir;

use super::{
    MutationRoot, TestFailPoint, append_file_required, ensure_directory_required,
    fail_once_for_test, promote_file_required, remove_file_required,
    truncate_existing_file_required, write_file_atomically, write_worktree_file_atomically,
};

fn mutation_root(path: &Path) -> MutationRoot {
    match MutationRoot::open(path) {
        Ok(root) => root,
        Err(error) => panic!("test mutation root failed: {error}"),
    }
}

#[test]
fn root_capability_remains_bound_after_path_replacement() {
    let path = unique_temp_dir("root-replacement");
    let replacement = path.with_extension("replacement");
    let root = mutation_root(&path);
    assert!(fs::rename(&path, &replacement).is_ok());
    assert!(fs::create_dir(&path).is_ok());
    assert!(write_file_atomically(&root, Path::new("state"), b"bound").is_ok());
    assert_eq!(
        fs::read(replacement.join("state")).unwrap_or_default(),
        b"bound"
    );
    assert!(!path.join("state").exists());
    let _ = fs::remove_dir_all(path);
    let _ = fs::remove_dir_all(replacement);
}

#[test]
fn worktree_writer_remains_bound_after_root_replacement() {
    let path = unique_temp_dir("worktree-root-replacement");
    let replacement = path.with_extension("replacement");
    let root = mutation_root(&path);
    assert!(fs::rename(&path, &replacement).is_ok());
    assert!(fs::create_dir(&path).is_ok());
    assert!(write_worktree_file_atomically(&root, Path::new("file"), b"bound").is_ok());
    assert_eq!(
        fs::read(replacement.join("file")).unwrap_or_default(),
        b"bound"
    );
    assert!(!path.join("file").exists());
    let _ = fs::remove_dir_all(path);
    let _ = fs::remove_dir_all(replacement);
}

#[test]
fn worktree_write_sync_failure_retains_file_and_is_retryable() {
    let path = unique_temp_dir("worktree-write-failure");
    let root = mutation_root(&path);
    fail_once_for_test(TestFailPoint::MutableParentSync);
    assert!(write_worktree_file_atomically(&root, Path::new("file"), b"retained").is_err());
    assert_eq!(fs::read(path.join("file")).unwrap_or_default(), b"retained");
    assert!(write_worktree_file_atomically(&root, Path::new("file"), b"retry").is_ok());
    assert_eq!(fs::read(path.join("file")).unwrap_or_default(), b"retry");
    let _ = fs::remove_dir_all(path);
}

#[test]
fn repository_layout_remains_bound_after_prikk_replacement() {
    let path = unique_temp_dir("repository-root-replacement");
    let layout = match RepositoryLayout::init(path.clone()) {
        Ok(layout) => layout,
        Err(error) => panic!("repository layout failed: {error}"),
    };
    let displaced = path.join(".prikk-displaced");
    assert!(fs::rename(layout.prikk_dir(), &displaced).is_ok());
    assert!(fs::create_dir(layout.prikk_dir()).is_ok());
    assert!(
        write_file_atomically(
            layout.repository_mutation_root(),
            Path::new("authority-test"),
            b"bound"
        )
        .is_ok()
    );
    assert_eq!(
        fs::read(displaced.join("authority-test")).unwrap_or_default(),
        b"bound"
    );
    assert!(!layout.prikk_dir().join("authority-test").exists());
    let _ = fs::remove_dir_all(path);
}

#[test]
fn required_open_failure_has_no_side_effect_and_is_retryable() {
    let path = unique_temp_dir("required-open-failure");
    let root = mutation_root(&path);
    fail_once_for_test(TestFailPoint::RequiredOpen);
    assert!(write_file_atomically(&root, Path::new("state"), b"candidate").is_err());
    assert!(!path.join("state").exists());
    assert!(write_file_atomically(&root, Path::new("state"), b"retry").is_ok());
    let _ = fs::remove_dir_all(path);
}

#[test]
fn mutable_atomic_write_replaces_complete_content() {
    let path = unique_temp_dir("required-atomic-write");
    let root = mutation_root(&path);
    assert!(write_file_atomically(&root, Path::new("state"), b"first").is_ok());
    assert!(write_file_atomically(&root, Path::new("state"), b"second").is_ok());
    assert_eq!(fs::read(path.join("state")).unwrap_or_default(), b"second");
    let _ = fs::remove_dir_all(path);
}

#[test]
fn failed_mutable_parent_sync_retains_replaced_final_name() {
    let path = unique_temp_dir("required-mutable-sync-failure");
    let root = mutation_root(&path);
    fail_once_for_test(TestFailPoint::MutableParentSync);
    assert!(write_file_atomically(&root, Path::new("state"), b"retained").is_err());
    assert_eq!(
        fs::read(path.join("state")).unwrap_or_default(),
        b"retained"
    );
    assert!(write_file_atomically(&root, Path::new("state"), b"retry").is_ok());
    assert_eq!(fs::read(path.join("state")).unwrap_or_default(), b"retry");
    let _ = fs::remove_dir_all(path);
}

#[test]
fn failed_mutable_file_sync_keeps_only_non_authoritative_temp() {
    let path = unique_temp_dir("required-mutable-file-failure");
    let root = mutation_root(&path);
    fail_once_for_test(TestFailPoint::MutableFileSync);
    assert!(write_file_atomically(&root, Path::new("state"), b"candidate").is_err());
    assert!(!path.join("state").exists());
    let debris = fs::read_dir(&path)
        .map(|entries| entries.filter_map(std::result::Result::ok).count())
        .unwrap_or_default();
    assert_eq!(debris, 1);
    let _ = fs::remove_dir_all(path);
}

#[test]
fn failed_mutable_rename_keeps_previous_authoritative_state() {
    let path = unique_temp_dir("required-mutable-rename-failure");
    let root = mutation_root(&path);
    assert!(write_file_atomically(&root, Path::new("state"), b"previous").is_ok());
    fail_once_for_test(TestFailPoint::MutableRename);
    assert!(write_file_atomically(&root, Path::new("state"), b"candidate").is_err());
    assert_eq!(
        fs::read(path.join("state")).unwrap_or_default(),
        b"previous"
    );
    let _ = fs::remove_dir_all(path);
}

#[test]
fn failed_append_write_is_retryable() {
    let path = unique_temp_dir("append-write-failure");
    let root = mutation_root(&path);
    fail_once_for_test(TestFailPoint::AppendWrite);
    assert!(append_file_required(&root, Path::new("log"), b"record").is_err());
    assert_eq!(fs::read(path.join("log")).unwrap_or_default(), b"");
    assert!(append_file_required(&root, Path::new("log"), b"record").is_ok());
    assert_eq!(fs::read(path.join("log")).unwrap_or_default(), b"record");
    let _ = fs::remove_dir_all(path);
}

#[test]
fn failed_truncate_retains_previous_state_and_is_retryable() {
    let path = unique_temp_dir("truncate-failure");
    let root = mutation_root(&path);
    assert!(fs::write(path.join("wal"), b"complete-partial").is_ok());
    fail_once_for_test(TestFailPoint::Truncate);
    assert!(truncate_existing_file_required(&root, Path::new("wal"), 8).is_err());
    assert_eq!(
        fs::read(path.join("wal")).unwrap_or_default(),
        b"complete-partial"
    );
    assert!(truncate_existing_file_required(&root, Path::new("wal"), 8).is_ok());
    assert_eq!(fs::read(path.join("wal")).unwrap_or_default(), b"complete");
    let _ = fs::remove_dir_all(path);
}

#[test]
fn failed_unlink_retains_file_and_cleanup_sync_reports_removed_state() {
    let path = unique_temp_dir("unlink-failure");
    let root = mutation_root(&path);
    assert!(fs::write(path.join("entry"), b"state").is_ok());
    fail_once_for_test(TestFailPoint::Unlink);
    assert!(remove_file_required(&root, Path::new("entry")).is_err());
    assert!(path.join("entry").is_file());
    fail_once_for_test(TestFailPoint::CleanupDirectorySync);
    assert!(super::remove_worktree_file_required(&root, Path::new("entry")).is_err());
    assert!(!path.join("entry").exists());
    let _ = fs::remove_dir_all(path);
}

#[cfg(target_os = "linux")]
#[test]
fn append_and_truncate_reject_fifo_without_blocking() {
    use std::sync::mpsc;
    use std::time::Duration;

    use rustix::fs::{CWD, Mode, mkfifoat};

    let path = unique_temp_dir("fifo-final-entry");
    let fifo = path.join("wal");
    assert!(mkfifoat(CWD, &fifo, Mode::from_raw_mode(0o600)).is_ok());
    let root = mutation_root(&path);
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let append = append_file_required(&root, Path::new("wal"), b"record");
        let truncate = truncate_existing_file_required(&root, Path::new("wal"), 0);
        let _ = sender.send((append, truncate));
    });
    let result = receiver.recv_timeout(Duration::from_secs(1));
    assert!(result.is_ok(), "special-file rejection must be bounded");
    if let Ok((append, truncate)) = result {
        assert!(append.is_err());
        assert!(truncate.is_err());
    }
    let _ = fs::remove_file(fifo);
    let _ = fs::remove_dir_all(path);
}

#[cfg(not(target_os = "linux"))]
#[test]
fn unsupported_mutation_fails_before_filesystem_side_effect() {
    let path = unique_temp_dir("unsupported-mutation");
    let root = mutation_root(&path);
    assert!(write_file_atomically(&root, Path::new("state"), b"candidate").is_err());
    assert!(!path.join("state").exists());
    let _ = fs::remove_dir_all(path);
}

#[test]
fn promotion_destination_sync_failure_retains_destination_state() {
    let path = unique_temp_dir("required-promotion-destination");
    let root = mutation_root(&path);
    assert!(ensure_directory_required(&root, Path::new("source")).is_ok());
    assert!(ensure_directory_required(&root, Path::new("destination")).is_ok());
    assert!(write_file_atomically(&root, Path::new("source/candidate"), b"pointer").is_ok());
    fail_once_for_test(TestFailPoint::PromotionDestinationSync);
    assert!(
        promote_file_required(
            &root,
            Path::new("source/candidate"),
            Path::new("destination/pointer")
        )
        .is_err()
    );
    assert!(!path.join("source/candidate").exists());
    assert_eq!(
        fs::read(path.join("destination/pointer")).unwrap_or_default(),
        b"pointer"
    );
    let _ = fs::remove_dir_all(path);
}

#[test]
fn promotion_rename_failure_retains_source_only() {
    let path = unique_temp_dir("required-promotion-rename");
    let root = mutation_root(&path);
    assert!(ensure_directory_required(&root, Path::new("source")).is_ok());
    assert!(ensure_directory_required(&root, Path::new("destination")).is_ok());
    assert!(write_file_atomically(&root, Path::new("source/candidate"), b"pointer").is_ok());
    fail_once_for_test(TestFailPoint::PromotionRename);
    assert!(
        promote_file_required(
            &root,
            Path::new("source/candidate"),
            Path::new("destination/pointer")
        )
        .is_err()
    );
    assert_eq!(
        fs::read(path.join("source/candidate")).unwrap_or_default(),
        b"pointer"
    );
    assert!(!path.join("destination/pointer").exists());
    let _ = fs::remove_dir_all(path);
}

#[test]
fn promotion_source_sync_failure_reports_committed_destination() {
    let path = unique_temp_dir("required-promotion-source");
    let root = mutation_root(&path);
    assert!(ensure_directory_required(&root, Path::new("source")).is_ok());
    assert!(ensure_directory_required(&root, Path::new("destination")).is_ok());
    assert!(write_file_atomically(&root, Path::new("source/candidate"), b"pointer").is_ok());
    fail_once_for_test(TestFailPoint::PromotionSourceSync);
    assert!(
        promote_file_required(
            &root,
            Path::new("source/candidate"),
            Path::new("destination/pointer")
        )
        .is_err()
    );
    assert!(!path.join("source/candidate").exists());
    assert_eq!(
        fs::read(path.join("destination/pointer")).unwrap_or_default(),
        b"pointer"
    );
    let _ = fs::remove_dir_all(path);
}
