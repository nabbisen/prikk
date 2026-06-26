//! Signature metadata and signed-byte construction.

use prikk_error::{PrikkError, Result};

use crate::{CanonicalEncode, CanonicalWriter, ObjectId, ObjectType};

/// Signature domain string.
pub const SIGNATURE_DOMAIN: &[u8] = b"prikk.sig.v1";

/// Supported signature algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SignatureAlgorithm {
    /// Ed25519. The only v1 signing and verification algorithm.
    Ed25519 = 1,
}

impl SignatureAlgorithm {
    /// Stable u16 code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// Parse a stable u16 code.
    pub fn from_code(code: u16) -> Result<Self> {
        match code {
            1 => Ok(Self::Ed25519),
            other => Err(PrikkError::InvalidSignature(format!(
                "unknown signature algorithm code: {other}"
            ))),
        }
    }
}

/// Role bound into signature preimages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SignerRole {
    /// Author of a patch.
    Author = 1,
    /// Maintainer publishing/sealing a block or ref state.
    Maintainer = 2,
    /// Continuous-integration actor.
    Ci = 3,
    /// Audit plugin or audit policy signer.
    Audit = 4,
}

impl SignerRole {
    /// Stable u16 code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// Parse a stable u16 code.
    pub fn from_code(code: u16) -> Result<Self> {
        match code {
            1 => Ok(Self::Author),
            2 => Ok(Self::Maintainer),
            3 => Ok(Self::Ci),
            4 => Ok(Self::Audit),
            other => {
                Err(PrikkError::InvalidSignature(format!("unknown signer role code: {other}")))
            }
        }
    }
}

/// Signature attached to an object envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// Signature algorithm.
    pub algorithm: SignatureAlgorithm,
    /// Key identifier.
    pub key_id: String,
    /// Raw signature bytes.
    pub signature_bytes: Vec<u8>,
    /// Advisory signing timestamp. Do not use as authoritative audit time.
    pub created_at: u64,
    /// Signer role.
    pub signer_role: SignerRole,
}

impl Signature {
    /// Build the bytes to sign for an object ID and role.
    #[must_use]
    pub fn signed_bytes(
        algorithm: SignatureAlgorithm,
        object_type: ObjectType,
        object_id: ObjectId,
        signer_role: SignerRole,
        key_id: &str,
    ) -> Vec<u8> {
        let mut out = Vec::with_capacity(SIGNATURE_DOMAIN.len() + 2 + 32 + 2 + 2 + key_id.len());
        out.extend_from_slice(SIGNATURE_DOMAIN);
        out.extend_from_slice(&algorithm.code().to_be_bytes());
        out.extend_from_slice(&object_type.code().to_be_bytes());
        out.extend_from_slice(object_id.as_bytes());
        out.extend_from_slice(&signer_role.code().to_be_bytes());
        out.extend_from_slice(&(key_id.len() as u16).to_be_bytes());
        out.extend_from_slice(key_id.as_bytes());
        out
    }

    /// Validate local structural constraints.
    pub fn validate(&self) -> Result<()> {
        if self.key_id.is_empty() {
            return Err(PrikkError::InvalidSignature(
                "signature key_id must not be empty".to_string(),
            ));
        }
        if self.signature_bytes.is_empty() {
            return Err(PrikkError::InvalidSignature(
                "signature bytes must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

impl CanonicalEncode for Signature {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_u32(1, self.algorithm.code() as u32)?;
        writer.field_string(2, &self.key_id)?;
        writer.field_bytes(3, &self.signature_bytes)?;
        writer.field_u64(4, self.created_at)?;
        writer.field_u32(5, self.signer_role.code() as u32)?;
        Ok(())
    }
}
