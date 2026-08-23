//! RFC 115 Stage 3 (design-v1.md D1/D2/D5, §7/§8) — the patch-exchange artifact and accept path.
//!
//! **Representational, not frozen** (RFC 114 §3): this format carries objects whose identity is
//! already frozen, and carries no identity of its own. It may change in a later version with a
//! documented read path — the same asymmetry every repository-format and bundle-format transition
//! in this project already has (read what the past wrote, write only the present). That licence is
//! not permission to be careless: every byte still has a stated reason.
//!
//! **`PEXCH002` is emitted on export and accepted on import** (RFC 117 stage 3 bumped `PEXCH001` ->
//! `PEXCH002` to add a Tag section; `PEXCH001` is refused, not read -- see `artifact.rs`'s own doc).
//!
//! **Scope, stated plainly (handoff §1): this stage does not deliver a seal-from-accepted path.**
//! `seal` builds a block from the active WAL; an accepted patch is a written object that was never
//! in the WAL, and closing that gap is real design work this stage does not otherwise touch. What
//! lands here: receive patches, verify them, store them, and see exactly what you hold that is not
//! yet sealed. You cannot yet seal them.

use std::collections::BTreeSet;

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectId, ObjectType};

use crate::container::decode_container_records;
use crate::fsutil::read_file_if_exists;
use crate::layout::{ContainerSlot, RepositoryLayout, persisted_object_types};
use crate::object_store::{ObjectReadSnapshot, ObjectReader};
use crate::patch_set_digest::patch_ids_reachable_from_block;
use crate::refs::RefStore;

mod accept;
mod artifact;

pub use accept::{
    AcceptOptions, AcceptReport, ClaimSignatureVerification, accept_exchange_artifact,
};
pub use artifact::{
    DEFAULT_EXCHANGE_ARTIFACT_MAX_OBJECT_COUNT, DEFAULT_EXCHANGE_ARTIFACT_MAX_TOTAL_BYTES,
    ExchangeExportReport, export_exchange_artifact,
};

/// Resolve one ref's tip to the Block it ultimately names -- one hop for a `Branch`, two for a
/// `Tag` (`TagPayload.target_block_id`). The same small resolution `bundle.rs`'s
/// `resolve_ref_target_block` and `patch_set_digest.rs`'s `resolve_ref_to_tip_block` already each
/// have their own copy of, for their own reasons (bundle export also collects the Tag envelope
/// itself; the digest refuses `remotes/*` outright). Neither shape fits this query, which needs
/// only the resolved Block id, for every local ref, and this is now the third place this exact
/// two-hop pattern is needed -- worth a name in a report, not a silent fourth copy, but also not a
/// cross-module refactor of two already-reviewed call sites this handoff does not otherwise touch.
fn resolve_ref_tip_block(
    object_store: &impl ObjectReader,
    ref_state_payload: &prikk_object::RefStatePayload,
) -> Result<ObjectId> {
    match ref_state_payload.kind {
        prikk_object::RefKind::Branch => Ok(ref_state_payload.target_object_id),
        prikk_object::RefKind::Tag => {
            let tag_id = ref_state_payload.target_object_id;
            let tag_envelope = object_store
                .read_typed(tag_id, ObjectType::Tag)?
                .ok_or_else(|| PrikkError::Integrity(format!("missing Tag object: {tag_id}")))?;
            let tag_payload =
                prikk_object::TagPayload::decode_canonical(&tag_envelope.canonical_payload)?;
            Ok(tag_payload.target_block_id)
        }
    }
}

/// D2's derived query (design-v1.md §3, RFC 115 Stage 3 §5): **no new container, no stored pending
/// state.** "Accepted but unsealed" is computed, every time it's asked:
///
/// > patch objects present in this repository, minus the patch ids reachable from any block.
///
/// Enumerates stored patches the way `verify/objects.rs:163` enumerates objects
/// (`persisted_object_types()` → per-type container → `decode_container_records`, narrowed here to
/// `ObjectType::Patch` alone) and reuses Stage 1's `patch_ids_reachable_from_block` for the
/// subtrahend, over every local ref's tip -- the same ancestry walk `export_bundle` and Stage 1's
/// digest already share, not a second one.
pub fn accepted_but_unsealed_patch_ids(layout: &RepositoryLayout) -> Result<Vec<ObjectId>> {
    debug_assert!(
        persisted_object_types().contains(&ObjectType::Patch),
        "Patch must remain a persisted, containerized object type"
    );
    let container_path = layout.container_slot_path(ObjectType::Patch, ContainerSlot::A);
    let relative = layout.repository_relative(&container_path)?;
    let mut all_patch_ids: BTreeSet<ObjectId> = BTreeSet::new();
    if let Some(bytes) = read_file_if_exists(layout.repository_mutation_root(), &relative)? {
        let replay = decode_container_records(ObjectType::Patch, &bytes)?;
        for record in replay.records {
            all_patch_ids.insert(record.envelope.object_id());
        }
    }

    let object_store = ObjectReadSnapshot::open(layout)?;
    let ref_store = RefStore::new(layout.clone());
    let mut reachable: BTreeSet<ObjectId> = BTreeSet::new();
    for pointer in ref_store.list_ref_pointers()? {
        let ref_state_envelope = object_store
            .read_typed(pointer.ref_state_id, ObjectType::RefState)?
            .ok_or_else(|| {
                PrikkError::Integrity(format!(
                    "ref {} names missing RefState {}",
                    pointer.ref_name, pointer.ref_state_id
                ))
            })?;
        let ref_state_payload = prikk_object::RefStatePayload::decode_canonical(
            &ref_state_envelope.canonical_payload,
            ref_state_envelope.schema_version,
        )?;
        let tip_block_id = resolve_ref_tip_block(&object_store, &ref_state_payload)?;
        reachable.extend(patch_ids_reachable_from_block(&object_store, tip_block_id)?);
    }

    Ok(all_patch_ids.difference(&reachable).copied().collect())
}

#[cfg(test)]
mod exchange_test_support;
#[cfg(test)]
mod tests;
