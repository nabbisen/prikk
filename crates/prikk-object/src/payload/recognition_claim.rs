//! Recognition-claim payload type.
//!
//! RFC 115 Stage 2 (design-v1.md D3): a signed claim that named patches were sealed into a named
//! block, under the signer's key. Deliberately minimal -- every field omitted from the obvious
//! shape is omitted for a stated reason (implementation handoff, `stage-2-recognition-claim-
//! handoff-v1.md` §2.2): no claimer `key_id` field (the signature preimage already binds `key_id`,
//! `signature.rs`'s `signed_bytes`; duplicating it would create a second source of truth, and two
//! senders making the same claim then produce the same object id, carrying two signatures on one
//! object rather than two near-identical objects); no timestamp (this project has no trusted
//! clock); no `project_id`/genesis binding (block and patch ids are already content-addressed and
//! globally unique, so a claim is meaningless where its ids do not exist).
//!
//! **Never existence-checked against the block or patches it names.** That is the entire reason
//! this is a claim object and not a Block: a claim is verifiable with none of its referenced
//! objects present. See `prikk-store`'s recognition-claim consistency check for what a receiver
//! that *does* hold the referenced block must additionally verify (a claim contradicting a held
//! block is a detected lie, refused loudly) -- that check does not live here, because this payload
//! type has no access to an object store and must not gain one.
//!
//! **`patch_ids` carries the block's own order verbatim (design-v1.md §11, D6).** Amended in
//! `schema_version` 1, in place -- no release has ever shipped this object, so RFC 114's "every
//! object any prior release wrote" promise is not engaged, and this is the last moment the change
//! is free. `Block.patch_ids` has no sorted-or-unique invariant; it is a free sequence consumed in
//! order by `apply_candidate_patches`, and the claim mirrors it exactly -- not sorted, not
//! deduplicated. Order moved out of unsigned artifact metadata and into the claim payload,
//! therefore inside the object id and the signature preimage, at exactly the increment where order
//! starts being acted upon (Stage 4). See `prikk-store`'s recognition-claim consistency check for
//! why sequence equality, not set equality, is what detects an order-lie now that order is
//! load-bearing.
//!
//! **`parent_block_ids` carries the block's own parents verbatim (RFC 116 design-v1.md §3, N3).**
//! Amended in `schema_version` 1, in place, for the same reason and by the same argument as D6:
//! still no release has ever shipped this object, so this is again a free amendment and again
//! zero frozen bytes move (an empty repeated field writes nothing). Sealing one claim per call
//! (RFC 115 Stage 4) leaves no way to order two claims spanning a multi-block delta without this
//! field -- the topological sort RFC 116's negotiation stage performs over a claim batch consumes
//! it. `BlockPayload` imposes no order or uniqueness invariant on `parent_block_ids`, and the
//! claim mirrors it exactly -- not sorted, not deduplicated, and **may be empty** (a root block has
//! no parents; that is the correct, common case, not a degenerate one -- no non-empty guard).

use prikk_error::{PrikkError, Result};

use crate::canonical::WireType;
use crate::{CanonicalEncode, CanonicalWriter, ObjectId};

/// DC-86 bound on `patch_ids`'/`parent_block_ids`' length, matching
/// `DEFAULT_BUNDLE_MAX_OBJECT_COUNT` (`prikk-store/src/bundle.rs`, 100_000). This wire shape has
/// no declared-count prefix to check before allocating (unlike the bundle format) -- each repeated
/// field is its own TLV record, so the bound is enforced the same way: refused the moment the
/// count would exceed the limit, before the over-limit entry is even read, not after decoding
/// everything and counting.
pub const RECOGNITION_CLAIM_MAX_PATCH_IDS: usize = 100_000;

