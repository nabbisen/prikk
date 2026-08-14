//! Per-caller directory component failure and retry matrix.

use std::path::Path;

use prikk_object::{ObjectEnvelope, ObjectType};

use crate::fsutil::{TestFailPoint, fail_once_for_test};
use crate::test_support::{
    dummy_signature, signed_empty_block_envelope, signed_patch_envelope, signed_ref_state_envelope,
    signed_ref_update_envelope, unique_temp_dir,
};
use crate::worktree::materialize_manifest_entries;
use crate::{
    FileObjectStore, ObjectWriter, RefPublication, RefStore, RepoPath, RepositoryLayout,
    SnapshotEntry, SnapshotManifest, Wal, add_trusted_maintainer, write_active_ref_metadata,
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

#[test]
fn object_directory_component_matrix() -> prikk_error::Result<()> {
    for point in COMPONENT_POINTS {
        let root = unique_temp_dir("object-component-matrix");
        let layout = RepositoryLayout::init(root.clone())?;
        let mut object = ObjectEnvelope::unsigned(ObjectType::Blob, 1, b"matrix".to_vec());
        object.add_signature(dummy_signature())?;
        let object_dir = layout
            .object_path(ObjectType::Blob, object.object_id())
            .parent()
            .ok_or_else(|| prikk_error::PrikkError::Io("object path has no parent".to_string()))?
            .to_path_buf();
        assert!(!object_dir.exists());
        let mut store = FileObjectStore::new(layout);
        fail_once_for_test(point);
        assert!(store.write_object(&object).is_err());
        assert_retained_component(point, &object_dir);
        assert_eq!(store.write_object(&object)?, object.object_id());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn wal_directory_component_matrix() -> prikk_error::Result<()> {
    for point in COMPONENT_POINTS {
        let root = unique_temp_dir("wal-component-matrix");
        let layout = RepositoryLayout::init(root.clone())?;
        let active_dir = layout.default_active_dir();
        // RFC 102 Stage 1: `init` now creates `queue.wal` itself, so the directory this test needs
        // empty (to exercise `ensure_directory_required`'s own recreation, not the WAL's presence)
        // must have it removed first.
        std::fs::remove_file(layout.default_queue_wal_path())?;
        remove_empty_directory(&active_dir)?;
        let wal = Wal::for_layout(&layout);
        let patch = signed_patch_envelope();
        fail_once_for_test(point);
        assert!(wal.append_patch(&patch).is_err());
        assert_retained_component(point, &active_dir);
        assert_eq!(wal.append_patch(&patch)?, 1);
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

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
        remove_empty_directory(&active_dir)?;
        fail_once_for_test(point);
        assert!(write_active_ref_metadata(&layout, "heads/main").is_err());
        assert_retained_component(point, &active_dir);
        assert!(write_active_ref_metadata(&layout, "heads/main").is_ok());
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

#[test]
fn ref_log_directory_component_matrix() -> prikk_error::Result<()> {
    for point in COMPONENT_POINTS {
        let root = unique_temp_dir("ref-component-matrix");
        let layout = RepositoryLayout::init(root.clone())?;
        let mut objects = FileObjectStore::new(layout.clone());
        let target = objects.write_object(&signed_empty_block_envelope())?;
        let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
        let ref_state_id = ref_state.object_id();
        assert_eq!(objects.write_object(&ref_state)?, ref_state_id);
        let publication = RefPublication {
            ref_name: "heads/main".to_string(),
            expected_previous_ref_state_id: None,
            ref_update: signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1),
            ref_state,
        };
        let logs_dir = layout.refs_dir().join("logs");
        remove_empty_directory(&logs_dir)?;
        let store = RefStore::new(layout);
        fail_once_for_test(point);
        assert!(store.publish(&publication).is_err());
        assert_retained_component(point, &logs_dir);
        assert_eq!(store.publish(&publication)?, ref_state_id);
        let _ = std::fs::remove_dir_all(root);
    }
    Ok(())
}

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
