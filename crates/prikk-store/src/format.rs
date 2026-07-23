//! Repository-format schema admission rules.

use prikk_error::{PrikkError, Result};
use prikk_object::{ObjectEnvelope, ObjectType};

use crate::layout::RepositoryFormat;

pub(crate) fn validate_object_envelope(
    format: RepositoryFormat,
    envelope: &ObjectEnvelope,
) -> Result<()> {
    envelope.validate_strict()?;
    match format {
        RepositoryFormat::LegacyV1 => Err(PrikkError::UnsupportedFormatVersion(1)),
        RepositoryFormat::CurrentV2 => validate_format2_schema(envelope),
    }
}

pub(crate) fn validate_format2_schema(envelope: &ObjectEnvelope) -> Result<()> {
    let expected = match envelope.object_type {
        ObjectType::Block => 2,
        ObjectType::Patch
        | ObjectType::RefState
        | ObjectType::RefUpdate
        | ObjectType::Tag
        | ObjectType::Attestation
        | ObjectType::Blob => 1,
        ObjectType::BlockSummaryCache | ObjectType::RecoveryNote | ObjectType::ProjectGenesis => {
            return Err(PrikkError::Integrity(format!(
                "{} is not authorized in a format-2 identity position",
                envelope.object_type
            )));
        }
    };
    if envelope.schema_version != expected {
        return Err(PrikkError::Integrity(format!(
            "format-2 {} requires envelope schema {expected}, got {}",
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
        RepositoryFormat::LegacyV1 => {
            if envelope.schema_version != 1 {
                return Err(PrikkError::Integrity(format!(
                    "format-1 {} requires envelope schema 1, got {}",
                    envelope.object_type, envelope.schema_version
                )));
            }
            Ok(())
        }
        RepositoryFormat::CurrentV2 => {
            envelope.validate_strict()?;
            validate_format2_schema(envelope)
        }
    }
}

#[cfg(test)]
mod tests;
