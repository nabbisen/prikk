//! Empty commit scaffold helpers.

use prikk_hash::sha256;
use prikk_object::{
    CanonicalEncode, ObjectEnvelope, ObjectType, OperationCondition, OperationConditionEntry,
    PatchPayload, Signature, SignatureAlgorithm, SignerRole,
};

/// Build the current placeholder signed patch envelope for `--allow-empty` commits.
pub(crate) fn empty_patch_envelope(message: &str) -> std::result::Result<ObjectEnvelope, String> {
    let message_hash = sha256(message.as_bytes());
    let payload = PatchPayload {
        operations: Vec::new(),
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: vec![OperationConditionEntry {
            key: "prikk.dev.empty-commit-message-sha256".to_string(),
            value: OperationCondition::OldContentHash(message_hash.to_vec()),
        }],
    };
    let payload_bytes = payload
        .to_canonical_bytes()
        .map_err(|err| err.to_string())?;
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, payload_bytes);
    envelope
        .add_signature(dev_author_signature(message))
        .map_err(|err| err.to_string())?;
    Ok(envelope)
}

fn dev_author_signature(message: &str) -> Signature {
    let mut signature_preimage = Vec::new();
    signature_preimage.extend_from_slice(b"prikk.dev.placeholder-signature.v1");
    signature_preimage.extend_from_slice(message.as_bytes());
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "dev-placeholder-author".to_string(),
        signature_bytes: sha256(&signature_preimage).to_vec(),
        created_at: 0,
        signer_role: SignerRole::Author,
    }
}
