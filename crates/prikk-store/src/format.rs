//! Repository-format schema admission rules.

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectType, REF_STATE_CLOSED_SCHEMA};

use crate::layout::RepositoryFormat;

pub(crate) fn validate_object_envelope(
    format: RepositoryFormat,
    envelope: &ObjectEnvelope,
) -> Result<()> {
    envelope.validate_strict()?;
    match format {
        RepositoryFormat::CurrentV4 => validate_format2_schema(envelope),
    }
}

/// Format-2 schema admission per object type. Every type but `RefState` accepts exactly one
/// schema; `RefState` accepts schema 1 (open, the pre-DC-61 shape) or `REF_STATE_CLOSED_SCHEMA`
/// (closed, DC-61) — the only type in this repository with more than one live schema, because
/// closure is the only field ever added to an existing payload after its type shipped.
pub(crate) fn validate_format2_schema(envelope: &ObjectEnvelope) -> Result<()> {
    let accepted: &[u32] = match envelope.object_type {
        ObjectType::Block => &[2],
        ObjectType::RefState => &[1, REF_STATE_CLOSED_SCHEMA],
        ObjectType::Patch
        | ObjectType::RefUpdate
        | ObjectType::Tag
        | ObjectType::Attestation
        | ObjectType::Blob => &[1],
        ObjectType::BlockSummaryCache | ObjectType::RecoveryNote | ObjectType::ProjectGenesis => {
            return Err(PrikkError::Integrity(format!(
                "{} is not authorized in a format-2 identity position",
                envelope.object_type
            )));
        }
    };
    if !accepted.contains(&envelope.schema_version) {
        return Err(PrikkError::Integrity(format!(
            "format-2 {} does not accept envelope schema {} (accepted: {accepted:?})",
            envelope.object_type, envelope.schema_version
        )));
    }
    Ok(())
}

pub(crate) fn validate_read_schema(
    format: RepositoryFormat,
    envelope: &ObjectEnvelope,
) -> Result<()> {
    match format {
        RepositoryFormat::CurrentV4 => {
            envelope.validate_strict()?;
            validate_format2_schema(envelope)
        }
    }
}

#[cfg(test)]
mod tests;
