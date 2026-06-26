//! Block payload types.

use prikk_error::{PrikkError, Result};

use crate::canonical::is_strictly_sorted;
use crate::payload::common::MerkleRoot;
use crate::{CanonicalEncode, CanonicalWriter, ObjectId};

/// Block kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum BlockKind {
    /// Root block.
    Root = 1,
    /// Normal block.
    Normal = 2,
    /// Merge block.
    Merge = 3,
    /// Repair block.
    Repair = 4,
    /// Import block.
    Import = 5,
}

impl BlockKind {
    /// Stable code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }
}

/// Block payload. Block summaries are intentionally not identity-bearing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockPayload {
    /// Parent block IDs, sorted unless a later design adds semantic parent roles.
    pub parent_block_ids: Vec<ObjectId>,
    /// Block kind.
    pub kind: BlockKind,
    /// Patch IDs in canonical block patch order.
    pub patch_ids: Vec<ObjectId>,
    /// State Merkle root.
    pub state_merkle_root: MerkleRoot,
    /// Optional full snapshot blob reference.
    pub snapshot_blob_ref: Option<ObjectId>,
}

impl CanonicalEncode for BlockPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        if !is_strictly_sorted(&self.parent_block_ids) {
            return Err(PrikkError::CanonicalEncoding(
                "parent_block_ids must be sorted and unique".to_string(),
            ));
        }
        writer.repeated_object_id(1, &self.parent_block_ids)?;
        writer.field_u32(2, u32::from(self.kind.code()))?;
        writer.repeated_object_id(3, &self.patch_ids)?;
        writer.field_bytes(4, &self.state_merkle_root.0)?;
        if let Some(snapshot) = self.snapshot_blob_ref {
            writer.field_bytes(5, snapshot.as_bytes())?;
        }
        Ok(())
    }
}
