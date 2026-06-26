//! Tag payload type.

use prikk_error::Result;

use crate::{CanonicalEncode, CanonicalWriter, ObjectId};

/// Immutable tag payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagPayload {
    /// Tag name.
    pub name: String,
    /// Target block ID.
    pub target_block_id: ObjectId,
    /// Tag message.
    pub message: String,
    /// Authoritative creation timestamp.
    pub created_at: u64,
    /// Author key ID.
    pub author_key_id: String,
}

impl CanonicalEncode for TagPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.name)?;
        writer.field_bytes(2, self.target_block_id.as_bytes())?;
        writer.field_string(3, &self.message)?;
        writer.field_u64(4, self.created_at)?;
        writer.field_string(5, &self.author_key_id)?;
        Ok(())
    }
}
