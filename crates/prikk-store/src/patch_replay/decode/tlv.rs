//! Canonical TLV cursor and field reading. Split out of `decode.rs` (DC-58) — no behaviour
//! change, all items moved verbatim.

use prikk_error::{PrikkError, Result};
use prikk_object::{NodeId, NodeKind, ObjectId, TEXT_SPAN_HASH_BYTES, WireType};

pub(super) struct TlvCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    last_tag: Option<u16>,
}

impl<'a> TlvCursor<'a> {
    pub(super) const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            last_tag: None,
        }
    }

    pub(super) fn next_field(&mut self) -> Result<Option<TlvField<'a>>> {
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
        Ok(Some(TlvField {
            tag,
            wire_type,
            value,
        }))
    }

    fn read_u8(&mut self) -> Result<u8> {
        let bytes = self.read_exact(1)?;
        let Some(byte) = bytes.first() else {
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

pub(super) struct TlvField<'a> {
    pub(super) tag: u16,
    wire_type: u8,
    pub(super) value: &'a [u8],
}

impl<'a> TlvField<'a> {
    pub(super) fn read_string(&self) -> Result<String> {
        self.require_wire(WireType::String)?;
        String::from_utf8(self.value.to_vec())
            .map_err(|err| PrikkError::MalformedData(format!("invalid UTF-8 string: {err}")))
    }

    pub(super) fn read_u32(&self) -> Result<u32> {
        self.require_wire(WireType::U32)?;
        Ok(u32::from_be_bytes(self.read_array::<4>()?))
    }

    pub(super) fn read_u16(&self) -> Result<u16> {
        self.require_wire(WireType::EnumU16)?;
        Ok(u16::from_be_bytes(self.read_array::<2>()?))
    }

    /// Read an `object_id` (0x12) field. §9.3 references use the `object_id` value
    /// type (not `bytes`).
    pub(super) fn read_object_id_typed(&self) -> Result<ObjectId> {
        self.require_wire(WireType::ObjectId)?;
        Ok(ObjectId::from_bytes(self.read_array::<32>()?))
    }

    /// Read a `repo_path` (0x13) field as a UTF-8 string. Callers still parse it
    /// through `RepoPath` for path-safety validation.
    pub(super) fn read_repo_path(&self) -> Result<String> {
        self.require_wire(WireType::RepoPath)?;
        String::from_utf8(self.value.to_vec())
            .map_err(|err| PrikkError::MalformedData(format!("invalid UTF-8 repo_path: {err}")))
    }

    /// Read a `bytes` (0x11) field as a validated 32-byte node identity; rejects
    /// the all-zero reserved value via `NodeId::try_from_bytes`.
    pub(super) fn read_node_id(&self) -> Result<NodeId> {
        self.require_wire(WireType::Bytes)?;
        NodeId::try_from_bytes(self.read_array::<32>()?)
    }

    /// Read an `enum_u16` (0x05) field as a `NodeKind`; rejects 0x0000/unknown.
    pub(super) fn read_node_kind(&self) -> Result<NodeKind> {
        self.require_wire(WireType::EnumU16)?;
        NodeKind::from_code(u16::from_be_bytes(self.read_array::<2>()?))
    }

    pub(super) fn read_span_hash(&self) -> Result<[u8; TEXT_SPAN_HASH_BYTES]> {
        self.require_wire(WireType::Bytes)?;
        self.read_array::<TEXT_SPAN_HASH_BYTES>()
    }

    /// Read a variable-length `bytes` (0x11) field.
    pub(super) fn read_bytes_vec(&self) -> Result<Vec<u8>> {
        self.require_wire(WireType::Bytes)?;
        Ok(self.value.to_vec())
    }

    pub(super) fn require_wire(&self, expected: WireType) -> Result<()> {
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
