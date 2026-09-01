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

    /// Parse a stable block-kind code.
    pub fn from_code(code: u32) -> Result<Self> {
        match code {
            1 => Ok(Self::Root),
            2 => Ok(Self::Normal),
            3 => Ok(Self::Merge),
            4 => Ok(Self::Repair),
            5 => Ok(Self::Import),
            other => Err(PrikkError::MalformedData(format!(
                "unknown block kind code: {other}"
            ))),
        }
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
    /// The parent state derivation and replay follow. Present only on `Merge` blocks (DC-75); must
    /// name one of `parent_block_ids`. `None` for every other kind, which have at most one parent
    /// already and need no designation.
    pub mainline_parent_id: Option<ObjectId>,
    /// The block confluence was proven against when this `Merge` was sealed (DC-75). A claim, not a
    /// trust boundary: `verify` independently re-derives the true merge base and reports disagreement
    /// rather than trusting this field. `None` for every other kind.
    pub merge_baseline_block_id: Option<ObjectId>,
}

impl BlockPayload {
    /// Decode a block payload from Prikk canonical TLV bytes.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self> {
        let mut cursor = BlockCanonicalCursor::new(bytes);
        let mut parent_block_ids = Vec::new();
        let mut kind = None;
        let mut patch_ids = Vec::new();
        let mut state_merkle_root = None;
        let mut snapshot_blob_ref = None;
        let mut mainline_parent_id = None;
        let mut merge_baseline_block_id = None;
        while let Some(field) = cursor.next_field()? {
            match field.tag {
                1 => parent_block_ids.push(field.read_object_id()?),
                2 => {
                    if kind.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Block kind field".to_string(),
                        ));
                    }
                    kind = Some(BlockKind::from_code(u32::from(field.read_enum_u16()?))?);
                }
                3 => patch_ids.push(field.read_object_id()?),
                4 => {
                    if state_merkle_root.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Block state_merkle_root field".to_string(),
                        ));
                    }
                    state_merkle_root = Some(MerkleRoot(field.read_array::<32>()?));
                }
                5 => {
                    if snapshot_blob_ref.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Block snapshot_blob_ref field".to_string(),
                        ));
                    }
                    snapshot_blob_ref = Some(field.read_object_id()?);
                }
                6 => {
                    if mainline_parent_id.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Block mainline_parent_id field".to_string(),
                        ));
                    }
                    mainline_parent_id = Some(field.read_object_id()?);
                }
                7 => {
                    if merge_baseline_block_id.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Block merge_baseline_block_id field".to_string(),
                        ));
                    }
                    merge_baseline_block_id = Some(field.read_object_id()?);
                }
                other => {
                    return Err(PrikkError::MalformedData(format!(
                        "unknown Block field tag: {other}"
                    )));
                }
            }
        }
        let payload = Self {
            parent_block_ids,
            kind: kind
                .ok_or_else(|| PrikkError::MalformedData("Block missing kind".to_string()))?,
            patch_ids,
            state_merkle_root: state_merkle_root.ok_or_else(|| {
                PrikkError::MalformedData("Block missing state_merkle_root".to_string())
            })?,
            snapshot_blob_ref,
            mainline_parent_id,
            merge_baseline_block_id,
        };
        if !is_strictly_sorted(&payload.parent_block_ids) {
            return Err(PrikkError::MalformedData(
                "Block parent IDs are not sorted and unique".to_string(),
            ));
        }
        Ok(payload)
    }
}

struct BlockCanonicalCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    last_tag: Option<u16>,
}

impl<'a> BlockCanonicalCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            last_tag: None,
        }
    }

    fn next_field(&mut self) -> Result<Option<BlockCanonicalField<'a>>> {
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
        Ok(Some(BlockCanonicalField {
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

struct BlockCanonicalField<'a> {
    tag: u16,
    wire_type: u8,
    value: &'a [u8],
}

impl<'a> BlockCanonicalField<'a> {
    fn read_object_id(&self) -> Result<ObjectId> {
        self.require_wire(crate::canonical::WireType::ObjectId)?;
        Ok(ObjectId::from_bytes(self.read_array::<32>()?))
    }

    fn read_enum_u16(&self) -> Result<u16> {
        self.require_wire(crate::canonical::WireType::EnumU16)?;
        Ok(u16::from_be_bytes(self.read_array::<2>()?))
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

impl CanonicalEncode for BlockPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        if !is_strictly_sorted(&self.parent_block_ids) {
            return Err(PrikkError::CanonicalEncoding(
                "parent_block_ids must be sorted and unique".to_string(),
            ));
        }
        writer.repeated_object_id(1, &self.parent_block_ids)?;
        writer.field_enum_u16(2, self.kind.code())?;
        writer.repeated_object_id(3, &self.patch_ids)?;
        writer.field_bytes(4, &self.state_merkle_root.0)?;
        if let Some(snapshot) = self.snapshot_blob_ref {
            writer.field_object_id(5, &snapshot)?;
        }
        if let Some(mainline) = self.mainline_parent_id {
            writer.field_object_id(6, &mainline)?;
        }
        if let Some(baseline) = self.merge_baseline_block_id {
            writer.field_object_id(7, &baseline)?;
        }
        Ok(())
    }
}
