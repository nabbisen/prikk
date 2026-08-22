//! Shared payload helper types.

use prikk_error::Result;

use crate::{CanonicalEncode, CanonicalWriter};

/// A 32-byte Merkle root for a materialized tree state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MerkleRoot(pub [u8; 32]);

/// A 32-byte digest over a canonicalized set of patch ids (RFC 115 Stage 1 design D4). Not
/// persisted as its own object -- a pure comparison value, `MerkleRoot`'s own shape. Lives here
/// rather than in `prikk-store` (where every value that *computes* one still does,
/// `compute_patch_set_digest` and friends) because `TagPayload` (RFC 117 T1) carries one, and
/// `prikk-object` cannot depend on `prikk-store` -- the same crate-boundary reason
/// `state_root.rs`'s `compute_state_root` stays in `prikk-store` while `MerkleRoot` itself lives
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PatchSetDigest(pub [u8; 32]);

/// Advisory patch intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum Intent {
    /// Feature work.
    Feature = 1,
    /// Bug fix.
    Fix = 2,
    /// Refactoring.
    Refactor = 3,
    /// Documentation.
    Docs = 4,
    /// Test-only change.
    Test = 5,
}

impl Intent {
    /// Stable numeric code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }
}

/// Operation-level condition entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperationConditionEntry {
    /// Condition key.
    pub key: String,
    /// Condition value.
    pub value: OperationCondition,
}

impl CanonicalEncode for OperationConditionEntry {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.key)?;
        writer.field_record(2, &self.value)?;
        Ok(())
    }
}

/// Operation precondition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationCondition {
    /// Old content hash must match before the operation applies.
    OldContentHash(Vec<u8>),
    /// Named anchor must exist.
    AnchorExists(String),
    /// Path must exist.
    PathExists(String),
    /// Path must be absent.
    PathAbsent(String),
}

impl CanonicalEncode for OperationCondition {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        match self {
            Self::OldContentHash(hash) => {
                writer.field_u32(1, 1)?;
                writer.field_bytes(2, hash)?;
            }
            Self::AnchorExists(anchor) => {
                writer.field_u32(1, 2)?;
                writer.field_string(2, anchor)?;
            }
            Self::PathExists(path) => {
                writer.field_u32(1, 3)?;
                writer.field_string(2, path)?;
            }
            Self::PathAbsent(path) => {
                writer.field_u32(1, 4)?;
                writer.field_string(2, path)?;
            }
        }
        Ok(())
    }
}
