//! Bundle export/import tests.

use prikk_object::{BlockKind, ObjectType};

use crate::bundle::{export_bundle, import_bundle};
use crate::received::read_received_pointer;
use crate::test_support::{
    signed_block, signed_patch_blob_envelope, signed_patch_envelope, signed_ref_state_envelope,
    signed_ref_update_envelope, unique_temp_dir,
};
use crate::{FileObjectStore, ObjectWriter, RefPublication, RefStore, RepositoryLayout};

/// Seal a two-block `heads/main` (a Root block plus a Normal child referencing one Patch, whose
/// `CreateFile` operation itself references one Blob) into `layout`, returning the tip Block id —
/// enough to exercise a genuinely multi-block, genesis-complete export rather than a single trivial
/// object, and to exercise blob discovery through a Patch's own operations, not just a Block's
/// `snapshot_blob_ref`.
fn seal_two_block_history(
    layout: &RepositoryLayout,
) -> prikk_error::Result<prikk_object::ObjectId> {
    let mut object_store = FileObjectStore::new(layout.clone());
    object_store.write_object(&signed_patch_blob_envelope())?;
    let patch = signed_patch_envelope();
    let patch_id = object_store.write_object(&patch)?;

    let root_block = signed_block(BlockKind::Root, Vec::new(), Vec::new(), None);
    let root_block_id = object_store.write_object(&root_block)?;

    let child_block = signed_block(BlockKind::Normal, vec![root_block_id], vec![patch_id], None);
    let child_block_id = object_store.write_object(&child_block)?;

    let ref_store = RefStore::new(layout.clone());
    let ref_state = signed_ref_state_envelope("heads/main", None, child_block_id, 1);
    let ref_state_id = ref_state.object_id();
    let ref_update =
        signed_ref_update_envelope("heads/main", None, ref_state_id, child_block_id, 1);
    ref_store.publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_state,
        ref_update,
    })?;
    Ok(child_block_id)
}

#[test]
fn export_of_missing_ref_fails() {
    let root = unique_temp_dir("bundle-export-missing-ref");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(export_bundle(&layout, "heads/main").is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn import_of_malformed_bytes_fails_closed() {
    let root = unique_temp_dir("bundle-import-malformed");
    let layout = RepositoryLayout::init(root.clone());
    assert!(layout.is_ok());
    if let Ok(layout) = layout {
        assert!(import_bundle(&layout, b"not a bundle").is_err());
        assert!(import_bundle(&layout, b"PBNDL001").is_err());
        assert!(import_bundle(&layout, &[]).is_err());
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn export_then_import_carries_the_full_genesis_complete_closure() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("bundle-export-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    let child_block_id = seal_two_block_history(&source)?;

    let (report, bytes) = export_bundle(&source, "heads/main")?;
    assert_eq!(report.ref_name, "heads/main");
    assert_eq!(report.tip_block_id, child_block_id);
    // RefState + 2 Blocks + 1 Patch + 1 Blob (the Patch's CreateFile references it) = 5 objects;
    // no Attestation in this fixture.
    assert_eq!(report.object_count, 5);

    let target_root = unique_temp_dir("bundle-import-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let import_report = import_bundle(&target, &bytes)?;
    assert_eq!(import_report.ref_name, "remotes/heads/main");
    assert_eq!(import_report.object_count, 5);
    assert_eq!(import_report.written_object_count, 5);

    let pointer = read_received_pointer(&target, "remotes/heads/main")?;
    assert!(pointer.is_some());
    if let Some(pointer) = pointer {
        assert_eq!(pointer.ref_state_id, import_report.ref_state_id);
    }

    let target_objects = FileObjectStore::new(target.clone());
    assert!(
        target_objects
            .read_typed(child_block_id, ObjectType::Block)?
            .is_some()
    );

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

#[test]
fn import_never_writes_a_local_ref_pointer() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("bundle-negctrl-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    seal_two_block_history(&source)?;
    let (_, bytes) = export_bundle(&source, "heads/main")?;

    let target_root = unique_temp_dir("bundle-negctrl-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let ref_store = RefStore::new(target.clone());
    let before = ref_store.list_ref_pointers()?;
    assert!(before.is_empty());

    import_bundle(&target, &bytes)?;

    let after = RefStore::new(target.clone()).list_ref_pointers()?;
    assert_eq!(
        before, after,
        "bundle import must never advance or create a local heads/*-or-tags/* ref pointer"
    );

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}

#[test]
fn reimporting_the_same_bundle_is_idempotent() -> prikk_error::Result<()> {
    let source_root = unique_temp_dir("bundle-reimport-source");
    let source = RepositoryLayout::init(source_root.clone())?;
    seal_two_block_history(&source)?;
    let (_, bytes) = export_bundle(&source, "heads/main")?;

    let target_root = unique_temp_dir("bundle-reimport-target");
    let target = RepositoryLayout::init(target_root.clone())?;
    let first = import_bundle(&target, &bytes)?;
    assert_eq!(first.written_object_count, 5);

    let second = import_bundle(&target, &bytes)?;
    assert_eq!(second.written_object_count, 0);
    assert_eq!(second.ref_state_id, first.ref_state_id);

    let _ = std::fs::remove_dir_all(source_root);
    let _ = std::fs::remove_dir_all(target_root);
    Ok(())
}
