//! Signature metadata and signed-byte construction.

use prikk_error::{PrikkError, Result};
use std::cmp::Ordering;

use crate::{CanonicalEncode, CanonicalWriter, ObjectId, ObjectType};

/// Signature domain string.
pub const SIGNATURE_DOMAIN: &[u8] = b"prikk.sig.v1";

/// Maximum byte length for a role-bound signature key id.
pub const SIGNATURE_KEY_ID_MAX_LEN: usize = 128;

/// Required byte length for a version-1 Ed25519 signature.
pub const ED25519_SIGNATURE_LEN: usize = 64;

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
            other => Err(PrikkError::InvalidSignature(format!(
                "unknown signer role code: {other}"
            ))),
        }
    }
}

/// Signature attached to an object envelope.
///
/// Standalone canonical encoding is structural field encoding only. Strict semantic admission is an
/// [`crate::ObjectEnvelope`] responsibility.
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
    /// Validate a key id used in role-bound signature preimages.
    pub fn validate_key_id(key_id: &str) -> Result<()> {
        if key_id.is_empty() {
            return Err(PrikkError::InvalidSignature(
                "signature key_id must not be empty".to_string(),
            ));
        }
        if key_id.len() > SIGNATURE_KEY_ID_MAX_LEN {
            return Err(PrikkError::InvalidSignature(format!(
                "signature key_id must be at most {SIGNATURE_KEY_ID_MAX_LEN} bytes"
            )));
        }
        if !key_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(PrikkError::InvalidSignature(
                "signature key_id must contain only ASCII letters, digits, '-' or '_'".to_string(),
            ));
        }
        Ok(())
    }

    /// Build the bytes to sign for an object ID and role.
    pub fn signed_bytes(
        algorithm: SignatureAlgorithm,
        object_type: ObjectType,
        object_id: ObjectId,
        signer_role: SignerRole,
        key_id: &str,
    ) -> Result<Vec<u8>> {
        Self::validate_key_id(key_id)?;
        let key_id_len = u16::try_from(key_id.len()).map_err(|_| {
            PrikkError::InvalidSignature(
                "signature key_id is too long for the signature preimage length field".to_string(),
            )
        })?;
        let mut out =
            Vec::with_capacity(SIGNATURE_DOMAIN.len() + 2 + 2 + 32 + 2 + 2 + key_id.len());
        out.extend_from_slice(SIGNATURE_DOMAIN);
        out.extend_from_slice(&algorithm.code().to_be_bytes());
        out.extend_from_slice(&object_type.code().to_be_bytes());
        out.extend_from_slice(object_id.as_bytes());
        out.extend_from_slice(&signer_role.code().to_be_bytes());
        out.extend_from_slice(&key_id_len.to_be_bytes());
        out.extend_from_slice(key_id.as_bytes());
        Ok(out)
    }

    /// Validate local structural constraints.
    pub fn validate(&self) -> Result<()> {
        Self::validate_key_id(&self.key_id)?;
        if self.signature_bytes.is_empty() {
            return Err(PrikkError::InvalidSignature(
                "signature bytes must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate the registered algorithm's syntactic signature shape.
    pub fn validate_shape(&self) -> Result<()> {
        match self.algorithm {
            SignatureAlgorithm::Ed25519 if self.signature_bytes.len() == ED25519_SIGNATURE_LEN => {
                Ok(())
            }
            SignatureAlgorithm::Ed25519 => Err(PrikkError::InvalidSignature(format!(
                "Ed25519 signature must be {ED25519_SIGNATURE_LEN} bytes, got {}",
                self.signature_bytes.len()
            ))),
        }
    }

    /// Compare signatures by the canonical envelope tuple.
    #[must_use]
    pub fn canonical_cmp(&self, other: &Self) -> Ordering {
        self.key_id
            .as_bytes()
            .cmp(other.key_id.as_bytes())
            .then_with(|| self.signer_role.code().cmp(&other.signer_role.code()))
            .then_with(|| self.algorithm.code().cmp(&other.algorithm.code()))
            .then_with(|| self.signature_bytes.cmp(&other.signature_bytes))
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
