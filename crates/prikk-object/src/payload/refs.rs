//! Reference payload types.

use prikk_error::{PrikkError, Result};

use crate::canonical::is_strictly_sorted;
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
