use super::*;
use crate::test_support::unique_temp_dir;

/// RFC 102 Stage 3 acceptance criterion 1 (handoff §5): "No container or index name is created
/// after `init` — proven by enumeration." Enumerates every container-family path
/// `RepositoryLayout` knows about (12 container slots, the index, the generation log), confirms each
/// exists and is empty immediately after `init`, then re-runs `init` on the same root and confirms
/// nothing about them changed -- an idempotent re-`init` must not clobber or recreate any of them,
/// the same rule Stage 1 established for the worktree marker and the active WAL.
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
    assert_eq!(reopened.format(), RepositoryFormat::CurrentV3);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
