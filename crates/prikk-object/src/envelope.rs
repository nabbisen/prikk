//! Object envelope.

use std::cmp::Ordering;

use prikk_error::{PrikkError, Result};

use crate::{CanonicalEncode, CanonicalWriter, ObjectId, ObjectType, Signature};

/// Non-canonical signature conditions visible during structural legacy decoding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SignatureEnvelopeIssues {
    /// At least one signature has a shape invalid for its registered algorithm.
    pub malformed_shape: bool,
    /// At least two signatures have the same canonical tuple.
    pub duplicate: bool,
    /// At least one non-equal adjacent tuple is descending.
    pub noncanonical_order: bool,
}

/// Object envelope containing unsigned canonical payload bytes plus external signatures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectEnvelope {
    /// Object type.
    pub object_type: ObjectType,
    /// Schema version.
    pub schema_version: u32,
    /// Unsigned canonical payload bytes.
    pub canonical_payload: Vec<u8>,
    /// Signatures over the object ID. Signatures are not part of the object ID preimage.
    pub signatures: Vec<Signature>,
}

impl ObjectEnvelope {
    /// Construct a new unsigned envelope.
    #[must_use]
    pub fn unsigned(
        object_type: ObjectType,
        schema_version: u32,
        canonical_payload: Vec<u8>,
    ) -> Self {
        Self {
            object_type,
            schema_version,
            canonical_payload,
            signatures: Vec::new(),
        }
    }

    /// Compute this envelope's object ID from its unsigned payload.
    #[must_use]
    pub fn object_id(&self) -> ObjectId {
        ObjectId::from_canonical_payload(
            self.object_type,
            self.schema_version,
            &self.canonical_payload,
        )
    }

    /// Validate envelope metadata and signatures structurally for legacy decoding.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version == 0 {
            return Err(PrikkError::UnsupportedFormatVersion(0));
        }
        for signature in &self.signatures {
            signature.validate()?;
        }
        Ok(())
    }

    /// Classify signature conditions after structural validation without changing the envelope.
    pub fn signature_issues(&self) -> Result<SignatureEnvelopeIssues> {
        self.validate()?;
        let mut issues = SignatureEnvelopeIssues {
            malformed_shape: self
                .signatures
                .iter()
                .any(|signature| signature.validate_shape().is_err()),
            ..SignatureEnvelopeIssues::default()
        };
        let mut signatures_by_tuple = self.signatures.iter().collect::<Vec<_>>();
        signatures_by_tuple.sort_unstable_by(|left, right| left.canonical_cmp(right));
        issues.duplicate = signatures_by_tuple
            .windows(2)
            .any(|pair| matches!(pair, [left, right] if left.canonical_cmp(right).is_eq()));
        for pair in self.signatures.windows(2) {
            let [left, right] = pair else {
                continue;
            };
            match left.canonical_cmp(right) {
                Ordering::Equal => {}
                Ordering::Greater => issues.noncanonical_order = true,
                Ordering::Less => {}
            }
        }
        Ok(issues)
    }

    /// Validate the envelope for canonical serialization, new writes, and current-format reads.
    pub fn validate_strict(&self) -> Result<()> {
        let issues = self.signature_issues()?;
        if issues.malformed_shape {
            return Err(PrikkError::InvalidSignature(
                "envelope contains a signature with malformed algorithm shape".to_string(),
            ));
        }
        if issues.duplicate {
            return Err(PrikkError::InvalidSignature(
                "envelope contains a duplicate signature tuple".to_string(),
            ));
        }
        if issues.noncanonical_order {
            return Err(PrikkError::InvalidSignature(
                "envelope signatures are not in canonical order".to_string(),
            ));
        }
        Ok(())
    }

    /// Append a signature without changing the object ID.
    pub fn add_signature(&mut self, signature: Signature) -> Result<()> {
        self.validate_strict()?;
        signature.validate()?;
        signature.validate_shape()?;
        match self
            .signatures
            .binary_search_by(|existing| existing.canonical_cmp(&signature))
        {
            Ok(_) => Err(PrikkError::InvalidSignature(
                "envelope contains a duplicate signature tuple".to_string(),
            )),
            Err(index) => {
                self.signatures.insert(index, signature);
                Ok(())
            }
        }
    }
}

impl CanonicalEncode for ObjectEnvelope {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        self.validate_strict()?;
        writer.field_u32(1, self.object_type.code() as u32)?;
        writer.field_u32(2, self.schema_version)?;
        writer.field_bytes(3, &self.canonical_payload)?;
        writer.repeated_record(4, &self.signatures)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
