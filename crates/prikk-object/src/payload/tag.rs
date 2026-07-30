//! Tag payload type.

use prikk_error::{PrikkError, Result};

use crate::canonical::WireType;
use crate::{CanonicalEncode, CanonicalWriter, ObjectId};

/// Immutable tag payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagPayload {
    /// Tag name.
    pub name: String,
    /// Target block ID.
    pub target_block_id: ObjectId,
    /// Tag message (optional per FDD-03 §9.8).
    pub message: Option<String>,
    /// Canonical no-clock sentinel, matching `RefUpdatePayload.created_at` (DC-34 "RefUpdate time
    /// policy"): zero in every production write, never an authoritative event-time claim. This
    /// project has no trusted clock; a real timestamp would require a versioned schema and a
    /// persistence design.
    pub created_at: u64,
    /// Author key ID.
    pub author_key_id: String,
}

impl CanonicalEncode for TagPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.name)?;
        writer.field_object_id(2, &self.target_block_id)?;
        if let Some(message) = &self.message {
            writer.field_string(3, message)?;
        }
        writer.field_u64(4, self.created_at)?;
        writer.field_string(5, &self.author_key_id)?;
        Ok(())
    }
}

impl TagPayload {
    /// Decode a Tag payload from Prikk canonical TLV bytes.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self> {
        let mut cursor = TagCursor::new(bytes);
        let mut name = None;
        let mut target_block_id = None;
        let mut message = None;
        let mut created_at = None;
        let mut author_key_id = None;
        while let Some(field) = cursor.next_field()? {
            match field.tag {
                1 => name = Some(field.read_string()?),
                2 => target_block_id = Some(field.read_object_id()?),
                3 => message = Some(field.read_string()?),
                4 => created_at = Some(field.read_u64()?),
                5 => author_key_id = Some(field.read_string()?),
                other => {
                    return Err(PrikkError::MalformedData(format!(
                        "unknown Tag field tag: {other}"
                    )));
                }
            }
        }
        Ok(Self {
            name: name.ok_or_else(|| PrikkError::MalformedData("Tag missing name".to_string()))?,
            target_block_id: target_block_id.ok_or_else(|| {
                PrikkError::MalformedData("Tag missing target_block_id".to_string())
            })?,
            message,
            created_at: created_at
                .ok_or_else(|| PrikkError::MalformedData("Tag missing created_at".to_string()))?,
            author_key_id: author_key_id.ok_or_else(|| {
                PrikkError::MalformedData("Tag missing author_key_id".to_string())
            })?,
        })
    }
}

struct TagCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    last_tag: Option<u16>,
}

impl<'a> TagCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            last_tag: None,
        }
    }

    fn next_field(&mut self) -> Result<Option<TagField<'a>>> {
        if self.pos == self.bytes.len() {
            return Ok(None);
        }
        let tag = u16::from_be_bytes(self.read_array::<2>()?);
        if tag == 0 {
            return Err(PrikkError::MalformedData(
                "field tag 0 is reserved".to_string(),
            ));
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
        Ok(Some(TagField {
            tag,
            wire_type,
            value,
        }))
    }

    fn read_u8(&mut self) -> Result<u8> {
        let value = self.read_exact(1)?;
        let Some(byte) = value.first() else {
            return Err(PrikkError::MalformedData(
                "unexpected empty byte".to_string(),
            ));
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

struct TagField<'a> {
    tag: u16,
    wire_type: u8,
    value: &'a [u8],
}

impl<'a> TagField<'a> {
    fn read_string(&self) -> Result<String> {
        self.require_wire(WireType::String)?;
        String::from_utf8(self.value.to_vec())
            .map_err(|err| PrikkError::MalformedData(format!("invalid UTF-8 string: {err}")))
    }

    fn read_u64(&self) -> Result<u64> {
        self.require_wire(WireType::U64)?;
        Ok(u64::from_be_bytes(self.read_array::<8>()?))
    }

    fn read_object_id(&self) -> Result<ObjectId> {
        self.require_wire(WireType::ObjectId)?;
        Ok(ObjectId::from_bytes(self.read_array::<32>()?))
    }

    fn require_wire(&self, expected: WireType) -> Result<()> {
        if self.wire_type == expected as u8 {
            return Ok(());
        }
        Err(PrikkError::MalformedData(format!(
            "field {} has wrong wire type: expected {}, got {}",
            self.tag, expected as u8, self.wire_type
        )))
    }

    fn read_array<const N: usize>(&self) -> Result<[u8; N]> {
        if self.value.len() != N {
            return Err(PrikkError::MalformedData(format!(
                "field {} expected {N} bytes, got {}",
                self.tag,
                self.value.len()
            )));
        }
        let mut out = [0_u8; N];
        out.copy_from_slice(self.value);
        Ok(out)
    }
}
