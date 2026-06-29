//! Node identity and kind types (FDD-03 §9.3.0).

use prikk_error::{PrikkError, Result};

use crate::payload::blob::BlobKind;

/// Length of a node identity in bytes.
pub const NODE_ID_BYTES: usize = 32;

/// A 32-byte node identity (FDD-03 §9.3). Minted once at node creation and
/// preserved across rename; encoded as a `bytes` field, not an `object_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId([u8; NODE_ID_BYTES]);

impl NodeId {
    /// Wrap raw bytes as a node identity. Unchecked: accepts any 32 bytes, including
    /// the reserved all-zero value. Intended for fixtures and round-trip
    /// construction; production decode paths MUST use [`NodeId::try_from_bytes`].
    #[must_use]
    pub const fn from_bytes(bytes: [u8; NODE_ID_BYTES]) -> Self {
        Self(bytes)
    }

    /// Wrap validated bytes as a node identity, rejecting the all-zero reserved
    /// value. Production operation decoders use this.
    pub fn try_from_bytes(bytes: [u8; NODE_ID_BYTES]) -> Result<Self> {
        if bytes == [0_u8; NODE_ID_BYTES] {
            return Err(PrikkError::MalformedData(
                "node_id must be nonzero".to_string(),
            ));
        }
        Ok(Self(bytes))
    }

    /// Borrow the raw identity bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; NODE_ID_BYTES] {
        &self.0
    }

    /// True if this is the reserved all-zero identity, which is never valid in a
    /// persisted node-bearing operation.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0 == [0_u8; NODE_ID_BYTES]
    }
}

/// Node kind. FDD-03 §9.3.0 `NodeKind` (`enum_u16`). Used by `DeleteNode`
/// (`old_node_kind`) and state-tree entries (§10.2). `0x0000` is reserved/invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum NodeKind {
    /// Text file node.
    TextFile = 0x0001,
    /// Binary file node.
    BinaryFile = 0x0002,
    /// Symlink node.
    Symlink = 0x0003,
}

impl NodeKind {
    /// Stable code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// Parse a stable code. Rejects `0x0000` (INVALID/reserved) and unknown values.
    pub fn from_code(code: u16) -> Result<Self> {
        match code {
            0x0001 => Ok(Self::TextFile),
            0x0002 => Ok(Self::BinaryFile),
            0x0003 => Ok(Self::Symlink),
            other => Err(PrikkError::MalformedData(format!(
                "unknown or reserved node_kind code: {other:#06x}"
            ))),
        }
    }

    /// Derive a file node's kind from its referenced blob's `blob_kind` (§9.3.0).
    /// A file node MUST NOT reference a `SNAPSHOT` blob.
    pub fn from_file_blob_kind(blob_kind: BlobKind) -> Result<Self> {
        match blob_kind {
            BlobKind::Text => Ok(Self::TextFile),
            BlobKind::Binary => Ok(Self::BinaryFile),
            BlobKind::Snapshot => Err(PrikkError::MalformedData(
                "file node must not reference a SNAPSHOT blob".to_string(),
            )),
        }
    }
}
