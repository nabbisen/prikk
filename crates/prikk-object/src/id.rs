//! Object identifiers and object type codes.

use core::fmt;
use core::str::FromStr;

use prikk_error::{PrikkError, Result};
use prikk_hash::{sha256, to_hex};

/// Single domain used for object identity preimages.
pub const OBJECT_ID_DOMAIN: &[u8] = b"PRIKK-OBJECT-ID-v1";

/// A PRIKK object type code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum ObjectType {
    /// Patch object.
    Patch = 1,
    /// Block object.
    Block = 2,
    /// RefState object.
    RefState = 3,
    /// Tag object.
    Tag = 4,
    /// Attestation object.
    Attestation = 5,
    /// Blob object.
    Blob = 6,
    /// Inline ref-update event payload when promoted to object form in future versions.
    RefUpdate = 7,
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
            1 => Ok(Self::Patch),
            2 => Ok(Self::Block),
            3 => Ok(Self::RefState),
            4 => Ok(Self::Tag),
            5 => Ok(Self::Attestation),
            6 => Ok(Self::Blob),
            7 => Ok(Self::RefUpdate),
            other => Err(PrikkError::MalformedData(format!("unknown object type code: {other}"))),
        }
    }

    /// Return a stable human-readable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Patch => "patch",
            Self::Block => "block",
            Self::RefState => "ref-state",
            Self::Tag => "tag",
            Self::Attestation => "attestation",
            Self::Blob => "blob",
            Self::RefUpdate => "ref-update",
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
mod tests {
    use super::{ObjectId, ObjectType};

    #[test]
    fn object_id_is_deterministic() {
        let a = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"payload");
        let b = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"payload");
        let c = ObjectId::from_canonical_payload(ObjectType::Block, 1, b"payload");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.to_hex(), "5f8711b3f84991d60b65221d66ed5ec260d28cc19c5c4ed3c1fe44d334265fe6");
    }

    #[test]
    fn hex_roundtrip() {
        let id = ObjectId::from_canonical_payload(ObjectType::Patch, 1, b"payload");
        let text = id.to_hex();
        let parsed = text.parse::<ObjectId>();
        assert_eq!(parsed, Ok(id));
    }
}
