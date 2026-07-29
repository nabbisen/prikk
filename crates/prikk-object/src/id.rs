//! Object identifiers and object type codes.

use core::fmt;
use core::str::FromStr;

use prikk_error::{PrikkError, Result};
use prikk_hash::{sha256, to_hex};

/// Single domain used for object identity preimages.
pub const OBJECT_ID_DOMAIN: &[u8] = b"PRIKK-OBJECT-ID-v1";

/// A Prikk object type code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum ObjectType {
    /// Patch object.
    Patch = 0x01,
    /// Block object.
    Block = 0x02,
    /// RefState object.
    RefState = 0x03,
    /// RefUpdate event. Object-envelope type stored inline in `refs/logs/`
    /// (journal then log), not a permanent object-store directory.
    RefUpdate = 0x04,
    /// Tag object.
    Tag = 0x05,
    /// Attestation object.
    Attestation = 0x06,
    /// Blob object.
    Blob = 0x07,
    /// Rebuildable block-summary cache. Uses the canonical codec for
    /// reproducibility but is never a root of trust or part of block identity.
    BlockSummaryCache = 0x08,
    /// Signed doctor-repair note stored inline in `refs/recovery/`. Never a
    /// `RefUpdate` substitute (FDD-02 §10.4).
    RecoveryNote = 0x09,
    /// Project identity anchor; its `ObjectId` is the `project_id` (FDD-03 §9.13).
    ProjectGenesis = 0x0A,
}

impl ObjectType {
    /// Return the stable u16 code used in object identity bytes.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// Parse a stable u16 code.
    pub fn from_code(code: u16) -> Result<Self> {
        match code {
            0x01 => Ok(Self::Patch),
            0x02 => Ok(Self::Block),
            0x03 => Ok(Self::RefState),
            0x04 => Ok(Self::RefUpdate),
            0x05 => Ok(Self::Tag),
            0x06 => Ok(Self::Attestation),
            0x07 => Ok(Self::Blob),
            0x08 => Ok(Self::BlockSummaryCache),
            0x09 => Ok(Self::RecoveryNote),
            0x0A => Ok(Self::ProjectGenesis),
            other => Err(PrikkError::MalformedData(format!(
                "unknown object type code: {other}"
            ))),
        }
    }

    /// Return a stable human-readable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Patch => "patch",
            Self::Block => "block",
            Self::RefState => "ref-state",
            Self::RefUpdate => "ref-update",
            Self::Tag => "tag",
            Self::Attestation => "attestation",
            Self::Blob => "blob",
            Self::BlockSummaryCache => "block-summary-cache",
            Self::RecoveryNote => "recovery-note",
            Self::ProjectGenesis => "project-genesis",
        }
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A 32-byte object identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId([u8; 32]);

impl ObjectId {
    /// Construct an object ID from raw bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Return raw ID bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Compute an object ID from object type, schema version, and unsigned canonical payload.
    #[must_use]
    pub fn from_canonical_payload(
        object_type: ObjectType,
        schema_version: u32,
        canonical_payload: &[u8],
    ) -> Self {
        let mut preimage =
            Vec::with_capacity(OBJECT_ID_DOMAIN.len() + 2 + 4 + 8 + canonical_payload.len());
        preimage.extend_from_slice(OBJECT_ID_DOMAIN);
        preimage.extend_from_slice(&object_type.code().to_be_bytes());
        preimage.extend_from_slice(&schema_version.to_be_bytes());
        preimage.extend_from_slice(&(canonical_payload.len() as u64).to_be_bytes());
        preimage.extend_from_slice(canonical_payload);
        Self(sha256(&preimage))
    }

    /// Return lowercase hex.
    #[must_use]
    pub fn to_hex(&self) -> String {
        to_hex(&self.0)
    }
}

impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObjectId({})", self.to_hex())
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl FromStr for ObjectId {
    type Err = PrikkError;

    fn from_str(s: &str) -> Result<Self> {
        if s.len() != 64 {
            return Err(PrikkError::InvalidObjectId(format!(
                "expected 64 lowercase hex chars, got {}",
                s.len()
            )));
        }
        let mut out = [0_u8; 32];
        for (slot, pair) in out.iter_mut().zip(s.as_bytes().chunks_exact(2)) {
            let mut bytes = pair.iter().copied();
            let high = bytes.next().ok_or_else(|| {
                PrikkError::InvalidObjectId("hex pair is unexpectedly short".to_string())
            })?;
            let low = bytes.next().ok_or_else(|| {
                PrikkError::InvalidObjectId("hex pair is unexpectedly short".to_string())
            })?;
            *slot = (hex_value(high)? << 4) | hex_value(low)?;
        }
        Ok(Self(out))
    }
}

fn hex_value(byte: u8) -> Result<u8> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(PrikkError::InvalidObjectId(
            "object IDs must use lowercase hex only".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests;
