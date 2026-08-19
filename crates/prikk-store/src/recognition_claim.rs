//! RFC 115 Stage 2 (design-v1.md D3, §3): what a receiver may check about a `RecognitionClaim`
//! against its own object store.
//!
//! **Must not**: `block_id`/`patch_ids` are never existence-checked. A claim is verifiable with
//! none of the objects it names present — that is the entire reason it is a claim object and not a
//! Block (D3). This module never refuses on absence.
//!
//! **Must**: if the receiver *does* hold the referenced block, the claim's `patch_ids` must match
//! that block's own `patch_ids` — a claim contradicting a block already held is a detected lie.
//!
//! **Also must**: a malformed claim (unsorted or duplicate `patch_ids`) is refused outright, never
//! compared and reported as `Contradicted` — that outcome must mean "this sender lied," never "this
//! caller built the struct wrong" (review v1 §2).

use prikk_error::{PrikkError, Result};
use prikk_object::canonical::is_strictly_sorted;
use prikk_object::{BlockPayload, ObjectId, ObjectType, RecognitionClaimPayload};

use crate::object_store::ObjectReader;

/// The outcome of checking a `RecognitionClaim` against the receiver's own store. Three states,
/// not a `bool` and not a `Result<()>` that would flatten "absent" into "fine" — `BlockAbsent` is
/// the expected case in real exchange and must not read as a degraded one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecognitionClaimConsistency {
    /// The referenced block is held, and its `patch_ids` match the claim's, as sets.
    Consistent,
    /// The referenced block is not held. Expected, not a defect — the claim is still meaningful.
    BlockAbsent,
    /// The referenced block is held, and its `patch_ids` do **not** match the claim's — a detected
    /// lie. Both sides are reported as sorted, deduplicated sets, matching how the mismatch is
    /// actually judged (see the module-level note on ordering below).
    Contradicted {
        /// The claim's own `patch_ids`, already sorted and deduplicated by construction.
        claimed: Vec<ObjectId>,
        /// The held block's `patch_ids`, sorted here for comparison.
        actual: Vec<ObjectId>,
    },
}

/// Check `claim` against `object_store`. **Comparison is by set, not by sequence**: `BlockPayload`'s
/// own `patch_ids` field is in WAL replay / authoring order (`prikk-cli/src/seal.rs`'s
/// `persist_wal_patches`, not sorted by `ObjectId`), while `RecognitionClaimPayload.patch_ids` is
/// sorted and deduplicated by construction. A literal sequence comparison would report a genuinely
/// truthful claim as contradicted almost every time a block seals more than one patch, since
/// authoring order and `ObjectId` sort order are unrelated. A claim asserts *which* patches were
/// sealed, not in what order — sorting the block's own list before comparing is what "the claim's
/// `patch_ids` must equal the block's own `patch_ids`" (design §3) actually means.
///
/// **Refuses, rather than normalizes, a claim whose own `patch_ids` are unsorted or contain
/// duplicates.** `RecognitionClaimPayload`'s fields are `pub` with no invariant-enforcing
/// constructor: the encoder and decoder both reject unsorted/duplicate input, so anything that
/// round-tripped through bytes is already sorted, but a claim built in-process (Stage 3's own
/// path, before encoding) is not checked by anything else before it reaches here. Comparing an
/// out-of-order claim against a normalized block would report a truthful claim as `Contradicted`
/// -- the exact failure mode the block-side normalization above exists to prevent, reintroduced
/// through the one door that skips it (review v1 §2). `Contradicted` must mean "this sender lied,"
/// never "this caller built the struct wrong" -- so this is a hard refusal, the same rule Stage 1's
/// `compute_patch_set_digest` and this type's own encoder/decoder already follow, not a silent sort.
pub fn check_recognition_claim_consistency(
    object_store: &impl ObjectReader,
    claim: &RecognitionClaimPayload,
) -> Result<RecognitionClaimConsistency> {
    if !is_strictly_sorted(&claim.patch_ids) {
        return Err(PrikkError::Integrity(
            "RecognitionClaim patch_ids are not sorted and unique -- refusing rather than \
             comparing, since an out-of-order claim would misreport as contradicted against a \
             normalized block"
                .to_string(),
        ));
    }
    let Some(block_envelope) = object_store.read_typed(claim.block_id, ObjectType::Block)? else {
        return Ok(RecognitionClaimConsistency::BlockAbsent);
    };
    let block_payload = BlockPayload::decode_canonical(&block_envelope.canonical_payload)?;
    let mut actual = block_payload.patch_ids;
    actual.sort_unstable();
    actual.dedup();
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
