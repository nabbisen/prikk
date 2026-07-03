//! Maintainer signing boundary for publication objects.
//!
//! Publication signing uses the same role-bound signature preimage shape as AUTHOR signing, but with
//! `SignerRole::Maintainer` and the publication object's own type and identity.

use prikk_crypto::Ed25519KeyPair;
use prikk_error::Result;
use prikk_object::{ObjectId, ObjectType, Signature, SignatureAlgorithm, SignerRole};

/// A provider that produces detached signature bytes for a publication object.
pub trait MaintainerSigner {
    /// The non-empty key identifier recorded in the produced signature.
    fn key_id(&self) -> &str;
    /// Produce the detached signature bytes over the role-bound `preimage`.
    fn sign(&self, preimage: &[u8]) -> Result<Vec<u8>>;
    /// Return the signer's public Ed25519 key bytes.
    fn public_key_bytes(&self) -> [u8; 32];
}

/// Build a role-bound MAINTAINER [`Signature`] for an unsigned publication object.
pub fn maintainer_signature(
    signer: &impl MaintainerSigner,
    object_type: ObjectType,
    object_id: ObjectId,
) -> Result<Signature> {
    let preimage = Signature::signed_bytes(
        SignatureAlgorithm::Ed25519,
        object_type,
        object_id,
        SignerRole::Maintainer,
        signer.key_id(),
    );
    let signature_bytes = signer.sign(&preimage)?;
    Ok(Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: signer.key_id().to_string(),
        signature_bytes,
        created_at: 0,
        signer_role: SignerRole::Maintainer,
    })
}

/// Ed25519-backed maintainer signer built from caller-supplied key material.
pub struct Ed25519MaintainerSigner {
    key_id: String,
    key_pair: Ed25519KeyPair,
}

impl Ed25519MaintainerSigner {
    /// Construct a deterministic signer from a 32-byte Ed25519 secret seed.
    #[must_use]
    pub fn from_seed(key_id: impl Into<String>, seed: &[u8; 32]) -> Self {
        Self {
            key_id: key_id.into(),
            key_pair: Ed25519KeyPair::from_seed(seed),
        }
    }
}

impl MaintainerSigner for Ed25519MaintainerSigner {
    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn sign(&self, preimage: &[u8]) -> Result<Vec<u8>> {
        Ok(self.key_pair.sign(preimage).to_vec())
    }

    fn public_key_bytes(&self) -> [u8; 32] {
        self.key_pair.public_key_bytes()
    }
}
