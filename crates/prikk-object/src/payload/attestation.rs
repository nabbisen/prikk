//! Attestation payload types.

use prikk_error::{PrikkError, Result};

use crate::canonical::is_strictly_sorted;
use crate::{CanonicalEncode, CanonicalWriter, ObjectId};

/// Attestation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum AttestationStatus {
    /// Passed policy.
    Pass = 1,
    /// Warning.
    Warn = 2,
    /// Failed policy.
    Fail = 3,
    /// Locally quarantined.
    Quarantine = 4,
}

impl AttestationStatus {
    /// Stable code.
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }
}

/// Plugin result entry, sorted by plugin ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PluginResultEntry {
    /// Plugin ID.
    pub plugin_id: String,
    /// Status.
    pub status: AttestationStatus,
    /// Report object ID.
    pub report_blob_id: Option<ObjectId>,
}

impl CanonicalEncode for PluginResultEntry {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.plugin_id)?;
        writer.field_u32(2, u32::from(self.status.code()))?;
        if let Some(report) = self.report_blob_id {
            writer.field_bytes(3, report.as_bytes())?;
        }
        Ok(())
    }
}

/// Attestation payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationPayload {
    /// Target block ID.
    pub target_block_id: ObjectId,
    /// Policy version string.
    pub policy_version: String,
    /// Policy hash.
    pub policy_hash: Vec<u8>,
    /// Plugin-set hash.
    pub plugin_set_hash: Vec<u8>,
    /// Results sorted by plugin ID.
    pub results: Vec<PluginResultEntry>,
    /// Overall status.
    pub status: AttestationStatus,
    /// Authoritative attestation creation timestamp.
    pub created_at: u64,
    /// True if this result can be reproduced offline from stored inputs.
    pub is_reproducible_offline: bool,
}

impl CanonicalEncode for AttestationPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        if !is_strictly_sorted(&self.results) {
            return Err(PrikkError::CanonicalEncoding(
                "plugin results must be sorted and unique by plugin_id".to_string(),
            ));
        }
        writer.field_bytes(1, self.target_block_id.as_bytes())?;
        writer.field_string(2, &self.policy_version)?;
        writer.field_bytes(3, &self.policy_hash)?;
        writer.field_bytes(4, &self.plugin_set_hash)?;
        writer.repeated_record(5, &self.results)?;
        writer.field_u32(6, u32::from(self.status.code()))?;
        writer.field_u64(7, self.created_at)?;
        writer.field_bool(8, self.is_reproducible_offline)?;
        Ok(())
    }
}
