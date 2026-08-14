#![allow(clippy::expect_used, clippy::indexing_slicing)]

use prikk_error::Result;
use prikk_object::{
    CanonicalEncode, CreateFile, NodeId, ObjectEnvelope, ObjectType, Operation, OperationKind,
    PatchPayload, PatchPurpose,
};

use super::{lookup_object_location, rebuild_index_from_containers, write_object_to_container};
use crate::container;
use crate::fsutil::append_file_required;
use crate::layout::{ContainerSlot, RepositoryLayout};
use crate::test_support::{sample_object_id, signed_patch_blob_envelope, signed_patch_envelope};

fn normal_patch_envelope(label: &str) -> Result<ObjectEnvelope> {
    let payload = PatchPayload {
        operations: vec![Operation {
            op_seq: 1,
            op_id: None,
            preconditions: Vec::new(),
            kind: OperationKind::CreateFile(CreateFile {
                path: "a.txt".to_string(),
                node_id: NodeId::from_bytes([0x61; 32]),
                blob_id: sample_object_id(label),
                mode: 0o100_644,
            }),
        }],
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: Vec::new(),
        purpose: PatchPurpose::Normal,
    };
    let mut envelope =
        ObjectEnvelope::unsigned(ObjectType::Patch, 1, payload.to_canonical_bytes()?);
    envelope.add_signature(crate::test_support::rollback_author_signature())?;
    Ok(envelope)
}

#[test]
fn write_then_lookup_round_trips() -> Result<()> {
    let root = crate::test_support::unique_temp_dir("index-write-lookup");
    let layout = RepositoryLayout::init(root.clone())?;
    let envelope = signed_patch_envelope();
    let id = write_object_to_container(&layout, ObjectType::Patch, &envelope)?;
    assert_eq!(id, envelope.object_id());

    let entry = lookup_object_location(&layout, id)?.expect("entry must be found after write");
    assert_eq!(entry.object_id, id);
    assert_eq!(entry.object_type, ObjectType::Patch);
    assert_eq!(entry.slot, ContainerSlot::A);
    assert_eq!(entry.offset, 0);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn rewriting_the_same_object_is_idempotent_not_a_second_entry() -> Result<()> {
    let root = crate::test_support::unique_temp_dir("index-idempotent-rewrite");
    let layout = RepositoryLayout::init(root.clone())?;
    let envelope = signed_patch_envelope();
    write_object_to_container(&layout, ObjectType::Patch, &envelope)?;
    write_object_to_container(&layout, ObjectType::Patch, &envelope)?;

    let replay = super::replay_index(&layout)?;
    assert_eq!(
        replay.entries.len(),
        1,
        "re-writing the same object must not grow the index"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn rebuild_recovers_every_sound_object_across_all_container_types() -> Result<()> {
    let root = crate::test_support::unique_temp_dir("index-rebuild-multi-type");
    let layout = RepositoryLayout::init(root.clone())?;
    let patch = normal_patch_envelope("rebuild-patch")?;
    let blob = signed_patch_blob_envelope();
    write_object_to_container(&layout, ObjectType::Patch, &patch)?;
    write_object_to_container(&layout, ObjectType::Blob, &blob)?;

    let rebuilt = rebuild_index_from_containers(&layout)?;
    let ids: Vec<_> = rebuilt.iter().map(|entry| entry.object_id).collect();
    assert!(ids.contains(&patch.object_id()));
    assert!(ids.contains(&blob.object_id()));
    assert_eq!(rebuilt.len(), 2);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// The acceptance criterion (handoff §5 criterion 3): a crash between the container append and the
/// index append leaves the object present and unindexed, recoverable by rebuild -- never the reverse
/// (an index entry pointing at bytes that are not there). Simulated by calling exactly step 1 of
/// `write_object_to_container`'s own protocol (append the container record) without step 2.
#[test]
fn crash_between_container_and_index_append_leaves_the_object_unindexed_and_recoverable()
-> Result<()> {
    let root = crate::test_support::unique_temp_dir("index-crash-ordering");
    let layout = RepositoryLayout::init(root.clone())?;
    let envelope = signed_patch_envelope();

    // Step 1 only, matching `write_object_to_container`'s own first step exactly -- no index append
    // follows, simulating a crash right after the container append became durable.
    let record_bytes = container::encode_container_record(ObjectType::Patch, &envelope)?;
    let container_relative = layout
        .repository_relative(&layout.container_slot_path(ObjectType::Patch, ContainerSlot::A))?;
    append_file_required(
        layout.repository_mutation_root(),
        &container_relative,
        &record_bytes,
    )?;

    // The object is present (a container scan finds it)...
    let container_bytes =
        std::fs::read(layout.container_slot_path(ObjectType::Patch, ContainerSlot::A))?;
    let container_replay =
        container::decode_container_records(ObjectType::Patch, &container_bytes)?;
    assert_eq!(container_replay.records.len(), 1);
    assert_eq!(container_replay.records[0].envelope, envelope);

    // ...but unindexed: an ordinary lookup finds nothing, never a dangling entry pointing at these
    // bytes (the property that must never be violated, design §5).
    assert!(lookup_object_location(&layout, envelope.object_id())?.is_none());

    // Recovered by rebuild, the safe direction.
    let rebuilt = rebuild_index_from_containers(&layout)?;
    assert_eq!(rebuilt.len(), 1);
    assert_eq!(rebuilt[0].object_id, envelope.object_id());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

/// Preserves `publish_immutable_file`'s exact idempotency contract (the loose-file mechanism this
/// replaces): a same-`object_id` rewrite is a silent no-op only when its full envelope bytes match
/// what is already stored. `object_id` does not cover signatures, so two envelopes can share an id
/// while differing in signature content -- that must still be a reported conflict, not silently
/// accepted, exactly as the old `compare_existing`'s `bytes != candidate` check enforced.
#[test]
fn rewriting_the_same_object_id_with_different_signatures_is_an_error() -> Result<()> {
    let root = crate::test_support::unique_temp_dir("index-conflicting-rewrite");
    let layout = RepositoryLayout::init(root.clone())?;
    let first = signed_patch_envelope();
    write_object_to_container(&layout, ObjectType::Patch, &first)?;

    // Same canonical_payload/type/schema -- hence the same object_id, which does not cover
    // signatures -- but a different signature.
    let mut second = first.clone();
    second.signatures.clear();
    let mut signature = crate::test_support::rollback_author_signature();
    signature.signature_bytes[0] ^= 0x01;
    second.add_signature(signature)?;
    assert_eq!(
        second.object_id(),
        first.object_id(),
        "the two envelopes must share an object_id for this test to prove anything"
    );

    assert!(write_object_to_container(&layout, ObjectType::Patch, &second).is_err());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn a_damaged_index_entry_blocks_lookup_as_a_reported_defect() -> Result<()> {
    let root = crate::test_support::unique_temp_dir("index-damaged-entry");
    let layout = RepositoryLayout::init(root.clone())?;
    let envelope = signed_patch_envelope();
    write_object_to_container(&layout, ObjectType::Patch, &envelope)?;

    let index_path = layout.container_index_path();
    let mut bytes = std::fs::read(&index_path)?;
    let last = bytes.last_mut().expect("index file must be non-empty");
    *last ^= 0x01;
    std::fs::write(&index_path, &bytes)?;

    assert!(lookup_object_location(&layout, envelope.object_id()).is_err());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
