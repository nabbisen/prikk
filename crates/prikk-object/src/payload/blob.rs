//! Blob payload type.

use prikk_error::{PrikkError, Result};

use crate::{CanonicalEncode, CanonicalWriter};

/// Blob kind. FDD-03 §9.3.0 / §9.11 `blob_kind` (`enum_u16`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum BlobKind {
    /// Text file content.
    Text = 0x0001,
    /// Binary file content.
    Binary = 0x0002,
    /// Full worktree snapshot.
    Snapshot = 0x0003,
}

impl BlobKind {
    /// Stable code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// Parse a stable code. Rejects `0x0000` (INVALID/reserved) and unknown values.
    pub fn from_code(code: u16) -> Result<Self> {
        match code {
            0x0001 => Ok(Self::Text),
            0x0002 => Ok(Self::Binary),
            0x0003 => Ok(Self::Snapshot),
            other => Err(PrikkError::MalformedData(format!(
                "unknown or reserved blob_kind code: {other:#06x}"
            ))),
        }
    }
}

/// Blob payload. FDD-03 §9.11.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobPayload {
    /// Blob kind.
    pub blob_kind: BlobKind,
    /// Blob content bytes.
    pub content: Vec<u8>,
    /// Declared size; must equal `content.len()` in v1.
    pub declared_size: u64,
}

impl BlobPayload {
    /// Construct a blob with `declared_size` set to the content length.
    #[must_use]
    pub fn new(blob_kind: BlobKind, content: Vec<u8>) -> Self {
        let declared_size = content.len() as u64;
        Self {
            blob_kind,
            content,
            declared_size,
        }
    }

    /// Decode a blob payload from Prikk canonical TLV bytes.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self> {
        let mut cursor = BlobCursor {
            bytes,
            pos: 0,
            last_tag: None,
        };
        let mut blob_kind = None;
        let mut content = None;
        let mut declared_size = None;
        while let Some(field) = cursor.next_field()? {
            match field.tag {
                1 => {
                    if blob_kind.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Blob blob_kind field".to_string(),
                        ));
                    }
                    field.require_wire(crate::canonical::WireType::EnumU16)?;
                    blob_kind = Some(BlobKind::from_code(u16::from_be_bytes(
                        field.read_array::<2>()?,
                    ))?);
                }
                2 => {
                    if content.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Blob content field".to_string(),
                        ));
                    }
                    field.require_wire(crate::canonical::WireType::Bytes)?;
                    content = Some(field.value.to_vec());
                }
                3 => {
                    if declared_size.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Blob declared_size field".to_string(),
                        ));
                    }
                    field.require_wire(crate::canonical::WireType::U64)?;
                    declared_size = Some(u64::from_be_bytes(field.read_array::<8>()?));
                }
                other => {
                    return Err(PrikkError::MalformedData(format!(
                        "unknown Blob field tag: {other}"
                    )));
                }
            }
        }
        let blob_kind = blob_kind
            .ok_or_else(|| PrikkError::MalformedData("Blob missing blob_kind".to_string()))?;
        let content =
            content.ok_or_else(|| PrikkError::MalformedData("Blob missing content".to_string()))?;
        let declared_size = declared_size
            .ok_or_else(|| PrikkError::MalformedData("Blob missing declared_size".to_string()))?;
        if declared_size != content.len() as u64 {
            return Err(PrikkError::MalformedData(format!(
                "Blob declared_size {declared_size} does not match content length {}",
                content.len()
            )));
        }
        Ok(Self {
            blob_kind,
            content,
            declared_size,
        })
    }
}

impl CanonicalEncode for BlobPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        if self.declared_size != self.content.len() as u64 {
            return Err(PrikkError::CanonicalEncoding(format!(
                "Blob declared_size {} does not match content length {}",
                self.declared_size,
                self.content.len()
            )));
        }
        writer.field_enum_u16(1, self.blob_kind.code())?;
        writer.field_bytes(2, &self.content)?;
        writer.field_u64(3, self.declared_size)?;
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
        Ok(Some(BlobField {
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

struct BlobField<'a> {
    tag: u16,
    wire_type: u8,
    value: &'a [u8],
}

impl BlobField<'_> {
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