/// A signed claim that `patch_ids` were sealed into `block_id`, on top of `parent_block_ids`,
/// under the signer's key. Never trust-conferring on its own -- see the module doc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecognitionClaimPayload {
    /// The block this claim is about.
    pub block_id: ObjectId,
    /// Patch ids claimed to have been sealed into `block_id`, in the block's own verbatim order
    /// (design-v1.md §11, D6) -- not sorted, not deduplicated, non-empty. A claim about no patches
    /// asserts nothing and is a decode error, not a degenerate value.
    pub patch_ids: Vec<ObjectId>,
    /// `block_id`'s own `parent_block_ids`, verbatim (RFC 116 design-v1.md §3, N3) -- not sorted,
    /// not deduplicated, and may be empty (a root block has no parents).
    pub parent_block_ids: Vec<ObjectId>,
}

impl CanonicalEncode for RecognitionClaimPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        if self.patch_ids.is_empty() {
            return Err(PrikkError::CanonicalEncoding(
                "RecognitionClaim patch_ids must not be empty".to_string(),
            ));
        }
        writer.field_object_id(1, &self.block_id)?;
        writer.repeated_object_id(2, &self.patch_ids)?;
        writer.repeated_object_id(3, &self.parent_block_ids)?;
        Ok(())
    }
}

impl RecognitionClaimPayload {
    /// Decode a RecognitionClaim payload from Prikk canonical TLV bytes. `patch_ids` and
    /// `parent_block_ids` both decode in wire order, unsorted and with duplicates preserved
    /// (design-v1.md §11 D6; RFC 116 design-v1.md §3 N3) -- each is the block's own verbatim
    /// sequence, not a set.
    pub fn decode_canonical(bytes: &[u8]) -> Result<Self> {
        let mut cursor = RecognitionClaimCursor::new(bytes);
        let mut block_id = None;
        let mut patch_ids = Vec::new();
        let mut parent_block_ids = Vec::new();
        while let Some(field) = cursor.next_field()? {
            match field.tag {
                1 => {
                    if block_id.is_some() {
                        return Err(PrikkError::MalformedData(
                            "duplicate RecognitionClaim block_id field".to_string(),
                        ));
                    }
                    block_id = Some(field.read_object_id()?);
                }
                2 => {
                    if patch_ids.len() >= RECOGNITION_CLAIM_MAX_PATCH_IDS {
                        return Err(PrikkError::MalformedData(format!(
                            "RecognitionClaim patch_ids exceeds the limit of \
                             {RECOGNITION_CLAIM_MAX_PATCH_IDS}"
                        )));
                    }
                    patch_ids.push(field.read_object_id()?);
                }
                3 => {
                    if parent_block_ids.len() >= RECOGNITION_CLAIM_MAX_PATCH_IDS {
                        return Err(PrikkError::MalformedData(format!(
                            "RecognitionClaim parent_block_ids exceeds the limit of \
                             {RECOGNITION_CLAIM_MAX_PATCH_IDS}"
                        )));
                    }
                    parent_block_ids.push(field.read_object_id()?);
                }
                other => {
                    return Err(PrikkError::MalformedData(format!(
                        "unknown RecognitionClaim field tag: {other}"
                    )));
                }
            }
        }
        let payload = Self {
            block_id: block_id.ok_or_else(|| {
                PrikkError::MalformedData("RecognitionClaim missing block_id".to_string())
            })?,
            patch_ids,
            parent_block_ids,
        };
        if payload.patch_ids.is_empty() {
            return Err(PrikkError::MalformedData(
                "RecognitionClaim patch_ids must not be empty".to_string(),
            ));
        }
        Ok(payload)
    }
}

struct RecognitionClaimCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    last_tag: Option<u16>,
}

impl<'a> RecognitionClaimCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            last_tag: None,
        }
    }

    fn next_field(&mut self) -> Result<Option<RecognitionClaimField<'a>>> {
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
        Ok(Some(RecognitionClaimField {
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

struct RecognitionClaimField<'a> {
    tag: u16,
    wire_type: u8,
    value: &'a [u8],
}

impl<'a> RecognitionClaimField<'a> {
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
