use std::collections::HashSet;
use std::path::PathBuf;

use super::*;
use crate::test_support::{
    signed_empty_block_envelope, signed_ref_state_envelope, signed_ref_update_envelope,
    unique_temp_dir,
};
use crate::{FileObjectStore, ObjectWriter, RefPublication, RefStore, verify_repository};
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
    assert_eq!(reopened.format(), RepositoryFormat::CurrentV6);

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
/// `init`" -- the shared ref-log container's both slots, plus the ref-pointer-index container. RFC
/// 102 Stage 5 adds the received-index container to this same directory (design-v1.md §14, Step 0
/// item 2), so this test covers it too rather than splitting into a separate one -- it belongs to the
/// exact same "every name under `refs_containers_dir()`" property. RFC 102 Stage 6 Step 1
/// (design-v1.md §15.6) gives the ref-pointer-index and received-index containers their own `A`/`B`
/// slots and generation logs -- the ref-log container's own `B` slot stays unused forever (§15.2), but
/// these two are two of the three genuine compaction targets, so this enumeration grows by two names
/// each. Mirrors `init_allocates_every_container_index_and_generation_log_name_once` exactly,
/// including the third phase that test's own doc calls out by name: real publishes through
/// `RefStore` (ordinary use, not just re-`init`), then the whole `refs/containers/` tree
/// re-enumerated from the filesystem itself to confirm the file *set* is still exactly those 8 -- not
/// just that `RepositoryLayout`'s own named paths still exist, which a call site bug could satisfy
/// while a real extra file sat unnoticed alongside them.
#[test]
fn init_allocates_every_ref_container_name_once() -> Result<()> {
    let root = unique_temp_dir("layout-ref-container-allocation");
    let layout = RepositoryLayout::init(root.clone())?;

    let ref_container_paths = vec![
        layout.ref_log_container_slot_path(ContainerSlot::A),
        layout.ref_log_container_slot_path(ContainerSlot::B),
        layout.ref_pointer_index_slot_path(ContainerSlot::A),
        layout.ref_pointer_index_slot_path(ContainerSlot::B),
        layout.ref_pointer_index_generation_log_path(),
        layout.received_index_slot_path(ContainerSlot::A),
        layout.received_index_slot_path(ContainerSlot::B),
        layout.received_index_generation_log_path(),
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
    assert_eq!(reopened.format(), RepositoryFormat::CurrentV6);

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

/// RFC 102 Stage 5 acceptance criterion 1, folded in per round 4's review §3: the trust key and
/// policy containers, mirroring `init_allocates_every_ref_container_name_once`'s exact shape --
/// existence and emptiness immediately after `init`, idempotent re-`init`, then real writes
/// (`add_trusted_maintainer`, not just re-`init`) followed by re-enumerating `trust_dir()` from the
/// filesystem itself to confirm the file set is still exactly those. RFC 102 Stage 6 Step 1
/// (design-v1.md §15.6) gives the trust policy container its own `A`/`B` slots and a generation log --
/// the third of the three genuine compaction targets -- while `trust_key_container_path` stays a
/// single unslotted name, unchanged: TOFU history must persist across removal (`trust.rs:77`), which
/// compacting the key container would break.
#[test]
fn init_allocates_every_trust_container_name_once() -> Result<()> {
    let root = unique_temp_dir("layout-trust-container-allocation");
    let layout = RepositoryLayout::init(root.clone())?;

    let trust_container_paths = vec![
        layout.trust_key_container_path(),
        layout.trust_policy_container_slot_path(ContainerSlot::A),
        layout.trust_policy_container_slot_path(ContainerSlot::B),
        layout.trust_policy_generation_log_path(),
    ];
    for path in &trust_container_paths {
        assert!(path.is_file(), "expected {path:?} to exist after init");
        assert_eq!(
            std::fs::metadata(path)?.len(),
            0,
            "expected {path:?} to be created empty"
        );
    }

    let reopened = RepositoryLayout::init(root.clone())?;
    for path in &trust_container_paths {
        assert!(path.is_file());
        assert_eq!(std::fs::metadata(path)?.len(), 0);
    }
    assert_eq!(reopened.format(), RepositoryFormat::CurrentV6);

    let expected: HashSet<PathBuf> = trust_container_paths.into_iter().collect();
    assert_eq!(files_under(&layout.trust_dir())?, expected);

    let key = "0707070707070707070707070707070707070707070707070707070707070707";
    crate::add_trusted_maintainer(&layout, "maintainer", key)?;
    // A second key id, adopted well after init -- the same "mints one later" scenario the ref
    // container test proves, applied here.
    crate::add_trusted_maintainer(&layout, "second", key)?;
    assert_eq!(
        files_under(&layout.trust_dir())?,
        expected,
        "ordinary adoptions -- including a second key id well after init -- must grow existing \
         container files, never create a new one"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// RFC 102 Stage 5 acceptance criterion 1, folded in per round 4's review §3: `active/default/`'s
/// `init`-allocated names (`queue.wal`, `ref-name`) enumerated the same way, with the one real
/// complication the review named directly -- `active.lock` is created at runtime by `ActiveLock::
/// acquire`, not `init`, and correctly so (a lock file lost to a crash is harmless; its holder is
/// gone too). Not a criterion-1 violation, but it does mean this directory's membership isn't a
/// plain fixed set the way `refs_containers_dir()`'s or `trust_dir()`'s is -- asserted here as "the
/// non-lock members are exactly these two names," both before and after the lock exists.
#[test]
fn init_allocates_every_active_default_container_name_once_excluding_the_runtime_lock() -> Result<()>
{
    let root = unique_temp_dir("layout-active-default-allocation");
    let layout = RepositoryLayout::init(root.clone())?;

    let active_container_paths = vec![
        layout.default_queue_wal_path(),
        layout.default_active_ref_name_path(),
    ];
    for path in &active_container_paths {
        assert!(path.is_file(), "expected {path:?} to exist after init");
        assert_eq!(
            std::fs::metadata(path)?.len(),
            0,
            "expected {path:?} to be created empty"
        );
    }

    let expected: HashSet<PathBuf> = active_container_paths.into_iter().collect();
    assert_eq!(
        files_under(&layout.default_active_dir())?,
        expected,
        "before any lock is ever acquired, active/default/ must contain exactly the two init-\
         allocated names"
    );

    let lock_path = layout.default_active_lock_path();
    let lock = crate::ActiveLock::acquire(&layout)?;
    assert!(lock_path.is_file(), "expected the lock file to now exist");
    let mut with_lock = files_under(&layout.default_active_dir())?;
    assert!(
        with_lock.remove(&lock_path),
        "the lock must be the only addition"
    );
    assert_eq!(
        with_lock, expected,
        "excluding the runtime lock, the set is still exactly the two init-allocated names"
    );
    drop(lock);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// Dead-surface consolidation, acceptance criterion 3: `required_directories()` dropping ten
/// directories (`objects/` + its six type subdirectories, `refs/by-id/`, `refs/logs/`,
/// `quarantine/`) is asserted, not argued, to be genuinely "not a format change" -- a fresh
/// repository must still `init`, accept ordinary use (a real object write, a real ref publish), and
/// `verify` clean, with none of the ten directories ever having existed. `refs/tmp/` is the control:
/// it stays in `required_directories()` (`refs/verify.rs`'s `candidate_issues` scans it on every
/// `verify`), so its continued presence -- proven the same way the other ten's absence is -- is what
/// distinguishes "removed because nothing reads it" from "removed and something broke."
#[test]
fn a_fresh_repository_opens_and_verifies_without_the_ten_removed_directories() -> Result<()> {
    let root = unique_temp_dir("layout-no-dead-directories");
    let layout = RepositoryLayout::init(root.clone())?;

    let mut absent_paths = vec![
        layout.prikk_dir().join("objects"),
        layout.refs_dir().join("by-id"),
        layout.refs_dir().join("logs"),
        layout.prikk_dir().join("quarantine"),
    ];
    for object_type in persisted_object_types() {
        absent_paths.push(
            layout
                .prikk_dir()
                .join("objects")
                .join(object_type_directory_name(object_type)),
        );
    }
    assert_eq!(
        absent_paths.len(),
        10,
        "objects/ + its six subdirectories + three others"
    );
    for absent in &absent_paths {
        assert!(
            !absent.exists(),
            "expected {absent:?} to not exist after init -- it is one of the ten removed \
             directories"
        );
    }
    assert!(
        layout.refs_dir().join("tmp").is_dir(),
        "refs/tmp/ is the one directory that stays -- refs/verify.rs's candidate scan reads it \
         on every verify"
    );

    let mut objects = FileObjectStore::new(layout.clone());
    let target = objects.write_object(&signed_empty_block_envelope())?;
    let ref_state = signed_ref_state_envelope("heads/main", None, target, 1);
    let ref_state_id = ref_state.object_id();
    RefStore::new(layout.clone()).publish(&RefPublication {
        ref_name: "heads/main".to_string(),
        expected_previous_ref_state_id: None,
        ref_update: signed_ref_update_envelope("heads/main", None, ref_state_id, target, 1),
        ref_state,
    })?;

    let report = verify_repository(&layout)?;
    assert!(
        !report.has_stage_failure(),
        "expected a clean verify on a repository that never had the ten removed directories, got: \
         {report:?}"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
