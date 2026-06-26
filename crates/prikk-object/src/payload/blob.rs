//! Blob payload type.

use prikk_error::Result;

use crate::{CanonicalEncode, CanonicalWriter};

/// Blob payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobPayload {
    /// Blob bytes.
    pub bytes: Vec<u8>,
}

impl CanonicalEncode for BlobPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_bytes(1, &self.bytes)?;
        Ok(())
    }
}
