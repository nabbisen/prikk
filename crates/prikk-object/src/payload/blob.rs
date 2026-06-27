//! Blob payload type.

use prikk_error::{PrikkError, Result};

use crate::{CanonicalEncode, CanonicalWriter};

/// Blob payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobPayload {
    /// Blob bytes.
    pub bytes: Vec<u8>,
}

impl BlobPayload {
    /// Decode a blob payload from Prikk canonical TLV bytes.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self> {
        let mut cursor = BlobCursor { bytes, pos: 0, last_tag: None };
        let mut blob = None;
        while let Some(field) = cursor.next_field()? {
            match field.tag {
                1 => {
                    field.require_wire(crate::canonical::WireType::Bytes)?;
                    blob = Some(field.value.to_vec());
                }
                other => {
                    return Err(PrikkError::MalformedData(format!(
                        "unknown Blob field tag: {other}"
                    )));
                }
            }
        }
        Ok(Self {
            bytes: blob.ok_or_else(|| PrikkError::MalformedData("Blob missing bytes".to_string()))?,
        })
    }
}

impl CanonicalEncode for BlobPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_bytes(1, &self.bytes)?;
        Ok(())
    }
}

struct BlobCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    last_tag: Option<u16>,
}

impl<'a> BlobCursor<'a> {
    fn next_field(&mut self) -> Result<Option<BlobField<'a>>> {
        if self.pos == self.bytes.len() {
            return Ok(None);
        }
        let tag = u16::from_be_bytes(self.read_array::<2>()?);
        if tag == 0 {
            return Err(PrikkError::MalformedData("field tag 0 is reserved".to_string()));
        }
        if let Some(last) = self.last_tag {
            if tag < last {
                return Err(PrikkError::MalformedData(format!(
                    "field tag order violation: {tag} after {last}"
                )));
            }
        }
        self.last_tag = Some(tag);
        let wire_type = self.read_u8()?;
        let len = usize::try_from(u64::from_be_bytes(self.read_array::<8>()?)).map_err(|_| {
            PrikkError::MalformedData("canonical field length does not fit usize".to_string())
        })?;
        let value = self.read_exact(len)?;
        Ok(Some(BlobField { tag, wire_type, value }))
    }

    fn read_u8(&mut self) -> Result<u8> {
        let value = self.read_exact(1)?;
        let Some(byte) = value.first() else {
            return Err(PrikkError::MalformedData("unexpected empty byte".to_string()));
        };
        Ok(*byte)
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = self.read_exact(N)?;
        let mut out = [0_u8; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| PrikkError::MalformedData("canonical range overflow".to_string()))?;
        let Some(slice) = self.bytes.get(self.pos..end) else {
            return Err(PrikkError::MalformedData(
                "unexpected end of canonical payload".to_string(),
            ));
        };
        self.pos = end;
        Ok(slice)
    }
}

struct BlobField<'a> {
    tag: u16,
    wire_type: u8,
    value: &'a [u8],
}

impl BlobField<'_> {
    fn require_wire(&self, expected: crate::canonical::WireType) -> Result<()> {
        if self.wire_type == expected as u8 {
            return Ok(());
        }
        Err(PrikkError::MalformedData(format!(
            "field {} has wrong wire type: expected {}, got {}",
            self.tag, expected as u8, self.wire_type
        )))
    }
}
