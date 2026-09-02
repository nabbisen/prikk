#![forbid(unsafe_code)]

//! Prikk cryptographic primitives.
//!
//! v1 scope is intentionally minimal: Ed25519 keypair construction, detached signing, and detached
//! verification. Trust stores, key persistence, key rotation, revocation, and signature policy are
//! out of scope here and belong to later phases (RFC-025). This crate is the single home for the v1
//! signing/verification algorithm so authoring, sealing, and verification cannot diverge.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use prikk_error::{PrikkError, Result};

/// Length in bytes of an Ed25519 secret seed and of an Ed25519 public key.
pub const ED25519_KEY_LEN: usize = 32;
/// Length in bytes of an Ed25519 detached signature.
pub const ED25519_SIGNATURE_LEN: usize = 64;

/// An Ed25519 keypair used to produce detached signatures.
pub struct Ed25519KeyPair {
    signing: SigningKey,
}

impl Ed25519KeyPair {
    /// Construct a keypair from a 32-byte secret seed.
    ///
    /// Used for caller-provided key material and for deterministic test keys. The seed is the
    /// Ed25519 secret scalar source; callers are responsible for its confidentiality.
    #[must_use]
    pub fn from_seed(seed: &[u8; ED25519_KEY_LEN]) -> Self {
        Self {
            signing: SigningKey::from_bytes(seed),
        }
    }

    /// Generate a fresh keypair from the operating-system CSPRNG.
    ///
    /// Fails closed if the OS entropy source is unavailable.
    pub fn generate() -> Result<Self> {
        let mut seed = [0_u8; ED25519_KEY_LEN];
        getrandom::fill(&mut seed)
            .map_err(|e| PrikkError::Integrity(format!("OS CSPRNG unavailable: {e}")))?;
        Ok(Self::from_seed(&seed))
    }

    /// The 32-byte public (verifying) key for this keypair.
    #[must_use]
    pub fn public_key_bytes(&self) -> [u8; ED25519_KEY_LEN] {
        self.signing.verifying_key().to_bytes()
    }

    /// Produce a detached 64-byte Ed25519 signature over `message`.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> [u8; ED25519_SIGNATURE_LEN] {
        self.signing.sign(message).to_bytes()
    }
}

/// Verify a detached Ed25519 `signature` over `message` against a 32-byte `public_key`.
///
/// Uses strict verification (rejects non-canonical encodings and small-order keys). Returns an
/// error if the public key or signature is malformed, or if verification fails.
pub fn verify_ed25519(
    public_key: &[u8; ED25519_KEY_LEN],
    message: &[u8],
    signature: &[u8],
) -> Result<()> {
    let verifying = VerifyingKey::from_bytes(public_key)
        .map_err(|e| PrikkError::InvalidSignature(format!("malformed public key: {e}")))?;
    let signature_array: [u8; ED25519_SIGNATURE_LEN] = signature.try_into().map_err(|_| {
        PrikkError::InvalidSignature(format!(
            "signature must be {ED25519_SIGNATURE_LEN} bytes, got {}",
            signature.len()
        ))
    })?;
    let signature = Signature::from_bytes(&signature_array);
    verifying
        .verify_strict(message, &signature)
        .map_err(|e| PrikkError::InvalidSignature(format!("signature verification failed: {e}")))
}

#[cfg(test)]
mod tests;
