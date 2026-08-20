//! RFC 115 Stage 2 (design-v1.md D3, §3), amended by §11 (D6) and by RFC 116 N3: what a receiver
//! may check about a `RecognitionClaim` against its own object store.
//!
//! **Must not**: `block_id`/`patch_ids`/`parent_block_ids` are never existence-checked. A claim is
//! verifiable with none of the objects it names present — that is the entire reason it is a claim
//! object and not a Block (D3). This module never refuses on absence.
//!
//! **Must**: if the receiver *does* hold the referenced block, **both** the claim's `patch_ids` and
//! its `parent_block_ids` must match that block's own, **in order** — a claim contradicting a block
//! already held is a detected lie, about whichever field disagreed.
//!
//! **The comparison is exact sequence equality, not set equality (D6; extended to
//! `parent_block_ids` by N3).** A block is content-addressed, so the same `block_id` names the same
//! canonical payload, therefore the same `patch_ids` sequence **and** the same `parent_block_ids`
//! sequence. An honest claim about a block the receiver genuinely holds therefore matches it in
//! order, always, on both fields — there is no honest way to name the right block and the wrong
//! patches, or the right block and the wrong parents. A differently-ordered (or differently-valued)
//! claim about a held block cannot arise from honesty; only from a lie or from a lossy claim
//! format. So sequence equality cannot produce a false accusation; it can only detect one — which
//! set equality, used before D6, structurally could not do, since order is exactly the information
//! a set discards. Neither side is sorted or deduplicated before comparing: `Block.patch_ids` and
//! `Block.parent_block_ids` are the free sequences the block itself carries, and
//! `RecognitionClaimPayload`'s own two fields mirror them verbatim by construction (the payload's
//! own decoder/encoder no longer accept anything else).
//!
//! **`Contradicted` names which field disagreed (N3 §4).** A parent mismatch reported through
//! `patch_ids`-shaped output would read as "your patches disagree" when the patches are fine — a
//! misleading diagnostic of exactly the class RFC 115 Stage 4's divergence-vs-corruption ruling
//! exists to prevent. A wrong explanation is worse than a vague one: it sends the reader somewhere
//! false.

use prikk_error::{PrikkError, Result};
use prikk_object::{
    BlockPayload, ObjectEnvelope, ObjectId, ObjectType, RecognitionClaimPayload, Signature,
    SignatureAlgorithm, SignerRole,
};

use crate::layout::RepositoryLayout;
use crate::object_store::ObjectReader;
use crate::trust::{MaintainerTrustPolicy, load_maintainer_trust_policy};
use crate::trust_index::read_current_trust_policy_snapshot;

/// Which field of a `RecognitionClaimPayload` a `RecognitionClaimConsistency::Contradicted`
/// outcome names as the one that disagreed with the held block (N3 §4) -- lets a caller
/// distinguish "the claim lies about which patches" from "the claim lies about which parents"
/// without parsing a string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContradictedField {
    /// The claim's `patch_ids` disagree with the held block's own.
    PatchIds,
    /// The claim's `parent_block_ids` disagree with the held block's own.
    ParentBlockIds,
}

/// The outcome of checking a `RecognitionClaim` against the receiver's own store. Three states,
/// not a `bool` and not a `Result<()>` that would flatten "absent" into "fine" — `BlockAbsent` is
/// the expected case in real exchange and must not read as a degraded one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecognitionClaimConsistency {
    /// The referenced block is held, and both the claim's `patch_ids` and `parent_block_ids`
    /// match the block's own, in order.
    Consistent,
    /// The referenced block is not held. Expected, not a defect — the claim is still meaningful.
    BlockAbsent,
    /// The referenced block is held, and one of the claim's fields does **not** match the block's
    /// own, in order — a detected lie. `field` names which one; `patch_ids` is checked first, so a
    /// claim disagreeing on both fields is reported as a `PatchIds` contradiction.
    Contradicted {
        /// Which field disagreed.
        field: ContradictedField,
        /// That field's own value as the claim states it, verbatim.
        claimed: Vec<ObjectId>,
        /// That field's own value as the held block actually has it, verbatim.
        actual: Vec<ObjectId>,
    },
}

/// Check `claim` against `object_store`. See the module doc for why sequence equality, not set
/// equality, is the correct comparison under D6/N3's verbatim-order contract, and for why
/// `Contradicted` names the disagreeing field.
pub fn check_recognition_claim_consistency(
    object_store: &impl ObjectReader,
    claim: &RecognitionClaimPayload,
) -> Result<RecognitionClaimConsistency> {
    let Some(block_envelope) = object_store.read_typed(claim.block_id, ObjectType::Block)? else {
        return Ok(RecognitionClaimConsistency::BlockAbsent);
    };
    let block_payload = BlockPayload::decode_canonical(&block_envelope.canonical_payload)?;
    if block_payload.patch_ids != claim.patch_ids {
        return Ok(RecognitionClaimConsistency::Contradicted {
            field: ContradictedField::PatchIds,
            claimed: claim.patch_ids.clone(),
            actual: block_payload.patch_ids,
        });
    }
    if block_payload.parent_block_ids != claim.parent_block_ids {
        return Ok(RecognitionClaimConsistency::Contradicted {
            field: ContradictedField::ParentBlockIds,
            claimed: claim.parent_block_ids.clone(),
            actual: block_payload.parent_block_ids,
        });
    }
    Ok(RecognitionClaimConsistency::Consistent)
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
