use std::collections::HashSet;
use std::path::PathBuf;

use super::*;
use crate::test_support::{
    signed_empty_block_envelope, signed_ref_state_envelope, signed_ref_update_envelope,
    unique_temp_dir,
};
use crate::{FileObjectStore, ObjectWriter, RefPublication, RefStore};
use prikk_object::{ObjectEnvelope, ObjectType};

/// Recursively lists every regular file under `dir` (there is no directory nesting deeper than
/// `containers/<type>/`, so a shallow, non-generic walk is enough here).
fn files_under(dir: &std::path::Path) -> Result<HashSet<PathBuf>> {
    let mut files = HashSet::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            files.extend(files_under(&path)?);
        } else {
            files.insert(path);
        }
    }
    Ok(files)
}

/// RFC 102 Stage 3 acceptance criterion 1 (handoff §5): "No container or index name is created
/// after `init` — proven by enumeration." Enumerates every container-family path
/// `RepositoryLayout` knows about (12 container slots, the index, the generation log), confirms each
/// exists and is empty immediately after `init`, then re-runs `init` on the same root and confirms
/// nothing about them changed -- an idempotent re-`init` must not clobber or recreate any of them,
/// the same rule Stage 1 established for the worktree marker and the active WAL. Finally writes real
/// objects of every persisted type through `FileObjectStore` (ordinary use, not just re-`init`) and
/// re-enumerates the whole `containers/` tree from the filesystem itself -- not just the 14 paths
/// `RepositoryLayout` names -- confirming the file *set* is still exactly those 14: ordinary writes
/// grow existing files, they never create a fifteenth.
#[test]
fn init_allocates_every_container_index_and_generation_log_name_once() -> Result<()> {
    let root = unique_temp_dir("layout-container-allocation");
    let layout = RepositoryLayout::init(root.clone())?;

    let mut container_paths = Vec::new();
    for object_type in persisted_object_types() {
        container_paths.push(layout.container_slot_path(object_type, ContainerSlot::A));
        container_paths.push(layout.container_slot_path(object_type, ContainerSlot::B));
    }
    container_paths.push(layout.container_index_path());
    container_paths.push(layout.container_generation_log_path());

    assert_eq!(
        container_paths.len(),
        14,
        "6 object types x 2 slots + index + generation log"
    );
    for path in &container_paths {
        assert!(path.is_file(), "expected {path:?} to exist after init");
        assert_eq!(
            std::fs::metadata(path)?.len(),
            0,
            "expected {path:?} to be created empty"
        );
    }

    // A second `init` on the same root must be a no-op for every one of these files: same content
    // (still empty), same mtime-independent identity -- re-reading each path must not error, and
    // nothing here re-creates or truncates an already-present file.
    let reopened = RepositoryLayout::init(root.clone())?;
    for path in &container_paths {
        assert!(path.is_file());
        assert_eq!(std::fs::metadata(path)?.len(), 0);
    }
    assert_eq!(reopened.format(), RepositoryFormat::CurrentV4);

    let expected: HashSet<PathBuf> = container_paths.into_iter().collect();
    assert_eq!(files_under(&layout.containers_dir())?, expected);

    let mut store = FileObjectStore::new(layout.clone());
    for object_type in persisted_object_types() {
        let schema_version = if object_type == ObjectType::Block {
            2
        } else {
            1
        };
        let mut envelope =
            ObjectEnvelope::unsigned(object_type, schema_version, b"acceptance-1".to_vec());
        envelope.add_signature(crate::test_support::dummy_signature())?;
        store.write_object(&envelope)?;
    }
    assert_eq!(
        files_under(&layout.containers_dir())?,
        expected,
        "ordinary object writes must grow existing container/index files, never create a new one"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// RFC 102 Stage 4 acceptance criterion 1 (handoff §4): "every new container name created at
/// `init`" -- the shared ref-log container's both slots, plus the ref-pointer-index container.
/// Mirrors `init_allocates_every_container_index_and_generation_log_name_once` exactly, including
/// the third phase that test's own doc calls out by name: real publishes through `RefStore`
/// (ordinary use, not just re-`init`), then the whole `refs/containers/` tree re-enumerated from the
/// filesystem itself to confirm the file *set* is still exactly those 3 -- not just that
/// `RepositoryLayout`'s own three named paths still exist, which a call site bug could satisfy while
/// a real fourth file sat unnoticed alongside them.
#[test]
fn init_allocates_every_ref_container_name_once() -> Result<()> {
    let root = unique_temp_dir("layout-ref-container-allocation");
    let layout = RepositoryLayout::init(root.clone())?;

    let ref_container_paths = vec![
        layout.ref_log_container_slot_path(ContainerSlot::A),
        layout.ref_log_container_slot_path(ContainerSlot::B),
        layout.ref_pointer_index_path(),
    ];
    for path in &ref_container_paths {
        assert!(path.is_file(), "expected {path:?} to exist after init");
        assert_eq!(
            std::fs::metadata(path)?.len(),
            0,
            "expected {path:?} to be created empty"
        );
    }

    let reopened = RepositoryLayout::init(root.clone())?;
    for path in &ref_container_paths {
        assert!(path.is_file());
        assert_eq!(std::fs::metadata(path)?.len(), 0);
    }
    assert_eq!(reopened.format(), RepositoryFormat::CurrentV4);

    let expected: HashSet<PathBuf> = ref_container_paths.into_iter().collect();
    assert_eq!(files_under(&layout.refs_containers_dir())?, expected);

    let mut objects = FileObjectStore::new(layout.clone());
    let target = objects.write_object(&signed_empty_block_envelope())?;
    let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
    let ref_state_id = ref_state.object_id();
    let first = RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_update: signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1),
        ref_state,
    };
    let store = RefStore::new(layout.clone());
    store.publish(&first)?;
    // A second ref name, minted well after `init` -- exactly the "branch create mints one later"
    // scenario acceptance criterion 1 is about, not just a second publish to the same name.
    let second_target = objects.write_object(&signed_empty_block_envelope())?;
    let second_ref_state = signed_ref_state_envelope("heads/topic", None, second_target, 1);
    let second_ref_state_id = second_ref_state.object_id();
    store.publish(&RefPublication {
        ref_name: "heads/topic".to_string(),
        expected_previous_ref_state_id: None,
        ref_update: signed_ref_update_envelope(
            "heads/topic",
            None,
            second_ref_state_id,
            second_target,
            1,
        ),
        ref_state: second_ref_state,
    })?;
    assert_eq!(
        files_under(&layout.refs_containers_dir())?,
        expected,
        "ordinary publishes -- including minting a brand-new ref name well after init -- must grow \
         existing container/index files, never create a new one"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
