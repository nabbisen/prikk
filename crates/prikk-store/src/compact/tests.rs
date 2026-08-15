#![allow(clippy::indexing_slicing)]

use prikk_crypto::Ed25519KeyPair;
use prikk_error::Result;

use super::{compact_received_index, compact_ref_pointer_index, compact_trust_policy};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use crate::fsutil::{TestFailPoint, fail_after_for_test};
use crate::generation::resolve_live_slot;
use crate::layout::{ContainerSlot, LockableContainer};
use crate::lock::acquire_container_locks;
use crate::test_support::{
    signed_empty_block_envelope, signed_ref_state_envelope, unique_temp_dir,
};
use crate::{
    FileObjectStore, ObjectWriter, RefPublication, RefStore, RepositoryLayout,
    add_trusted_maintainer, load_maintainer_trust_policy, remove_trusted_maintainer,
};

fn public_key_hex(seed: &[u8; 32]) -> String {
    Ed25519KeyPair::from_seed(seed)
        .public_key_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn publish_update(
    store: &RefStore,
    objects: &mut FileObjectStore,
    ref_name: &str,
    expected_previous: Option<prikk_object::ObjectId>,
    seq: u64,
) -> Result<prikk_object::ObjectId> {
    let target = objects.write_object(&signed_empty_block_envelope())?;
    let ref_state = signed_ref_state_envelope(ref_name, expected_previous, target, seq);
    let ref_state_id = ref_state.object_id();
    store.publish(&RefPublication {
        ref_name: ref_name.to_string(),
        expected_previous_ref_state_id: expected_previous,
        ref_update: crate::test_support::signed_ref_update_envelope(
            ref_name,
            expected_previous,
            ref_state_id,
            target,
            seq,
        ),
        ref_state,
    })
}

/// Acceptance criterion 1 (compactor publishes by appending a generation record, after the new
/// slot's bytes are durable) proven end to end: three updates to the same ref plus one update to an
/// unrelated ref leaves 4 raw entries but only 2 live pointers; compaction must reclaim exactly the
/// 2 stale ones while every lookup still resolves correctly afterward.
#[test]
fn compacting_the_ref_pointer_index_reclaims_stale_entries_and_preserves_current_pointers()
-> Result<()> {
    let root = unique_temp_dir("compact-pointer-index");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let store = RefStore::new(layout.clone());

    let first = publish_update(&store, &mut objects, "heads/main", None, 1)?;
    let second = publish_update(&store, &mut objects, "heads/main", Some(first), 2)?;
    let third = publish_update(&store, &mut objects, "heads/main", Some(second), 3)?;
    let other = publish_update(&store, &mut objects, "heads/topic", None, 1)?;

    let generation_log_path = layout.ref_pointer_index_generation_log_path();
    let live_before = resolve_live_slot(&layout, &generation_log_path)?;
    assert_eq!(live_before, ContainerSlot::A);

    let report = compact_ref_pointer_index(&layout)?;
    assert_eq!(report.entries_before, 4);
    assert_eq!(report.entries_after, 2);

    let live_after = resolve_live_slot(&layout, &generation_log_path)?;
    assert_eq!(live_after, ContainerSlot::B);

    assert_eq!(store.read_current_ref_state_id("heads/main")?, Some(third));
    assert_eq!(store.read_current_ref_state_id("heads/topic")?, Some(other));

    // The retired slot's raw bytes are untouched by this compaction -- nothing is destroyed, only
    // superseded. It becomes the *next* compaction's own target, truncated only then.
    let retired_bytes = std::fs::read(layout.ref_pointer_index_slot_path(ContainerSlot::A))?;
    assert!(!retired_bytes.is_empty());

    // The system stays fully functional post-compaction: a further update still publishes and
    // resolves correctly through the now-live slot.
    let fourth = publish_update(&store, &mut objects, "heads/main", Some(third), 4)?;
    assert_eq!(store.read_current_ref_state_id("heads/main")?, Some(fourth));

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// Acceptance criterion 2, shown rather than argued: a crash between the new slot's bytes landing
/// and the generation record being appended must leave the *old* generation authoritative -- the
/// retry must be safe, and nothing observes the half-published state in between. Failpoint-gated to
/// Linux/macOS, matching `TestFailPoint`'s own availability (`fsutil.rs`).
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn a_crash_before_the_generation_record_lands_leaves_the_old_generation_authoritative() -> Result<()>
{
    let root = unique_temp_dir("compact-pointer-index-crash-before-publish");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let store = RefStore::new(layout.clone());
    let first = publish_update(&store, &mut objects, "heads/main", None, 1)?;
    let second = publish_update(&store, &mut objects, "heads/main", Some(first), 2)?;

    let generation_log_path = layout.ref_pointer_index_generation_log_path();

    // Two `AppendWrite`s happen inside a successful run: the compacted slot's own bytes, then the
    // generation record. Skip 1 to fail on the second -- the generation record's own append -- so
    // the new slot's bytes are already durable when the crash hits.
    fail_after_for_test(TestFailPoint::AppendWrite, 1);
    assert!(compact_ref_pointer_index(&layout).is_err());

    // The old generation is still authoritative: nothing has appended a generation record, so the
    // resolver still (and only ever) has `A` to return.
    assert_eq!(
        resolve_live_slot(&layout, &generation_log_path)?,
        ContainerSlot::A
    );
    assert_eq!(store.read_current_ref_state_id("heads/main")?, Some(second));

    // Retry succeeds and completes the switch.
    let report = compact_ref_pointer_index(&layout)?;
    assert_eq!(report.entries_after, 1);
    assert_eq!(
        resolve_live_slot(&layout, &generation_log_path)?,
        ContainerSlot::B
    );
    assert_eq!(store.read_current_ref_state_id("heads/main")?, Some(second));

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// The §15.3 ruling, non-negotiable: compaction refuses to run on a container with a known-corrupt
/// record, rather than silently compacting around the damage. Damage a record's checksum-covered
/// body, observe the refusal, restore the bytes, observe compaction then succeeds.
#[test]
fn compaction_refuses_on_a_corrupt_container_and_touches_nothing() -> Result<()> {
    let root = unique_temp_dir("compact-pointer-index-corrupt");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let store = RefStore::new(layout.clone());
    publish_update(&store, &mut objects, "heads/main", None, 1)?;

    let live_path = layout.ref_pointer_index_slot_path(ContainerSlot::A);
    let sound_bytes = std::fs::read(&live_path)?;
    let mut damaged = sound_bytes.clone();
    let last = damaged.len() - 1;
    damaged[last] ^= 0x01;
    std::fs::write(&live_path, &damaged)?;

    assert!(compact_ref_pointer_index(&layout).is_err());
    // Nothing touched: the live slot is exactly as this test left it (still damaged, not repaired
    // or partially rewritten), the retired slot is still empty, and no generation record exists.
    assert_eq!(std::fs::read(&live_path)?, damaged);
    assert!(std::fs::read(layout.ref_pointer_index_slot_path(ContainerSlot::B))?.is_empty());
    assert_eq!(
        resolve_live_slot(&layout, &layout.ref_pointer_index_generation_log_path())?,
        ContainerSlot::A
    );

    std::fs::write(&live_path, &sound_bytes)?;
    let report = compact_ref_pointer_index(&layout)?;
    assert_eq!(report.entries_after, 1);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// Acceptance criterion 4: the compactor participates in the same container lock the writers do --
/// proven the same way round 2 proved the writers do, from the other direction: hold the lock
/// externally, observe the compactor refuse.
#[test]
fn compaction_refuses_while_its_own_container_lock_is_externally_held() -> Result<()> {
    let root = unique_temp_dir("compact-pointer-index-lock-conflict");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let store = RefStore::new(layout.clone());
    publish_update(&store, &mut objects, "heads/main", None, 1)?;

    let held = acquire_container_locks(&layout, &[LockableContainer::RefPointerIndex])?;
    assert!(compact_ref_pointer_index(&layout).is_err());
    drop(held);
    assert!(compact_ref_pointer_index(&layout).is_ok());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// The received-index compactor, mirroring the ref-pointer-index coverage above at lighter weight:
/// two imports to the same received-ref name leave one stale entry, reclaimed by compaction.
#[test]
fn compacting_the_received_index_reclaims_a_superseded_import() -> Result<()> {
    let root = unique_temp_dir("compact-received-index");
    let layout = RepositoryLayout::init(root.clone())?;
    let mut objects = FileObjectStore::new(layout.clone());
    let first = objects
        .write_object(&signed_empty_block_envelope())?
        .to_owned();
    let second_state = signed_ref_state_envelope("heads/main", None, first, 1);
    let second = second_state.object_id();
    let third_state = signed_ref_state_envelope("heads/main", None, first, 2);
    let third = third_state.object_id();

    crate::received::write_received_pointer(&layout, "remotes/heads/main", second)?;
    crate::received::write_received_pointer(&layout, "remotes/heads/main", third)?;

    let report = compact_received_index(&layout)?;
    assert_eq!(report.entries_before, 2);
    assert_eq!(report.entries_after, 1);
    let pointer = crate::received::read_received_pointer(&layout, "remotes/heads/main")?;
    assert!(pointer.is_some());
    if let Some(pointer) = pointer {
        assert_eq!(pointer.ref_state_id, third);
    }

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// The trust-policy compactor: unlike the other two, reduction keeps only the *last* snapshot, not
/// one entry per key -- three snapshots (add, add, remove) collapse to the one live snapshot.
#[test]
fn compacting_the_trust_policy_container_keeps_only_the_last_snapshot() -> Result<()> {
    let root = unique_temp_dir("compact-trust-policy");
    let layout = RepositoryLayout::init(root.clone())?;
    let first_key = public_key_hex(&[7_u8; 32]);
    let second_key = public_key_hex(&[8_u8; 32]);
    add_trusted_maintainer(&layout, "first", &first_key)?;
    add_trusted_maintainer(&layout, "second", &second_key)?;
    remove_trusted_maintainer(&layout, "first")?;

    let report = compact_trust_policy(&layout)?;
    assert_eq!(report.entries_before, 3);
    assert_eq!(report.entries_after, 1);

    let policy = load_maintainer_trust_policy(&layout)?;
    assert_eq!(policy.keys.len(), 1);
    assert_eq!(policy.keys[0].key_id, "second");

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
