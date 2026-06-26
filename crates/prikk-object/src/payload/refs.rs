//! Reference payload types.

use prikk_error::{PrikkError, Result};

use crate::canonical::{is_strictly_sorted, WireType};
use crate::{CanonicalEncode, CanonicalWriter, ObjectId};

/// Ref kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum RefKind {
    /// Branch ref.
    Branch = 1,
    /// Tag ref.
    Tag = 2,
}

impl RefKind {
    /// Stable code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// Parse a stable code.
    pub fn from_code(code: u32) -> Result<Self> {
        match code {
            1 => Ok(Self::Branch),
            2 => Ok(Self::Tag),
            other => Err(PrikkError::MalformedData(format!(
                "unknown ref kind code: {other}"
            ))),
        }
    }
}

/// RefState payload stored as a content-addressed object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefStatePayload {
    /// Human-readable ref name.
    pub ref_name: String,
    /// Ref kind.
    pub kind: RefKind,
    /// Target object ID.
    pub target_object_id: ObjectId,
    /// Monotonic sequence number.
    pub update_seq: u64,
    /// Previous ref-state object ID.
    pub previous_ref_state_id: Option<ObjectId>,
    /// Required attestation IDs that justified this state.
    pub required_attestation_ids: Vec<ObjectId>,
}

impl RefStatePayload {
    /// Decode a RefState payload from PRIKK canonical TLV bytes.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self> {
        let mut cursor = CanonicalCursor::new(bytes);
        let mut ref_name = None;
        let mut kind = None;
        let mut target_object_id = None;
        let mut update_seq = None;
        let mut previous_ref_state_id = None;
        let mut required_attestation_ids = Vec::new();
        while let Some(field) = cursor.next_field()? {
            match field.tag {
                1 => ref_name = Some(field.read_string()?),
                2 => kind = Some(RefKind::from_code(field.read_u32()?)?),
                3 => target_object_id = Some(field.read_object_id()?),
                4 => update_seq = Some(field.read_u64()?),
                5 => previous_ref_state_id = Some(field.read_object_id()?),
                6 => required_attestation_ids.push(field.read_object_id()?),
                other => {
                    return Err(PrikkError::MalformedData(format!(
                        "unknown RefState field tag: {other}"
                    )));
                }
            }
        }
        let payload = Self {
            ref_name: ref_name.ok_or_else(|| {
                PrikkError::MalformedData("RefState missing ref_name".to_string())
            })?,
            kind: kind
                .ok_or_else(|| PrikkError::MalformedData("RefState missing kind".to_string()))?,
            target_object_id: target_object_id.ok_or_else(|| {
                PrikkError::MalformedData("RefState missing target_object_id".to_string())
            })?,
            update_seq: update_seq.ok_or_else(|| {
                PrikkError::MalformedData("RefState missing update_seq".to_string())
            })?,
            previous_ref_state_id,
            required_attestation_ids,
        };
        if !is_strictly_sorted(&payload.required_attestation_ids) {
            return Err(PrikkError::MalformedData(
                "RefState attestation IDs are not sorted and unique".to_string(),
            ));
        }
        Ok(payload)
    }
}

impl CanonicalEncode for RefStatePayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        if !is_strictly_sorted(&self.required_attestation_ids) {
            return Err(PrikkError::CanonicalEncoding(
                "required_attestation_ids must be sorted and unique".to_string(),
            ));
        }
        writer.field_string(1, &self.ref_name)?;
        writer.field_u32(2, u32::from(self.kind.code()))?;
        writer.field_bytes(3, self.target_object_id.as_bytes())?;
        writer.field_u64(4, self.update_seq)?;
        if let Some(previous) = self.previous_ref_state_id {
            writer.field_bytes(5, previous.as_bytes())?;
        }
        writer.repeated_object_id(6, &self.required_attestation_ids)?;
        Ok(())
    }
}

/// Ref-update event payload stored inline in ref logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefUpdatePayload {
    /// Ref name.
    pub ref_name: String,
    /// Previous RefState ID.
    pub old_ref_state_id: Option<ObjectId>,
    /// New RefState ID.
    pub new_ref_state_id: ObjectId,
    /// New target object ID.
    pub new_target_object_id: ObjectId,
    /// Update sequence.
    pub update_seq: u64,
    /// Authoritative event creation timestamp.
    pub created_at: u64,
    /// Author key ID.
    pub author_key_id: String,
}

impl CanonicalEncode for RefUpdatePayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.ref_name)?;
        if let Some(old) = self.old_ref_state_id {
            writer.field_bytes(2, old.as_bytes())?;
        }
        writer.field_bytes(3, self.new_ref_state_id.as_bytes())?;
        writer.field_bytes(4, self.new_target_object_id.as_bytes())?;
        writer.field_u64(5, self.update_seq)?;
        writer.field_u64(6, self.created_at)?;
        writer.field_string(7, &self.author_key_id)?;
        Ok(())
    }
}

struct CanonicalCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    last_tag: Option<u16>,
}

impl<'a> CanonicalCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0, last_tag: None }
    }

    fn next_field(&mut self) -> Result<Option<CanonicalField<'a>>> {
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
        Ok(Some(CanonicalField { tag, wire_type, value }))
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

struct CanonicalField<'a> {
    tag: u16,
    wire_type: u8,
    value: &'a [u8],
}

impl<'a> CanonicalField<'a> {
    fn read_string(&self) -> Result<String> {
        self.require_wire(WireType::String)?;
        String::from_utf8(self.value.to_vec())
            .map_err(|err| PrikkError::MalformedData(format!("invalid UTF-8 string: {err}")))
    }

    fn read_u32(&self) -> Result<u32> {
        self.require_wire(WireType::U32)?;
        Ok(u32::from_be_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&self) -> Result<u64> {
        self.require_wire(WireType::U64)?;
        Ok(u64::from_be_bytes(self.read_array::<8>()?))
    }

    fn read_object_id(&self) -> Result<ObjectId> {
        self.require_wire(WireType::Bytes)?;
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
