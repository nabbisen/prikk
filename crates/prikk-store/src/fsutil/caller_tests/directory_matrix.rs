//! Per-caller directory component failure and retry matrix.

use std::path::Path;

use crate::fsutil::{TestFailPoint, fail_once_for_test};
use crate::test_support::unique_temp_dir;
use crate::worktree::materialize_manifest_entries;
use crate::{
    RepoPath, RepositoryLayout, SnapshotEntry, SnapshotManifest, add_trusted_maintainer,
    write_active_ref_metadata,
};

const COMPONENT_POINTS: [TestFailPoint; 2] = [
    TestFailPoint::DirectoryCreate,
    TestFailPoint::CreatedDirectoryParentSync,
];

fn assert_retained_component(point: TestFailPoint, path: &Path) {
    match point {
        TestFailPoint::DirectoryCreate => assert!(!path.exists()),
        TestFailPoint::CreatedDirectoryParentSync => assert!(path.is_dir()),
        _ => panic!("not a component-creation point"),
    }
}

fn remove_empty_directory(path: &Path) -> prikk_error::Result<()> {
    std::fs::remove_dir(path)?;
    Ok(())
}

#[test]
fn repository_initialization_component_matrix() -> prikk_error::Result<()> {
    for point in COMPONENT_POINTS {
        let root = unique_temp_dir("repository-init-component-matrix");
        fail_once_for_test(point);
        assert!(RepositoryLayout::init(root.clone()).is_err());
        assert_retained_component(point, &root.join(".prikk"));
        assert!(RepositoryLayout::init(root.clone()).is_ok());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

// RFC 102 Stage 3: `object_directory_component_matrix` (proving a first-object-write directory
// component failure is retryable) has no container-era equivalent and was removed rather than
// retargeted. Every container and index file is allocated once, at `init`, per
// `layout/tests.rs::init_allocates_every_container_index_and_generation_log_name_once` -- an
// object write never creates a directory component at all anymore, so this matrix's own
// `DirectoryCreate`/`CreatedDirectoryParentSync` failpoints are simply never reached by
// `FileObjectStore::write_object`, and there is no analogous scenario left to prove here.

// RFC 102 Stage 5, design-v1.md §14.3/§14.5: `wal_directory_component_matrix` (proving a
// missing-directory-plus-missing-WAL-file failure during append was retryable) has no
// strict-`durable_append`-era equivalent, and was removed rather than retargeted -- the same
// disposition Stage 3 and Stage 4 gave the two retirement notes above, for the same underlying
// reason arriving through a third door. Its setup deleted `queue.wal` and then its now-empty
// parent directory to force `wal.rs:194`'s defensive `ensure_directory_required` to recreate a
// directory from nothing; under the old lenient `durable_append`, the subsequent append also
// transparently recreated the deleted file. Strict `durable_append` closes exactly that
// transparent recreation (design-v1.md §14.3's own point: it "silently repairs" an interrupted-
// init-shaped state into an undetectable one), so the fixture's constructed state --
// directory absent, file absent -- is no longer one `wal.append_patch` can recover from, and is
// not a state a correctly-`init`ed repository can reach in the first place:
// `default_active_dir()` is in `required_directories()` (`layout.rs:378`) and nothing in this
// codebase ever removes a required directory. `wal.rs:194`'s `ensure_directory_required` call
// is therefore defensive-only for a directory that cannot legitimately be absent, the same
// conclusion already reached for object containers (Stage 3) and ref containers (Stage 4).
// Flagged as a finding for the same reason those two were: verified from the code, not assumed,
// and reported for an affirmative ruling rather than retired silently.

/// RFC 102 Stage 5, design-v1.md §14.6: `init` now also creates `default_active_ref_name_path()`
/// (`active/default/ref-name`) in this same directory, so it must be removed alongside `queue.wal`
/// before the directory is genuinely empty -- the same discipline the `queue.wal` removal below
/// already documents for itself. Still a live scenario, unlike `wal_directory_component_matrix`'s
/// retirement above: `write_active_ref_metadata`'s truncate step goes through `durable_truncate_to_
/// empty`, which (unlike strict `durable_append`) still creates a missing directory via `prepare_
/// directory_required` -- confirmed by reading `linux.rs`'s two truncate primitives side by side, not
/// assumed symmetric with `durable_append`. See the standing finding on that asymmetry in this round's
/// own report.
#[test]
fn active_metadata_directory_component_matrix() -> prikk_error::Result<()> {
    for point in COMPONENT_POINTS {
        let root = unique_temp_dir("active-component-matrix");
        let layout = RepositoryLayout::init(root.clone())?;
        let active_dir = layout.default_active_dir();
        // RFC 102 Stage 1: `init` now creates `queue.wal` itself, so the directory this test needs
        // empty (to exercise `ensure_directory_required`'s own recreation, not the WAL's presence)
        // must have it removed first.
        std::fs::remove_file(layout.default_queue_wal_path())?;
        std::fs::remove_file(layout.default_active_ref_name_path())?;
        remove_empty_directory(&active_dir)?;
        fail_once_for_test(point);
        assert!(write_active_ref_metadata(&layout, "heads/main").is_err());
        assert_retained_component(point, &active_dir);
        assert!(write_active_ref_metadata(&layout, "heads/main").is_ok());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

// RFC 102 Stage 4: `ref_log_directory_component_matrix` (proving a first-ref-publish directory
// component failure on `refs/logs/` is retryable) has no container-era equivalent, retired the
// same way and for the same reason `object_directory_component_matrix` was above. The ref-log
// container is allocated once, at `init`, per `layout/tests.rs::init_allocates_every_ref_
// container_name_once` -- a ref publish never creates a directory component at all anymore, so
// this matrix's own `DirectoryCreate`/`CreatedDirectoryParentSync` failpoints are simply never
// reached by `RefStore::publish`, and there is no analogous scenario left to prove here.

#[test]
fn trust_directory_component_matrix() -> prikk_error::Result<()> {
    for point in COMPONENT_POINTS {
        let root = unique_temp_dir("trust-component-matrix");
        let layout = RepositoryLayout::init(root.clone())?;
        let keys_dir = layout.maintainer_trust_keys_dir();
        remove_empty_directory(&keys_dir)?;
        let key = "0707070707070707070707070707070707070707070707070707070707070707";
        fail_once_for_test(point);
        assert!(add_trusted_maintainer(&layout, "maintainer", key).is_err());
        assert_retained_component(point, &keys_dir);
        assert!(add_trusted_maintainer(&layout, "maintainer", key).is_ok());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn lock_directory_component_matrix() -> prikk_error::Result<()> {
    for point in COMPONENT_POINTS {
        let root = unique_temp_dir("lock-component-matrix");
        let layout = RepositoryLayout::init(root.clone())?;
        let locks_dir = layout.refs_dir().join("locks");
        remove_empty_directory(&locks_dir)?;
        fail_once_for_test(point);
        assert!(crate::RefLock::acquire(&layout, "heads/main").is_err());
        assert_retained_component(point, &locks_dir);
        assert!(crate::RefLock::acquire(&layout, "heads/main").is_ok());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn worktree_directory_component_matrix() -> prikk_error::Result<()> {
    for point in COMPONENT_POINTS {
        let root = unique_temp_dir("worktree-component-matrix");
        let layout = RepositoryLayout::init(root.clone())?;
        let nested = root.join("nested");
        let manifest = manifest("nested/file.txt")?;
        fail_once_for_test(point);
        assert!(materialize_manifest_entries(&layout, &manifest).is_err());
        assert_retained_component(point, &nested);
        assert!(materialize_manifest_entries(&layout, &manifest).is_ok());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

fn manifest(path: &str) -> prikk_error::Result<SnapshotManifest> {
    Ok(SnapshotManifest {
        files: vec![SnapshotEntry {
            path: RepoPath::parse(path)?,
            bytes: b"content".to_vec(),
        }],
    })
}
