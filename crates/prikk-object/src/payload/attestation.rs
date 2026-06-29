//! Attestation payload types.

use prikk_error::{PrikkError, Result};

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

/// Plugin result entry, sorted by plugin ID. FDD-03 §9.10. The canonical sort key
/// is `plugin_id` only (see `results_sorted_by_plugin_id`), so a full-record
/// ordering is deliberately not derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginResultEntry {
    /// Plugin ID.
    pub plugin_id: String,
    /// Plugin version.
    pub plugin_version: String,
    /// Status.
    pub status: AttestationStatus,
    /// Report hash (not an object id).
    pub report_hash: Vec<u8>,
    /// Number of findings reported.
    pub finding_count: u32,
}

impl CanonicalEncode for PluginResultEntry {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_string(1, &self.plugin_id)?;
        writer.field_string(2, &self.plugin_version)?;
        writer.field_enum_u16(3, self.status.code())?;
        writer.field_bytes(4, &self.report_hash)?;
        writer.field_u32(5, self.finding_count)?;
        Ok(())
    }
}

/// Attestation payload. FDD-03 §9.9.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationPayload {
    /// Target block ID.
    pub target_block_id: ObjectId,
    /// Policy version string.
    pub policy_version: String,
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

/// FDD-03 §9.9 sort key for `results`: strictly ascending by `plugin_id` UTF-8
/// bytes. Strictness also forbids duplicate `plugin_id` values. No secondary key is
/// used in v1, so later fields never participate in canonical ordering.
fn results_sorted_by_plugin_id(results: &[PluginResultEntry]) -> bool {
    results.windows(2).all(|pair| match pair {
        [a, b] => a.plugin_id.as_bytes() < b.plugin_id.as_bytes(),
        _ => true,
    })
}

impl CanonicalEncode for AttestationPayload {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        if !results_sorted_by_plugin_id(&self.results) {
            return Err(PrikkError::CanonicalEncoding(
                "plugin results must be strictly ordered and unique by plugin_id".to_string(),
            ));
        }
        writer.field_object_id(1, &self.target_block_id)?;
        writer.field_string(2, &self.policy_version)?;
        writer.field_bytes(3, &self.plugin_set_hash)?;
        writer.repeated_record_list(4, &self.results)?;
        writer.field_enum_u16(5, self.status.code())?;
        writer.field_u64(6, self.created_at)?;
        writer.field_bool(7, self.is_reproducible_offline)?;
        Ok(())
    }
}
