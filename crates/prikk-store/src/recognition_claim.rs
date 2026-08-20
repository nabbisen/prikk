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

use prikk_error::{PrikkError, Result};
use prikk_object::{
    BlockPayload, ObjectEnvelope, ObjectId, ObjectType, RecognitionClaimPayload, Signature,
    SignatureAlgorithm, SignerRole,
};

use crate::layout::RepositoryLayout;
use crate::object_store::ObjectReader;
use crate::trust::{MaintainerTrustPolicy, load_maintainer_trust_policy};
use crate::trust_index::read_current_trust_policy_snapshot;

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

/// The outcome of checking one `RecognitionClaim`'s own MAINTAINER signature (Stage 3 handoff §4.2
/// item 8; reused by Stage 4 §3's "report the outcome alongside the result"). Shaped identically to
/// `AuthorSignatureVerification` and for the same reason: **never gating** (design D3) means a
/// claim naming a `key_id` this repository has not adopted still accepts -- it reads
/// `Unverifiable`, never `Sound`, and does not by itself refuse. Only a signature that fails to
/// verify against a `key_id` this repository *has* adopted refuses (a forged claim under a
/// locally-trusted identity is an integrity failure, not a trust question). There is no `Fails`
/// variant for the same reason `AuthorSignatureVerification` has none: that outcome is a genuine
/// refusal, propagated as an `Err`, not a value this type carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimSignatureVerification {
    /// The signature verifies against a `key_id` this repository has adopted as a trusted
    /// maintainer.
    Sound {
        /// The MAINTAINER key id the signature named and verified against.
        key_id: String,
    },
    /// This repository has not adopted `key_id`, so the signature cannot be checked. Not a
    /// failure and not by itself a refusal -- see the type doc.
    Unverifiable {
        /// The MAINTAINER key id named, which this repository has not adopted.
        key_id: String,
    },
}

/// Verify `envelope`'s own MAINTAINER signature against `trust_policy`. The single definition of
/// *how* a claim's signature is checked -- Stage 3's accept path and Stage 4's seal-from-accepted
/// path both call this rather than each carrying their own copy, the same reason
/// `verify_author_signature_against_material` was extracted for AUTHOR signatures. Refuses
/// (`Err`) if `envelope` carries no MAINTAINER signature at all, if the signature's algorithm is
/// not Ed25519, or if it fails to verify against an *adopted* key; reads `Unverifiable` rather
/// than refusing when `key_id` is simply not adopted -- see `ClaimSignatureVerification`'s own doc.
pub(crate) fn verify_claim_signature(
    envelope: &ObjectEnvelope,
    trust_policy: &MaintainerTrustPolicy,
) -> Result<ClaimSignatureVerification> {
    let claim_id = envelope.object_id();
    let Some(signature) = envelope
        .signatures
        .iter()
        .find(|signature| signature.signer_role == SignerRole::Maintainer)
    else {
        return Err(PrikkError::Integrity(format!(
            "recognition claim {claim_id} carries no MAINTAINER signature -- a claim is, by \
             definition, signed by the sender's maintainer key"
        )));
    };
    if signature.algorithm != SignatureAlgorithm::Ed25519 {
        return Err(PrikkError::InvalidSignature(format!(
            "recognition claim {claim_id} MAINTAINER signature is not Ed25519"
        )));
    }
    match trust_policy
        .keys
        .iter()
        .find(|adopted| adopted.key_id == signature.key_id)
    {
        None => Ok(ClaimSignatureVerification::Unverifiable {
            key_id: signature.key_id.clone(),
        }),
        Some(adopted) => {
            let preimage = Signature::signed_bytes(
                SignatureAlgorithm::Ed25519,
                envelope.object_type,
                claim_id,
                SignerRole::Maintainer,
                &signature.key_id,
            )?;
            if prikk_crypto::verify_ed25519(
                &adopted.public_key,
                &preimage,
                &signature.signature_bytes,
            )
            .is_err()
            {
                return Err(PrikkError::InvalidSignature(format!(
                    "recognition claim {claim_id} MAINTAINER signature does not verify against \
                     adopted key {}",
                    signature.key_id
                )));
            }
            Ok(ClaimSignatureVerification::Sound {
                key_id: signature.key_id.clone(),
            })
        }
    }
}

/// `load_maintainer_trust_policy` deliberately errors when no policy snapshot has ever been
/// appended -- correct for *publication* trust (`trust.rs`'s own module doc: a repository with no
/// adopted maintainer is a trust failure for every publication), because a Block/RefState needs a
/// definitively trusted signer to be considered sealed at all. A `RecognitionClaim`'s own signature
/// check has no such requirement: design D3 rules a claim **never gates** on trust, so a repository
/// that has never adopted anyone must read every claim's signer as simply not adopted -- the same
/// outcome as an adopted-but-empty policy would produce -- not refuse over a question claim
/// verification was never supposed to ask. A genuinely damaged policy or key-material container
/// still propagates its error unchanged; only the "nothing has ever been adopted" case is treated
/// as empty here.
pub(crate) fn maintainer_trust_policy_or_empty(
    layout: &RepositoryLayout,
) -> Result<MaintainerTrustPolicy> {
    match read_current_trust_policy_snapshot(layout)? {
        Some(_) => load_maintainer_trust_policy(layout),
        None => Ok(MaintainerTrustPolicy { keys: Vec::new() }),
    }
}

#[cfg(test)]
mod tests;
