//! RFC 115 Stage 2 (design-v1.md D3, §3), amended by §11 (D6): what a receiver may check about a
//! `RecognitionClaim` against its own object store.
//!
//! **Must not**: `block_id`/`patch_ids` are never existence-checked. A claim is verifiable with
//! none of the objects it names present — that is the entire reason it is a claim object and not a
//! Block (D3). This module never refuses on absence.
//!
//! **Must**: if the receiver *does* hold the referenced block, the claim's `patch_ids` must match
//! that block's own `patch_ids` **in order** — a claim contradicting a block already held is a
//! detected lie.
//!
//! **The comparison is exact sequence equality, not set equality (D6).** A block is
//! content-addressed, so the same `block_id` names the same canonical payload, therefore the same
//! `patch_ids` sequence. An honest claim about a block the receiver genuinely holds therefore
//! matches it in order, always — there is no honest way to name the right block and the right
//! patches in the wrong order. A differently-ordered claim about a held block cannot arise from
//! honesty; only from a lie or from a lossy claim format. So sequence equality cannot produce a
//! false accusation; it can only detect one — which set equality, used before this amendment,
//! structurally could not do, since order is exactly the information a set discards. Neither side
//! is sorted or deduplicated before comparing: `Block.patch_ids` is the free sequence
//! `apply_candidate_patches` actually consumed, and `RecognitionClaimPayload.patch_ids` mirrors it
//! verbatim by construction (the payload's own decoder/encoder no longer accept anything else).

use prikk_error::Result;
use prikk_object::{BlockPayload, ObjectId, ObjectType, RecognitionClaimPayload};

use crate::object_store::ObjectReader;

/// The outcome of checking a `RecognitionClaim` against the receiver's own store. Three states,
/// not a `bool` and not a `Result<()>` that would flatten "absent" into "fine" — `BlockAbsent` is
/// the expected case in real exchange and must not read as a degraded one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecognitionClaimConsistency {
    /// The referenced block is held, and its `patch_ids` match the claim's, in order.
    Consistent,
    /// The referenced block is not held. Expected, not a defect — the claim is still meaningful.
    BlockAbsent,
    /// The referenced block is held, and its `patch_ids` do **not** match the claim's, in order —
    /// a detected lie.
    Contradicted {
        /// The claim's own `patch_ids`, verbatim.
        claimed: Vec<ObjectId>,
        /// The held block's own `patch_ids`, verbatim.
        actual: Vec<ObjectId>,
    },
}

/// Check `claim` against `object_store`. See the module doc for why sequence equality, not set
/// equality, is the correct comparison under D6's verbatim-order contract.
pub fn check_recognition_claim_consistency(
    object_store: &impl ObjectReader,
    claim: &RecognitionClaimPayload,
) -> Result<RecognitionClaimConsistency> {
    let Some(block_envelope) = object_store.read_typed(claim.block_id, ObjectType::Block)? else {
        return Ok(RecognitionClaimConsistency::BlockAbsent);
    };
    let block_payload = BlockPayload::decode_canonical(&block_envelope.canonical_payload)?;
    let actual = block_payload.patch_ids;
    if actual == claim.patch_ids {
        Ok(RecognitionClaimConsistency::Consistent)
    } else {
        Ok(RecognitionClaimConsistency::Contradicted {
            claimed: claim.patch_ids.clone(),
            actual,
        })
    }
}

#[cfg(test)]
mod tests;
