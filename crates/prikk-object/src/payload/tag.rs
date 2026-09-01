//! Tag payload type.
//!
//! **RFC 117 T1 (2026-08-22, owner ruling -- "No project has been created in production in the
//! world yet. Breaking change is accepted."): `patch_set_digest` is amended in at `schema_version`
//! 1, in place, required.** A tag is therefore a local pointer plus a global identity:
//! `target_block_id` names a block this repository can resolve locally; `patch_set_digest` is the
//! digest of that block's own patch closure (`compute_patch_set_digest_from_block`,
//! `prikk-store/src/patch_set_digest.rs`) and is what travels between repositories -- **two
//! repositories holding the same patches produce the same `patch_set_digest`, by construction**,
//! which is the property a tag's portability depends on. There is no schema 2: every Tag object
//! written before this change stops decoding (`rfc114_vector_11` moved to record this deliberately;
//! `empty_tag` did not, since it is generated from a literal empty payload independent of this
//! struct).
//!
//! **RFC 117 T7 (2026-08-22, owner ruling "Take it now", after stage 2 measured resolution at
//! O(N²)): `patch_count` is field 7, also required.** The number of distinct patch ids in the
//! closure `patch_set_digest` covers -- not new information, since `patch_set_digest_preimage`
//! already hashes `DOMAIN ‖ count ‖ sorted ids`; field 7 exposes a fact the digest already commits
//! to, as a cheap integer `resolve_patch_set_digest` (`prikk-store`) can compare before hashing a
//! candidate, instead of after. **The count is a hint that prunes, never an authority (design §9.4):
//! a wrong `patch_count` can only cause the right candidate to be skipped or extra candidates to be
//! hashed -- it can never produce a wrong resolution, because the digest still has to match.** The
//! same tried-not-trusted shape D6 §11.6 already established for a different object.

use prikk_error::{PrikkError, Result};

use crate::canonical::WireType;
use crate::payload::common::PatchSetDigest;
use crate::{CanonicalEncode, CanonicalWriter, ObjectId};

/// Immutable tag payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagPayload {
    /// Tag name.
    pub name: String,
    /// Target block ID -- the local pointer half of the tag's identity.
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
    /// The digest of `target_block_id`'s own patch closure (RFC 117 T1) -- the global-identity half
    /// of the tag, portable across repositories that hold the same patches. Required: a tag without
    /// one would be the old tag with extra steps.
    pub patch_set_digest: PatchSetDigest,
    /// The number of distinct patch ids in the closure `patch_set_digest` covers (RFC 117 T7) --
    /// already part of what the digest hashes, exposed here so resolution can prune by size before
    /// hashing a candidate. A hint that narrows, never an authority: the digest still decides.
    pub patch_count: u64,
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
        writer.field_bytes(6, &self.patch_set_digest.0)?;
        writer.field_u64(7, self.patch_count)?;
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
        let mut patch_set_digest = None;
        let mut patch_count = None;
        while let Some(field) = cursor.next_field()? {
            match field.tag {
                1 => {
                    if name.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Tag name field".to_string(),
                        ));
                    }
                    name = Some(field.read_string()?);
                }
                2 => {
                    if target_block_id.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Tag target_block_id field".to_string(),
                        ));
                    }
                    target_block_id = Some(field.read_object_id()?);
                }
                3 => {
                    if message.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Tag message field".to_string(),
                        ));
                    }
                    message = Some(field.read_string()?);
                }
                4 => {
                    if created_at.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Tag created_at field".to_string(),
                        ));
                    }
                    created_at = Some(field.read_u64()?);
                }
                5 => {
                    if author_key_id.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Tag author_key_id field".to_string(),
                        ));
                    }
                    author_key_id = Some(field.read_string()?);
                }
                6 => {
                    if patch_set_digest.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Tag patch_set_digest field".to_string(),
                        ));
                    }
                    patch_set_digest = Some(PatchSetDigest(field.read_array::<32>()?));
                }
                7 => {
                    if patch_count.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate Tag patch_count field".to_string(),
                        ));
                    }
                    patch_count = Some(field.read_u64()?);
                }
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
            patch_set_digest: patch_set_digest.ok_or_else(|| {
                PrikkError::MalformedData("Tag missing patch_set_digest".to_string())
            })?,
            patch_count: patch_count
                .ok_or_else(|| PrikkError::MalformedData("Tag missing patch_count".to_string()))?,
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
