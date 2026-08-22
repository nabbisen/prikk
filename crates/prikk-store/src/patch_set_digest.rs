//! RFC 115 Stage 1 (design-v1.md §5, D4) — the patch-set digest: a canonical value over the set of
//! patch ids reachable from a ref, answering "are these two repositories the same?" at the level
//! where identity actually holds (RFC 115 §2.5-§2.7). Shaped like `state_root.rs`, not `id.rs`: a
//! comparison value, not a storable object -- a dedicated newtype, never an `ObjectId`, and no
//! `(object_type, schema_version)` pair.
//!
//! **RFC 117 T1: `PatchSetDigest` itself now lives in `prikk-object`**, re-exported here unchanged
//! (see this module's own `pub use` below) -- `TagPayload` carries one as its field 6, and
//! `prikk-object` cannot depend on `prikk-store`. Every function that *computes* a digest stays
//! here, the same split `MerkleRoot` (`prikk-object`) and `compute_state_root` (`prikk-store`)
//! already have.
//!
//! **Ref-kind support, per `RFC-115-stage-1-reachable-set-ruling-v1.md`:**
//! - `heads/*` (`RefKind::Branch`): `target_object_id` names the Block directly.
//! - `tags/*` (`RefKind::Tag`): `target_object_id` names a Tag object one hop away from the Block
//!   (`tag.rs`'s own model: "ref -> tag object -> block"); this module resolves that second hop
//!   itself, which `export_bundle` does not (`bundle-export-tag-ref-gap-v1.md`, a separate defect,
//!   not fixed here). The digest is the digest of the target Block's own patch-set closure only --
//!   the Tag object carries a name, an optional message and a signature, none of which are patches,
//!   and folding them in would make two repositories holding identical patches compare unequal over
//!   tag metadata, which inverts the whole purpose (ruling §2.2).
//! - `remotes/*` (received pointers): **refused explicitly**, not resolved. Not a difficulty --
//!   `received_index.rs`'s resolution is straightforward -- but the received namespace is precisely
//!   what patch-level exchange itself restructures (design D2/D5), and its digest semantics belong
//!   to Stage 3, not assumed here. Refusing by name check, before any `RefStore` lookup, so the
//!   error names the real reason rather than a misleading "ref does not exist" (ruling §2.3).
//! - Closed refs need no special case: `RefStatePayload.closed` gates further publication, it does
//!   not change what `target_object_id` names (ruling §2.4).

use std::collections::BTreeSet;

use prikk_error::{PrikkError, Result};
use prikk_hash::sha256;
use prikk_object::{ObjectId, ObjectType, RefKind, RefStatePayload, TagPayload};

use crate::layout::RepositoryLayout;
use crate::merge_evidence::ancestors_inclusive;
use crate::object_store::{ObjectReadSnapshot, ObjectReader};
use crate::refs::RefStore;

/// RFC 117 T1: the newtype itself now lives in `prikk-object` (`TagPayload` carries one, and
/// `prikk-object` cannot depend on this crate) -- re-exported here so every existing
/// `crate::patch_set_digest::PatchSetDigest` / `prikk_store::PatchSetDigest` path keeps resolving
/// to the same type, unchanged. Every function below that *computes* one stays here, matching
/// `MerkleRoot`/`compute_state_root`'s own split.
pub use prikk_object::PatchSetDigest;

const PATCH_SET_DIGEST_DOMAIN: &[u8] = b"PRIKK-PATCH-SET-DIGEST-v1";

/// Construct the exact preimage over an already-sorted, deduplicated, strictly-ascending slice of
/// patch ids. **Identity-bearing** (documented in `release-compatibility.md`'s frozen list): two
/// prikk versions must produce identical bytes over the same patch set, or the comparison this
/// digest exists for means nothing across an upgrade.
pub fn patch_set_digest_preimage(patch_ids: &[ObjectId]) -> Result<Vec<u8>> {
    if !prikk_object::canonical::is_strictly_sorted(patch_ids) {
        return Err(PrikkError::Integrity(
            "patch-set digest input is not strictly sorted and deduplicated".to_string(),
        ));
    }
    let count = u64::try_from(patch_ids.len())
        .map_err(|_| PrikkError::Integrity("patch-set digest count exceeds u64".to_string()))?;
    let mut preimage = Vec::with_capacity(PATCH_SET_DIGEST_DOMAIN.len() + 8 + patch_ids.len() * 32);
    preimage.extend_from_slice(PATCH_SET_DIGEST_DOMAIN);
    preimage.extend_from_slice(&count.to_be_bytes());
    for patch_id in patch_ids {
        preimage.extend_from_slice(patch_id.as_bytes());
    }
    Ok(preimage)
}

/// Compute the patch-set digest over an already-sorted, deduplicated slice of patch ids. The count
/// is hashed even when `patch_ids` is empty, so an empty set is distinguishable from a degenerate
/// one (matching `state_root.rs`'s own empty-case discipline, `compute_state_root`).
pub fn compute_patch_set_digest(patch_ids: &[ObjectId]) -> Result<PatchSetDigest> {
    Ok(PatchSetDigest(sha256(&patch_set_digest_preimage(
        patch_ids,
    )?)))
}

/// Every patch id reachable from `tip_block_id`'s ancestry, sorted and deduplicated -- the same
/// closure `export_bundle` walks (`bundle.rs:189,208-209`), narrowed to patch ids only (no blobs,
/// no attestations: Stage 1's scope is the patch set, nothing else, per the handoff's §6).
pub fn patch_ids_reachable_from_block(
    object_store: &impl ObjectReader,
    tip_block_id: ObjectId,
) -> Result<Vec<ObjectId>> {
    let ancestors = ancestors_inclusive(object_store, tip_block_id)?;
    let mut patch_ids: BTreeSet<ObjectId> = BTreeSet::new();
    for block in ancestors.values() {
        patch_ids.extend(block.patch_ids.iter().copied());
    }
    Ok(patch_ids.into_iter().collect())
}

/// Compute the patch-set digest for the closure reachable from `tip_block_id` directly -- the
/// block-rooted core, ref-resolution-agnostic, so a caller that has already resolved a ref through
/// any mechanism (`RefStore`, a received pointer, a future one) can reach the same computation
/// without this module re-deriving how to resolve it.
pub fn compute_patch_set_digest_from_block(
    object_store: &impl ObjectReader,
    tip_block_id: ObjectId,
) -> Result<PatchSetDigest> {
    compute_patch_set_digest(&patch_ids_reachable_from_block(object_store, tip_block_id)?)
}

/// Resolve `ref_name` to its target Block, per this module's own doc: `heads/*` directly, `tags/*`
/// through the Tag object's own `target_block_id`. `remotes/*` is refused explicitly before any
/// `RefStore` lookup is attempted, so the refusal names the real reason (ruling §2.3) rather than a
/// misleading "ref does not exist".
fn resolve_ref_to_tip_block(
    layout: &RepositoryLayout,
    object_store: &impl ObjectReader,
    ref_name: &str,
) -> Result<ObjectId> {
    if ref_name.starts_with("remotes/") {
        return Err(PrikkError::Integrity(format!(
            "patch-set digest does not support received refs ({ref_name}) yet -- the received \
             namespace is what patch-level exchange itself restructures (RFC 115 design D2/D5); \
             its digest semantics belong to a later stage, not assumed here"
        )));
    }
    let ref_store = RefStore::new(layout.clone());
    let Some(ref_state_id) = ref_store.read_current_ref_state_id(ref_name)? else {
        return Err(PrikkError::Integrity(format!(
            "ref {ref_name} does not exist, nothing to compute a patch-set digest for"
        )));
    };
    let ref_state_envelope = object_store
        .read_typed(ref_state_id, ObjectType::RefState)?
        .ok_or_else(|| PrikkError::Integrity(format!("missing RefState object: {ref_state_id}")))?;
    let ref_state_payload = RefStatePayload::decode_canonical(
        &ref_state_envelope.canonical_payload,
        ref_state_envelope.schema_version,
    )?;
    match ref_state_payload.kind {
        RefKind::Branch => Ok(ref_state_payload.target_object_id),
        RefKind::Tag => {
            let tag_id = ref_state_payload.target_object_id;
            let tag_envelope = object_store
                .read_typed(tag_id, ObjectType::Tag)?
                .ok_or_else(|| PrikkError::Integrity(format!("missing Tag object: {tag_id}")))?;
            let tag_payload = TagPayload::decode_canonical(&tag_envelope.canonical_payload)?;
            Ok(tag_payload.target_block_id)
        }
    }
}

/// The ref-rooted entry point, and the one two independent repositories can actually use: neither
/// side can name the other's Block id (that is the premise the digest exists to work around, RFC
/// 115 design §7), so a Block-rooted call alone would not serve the digest's own purpose.
pub fn compute_patch_set_digest_for_ref(
    layout: &RepositoryLayout,
    ref_name: &str,
) -> Result<PatchSetDigest> {
    let object_store = ObjectReadSnapshot::open(layout)?;
    let tip_block_id = resolve_ref_to_tip_block(layout, &object_store, ref_name)?;
    compute_patch_set_digest_from_block(&object_store, tip_block_id)
}

#[cfg(test)]
mod tests;
