//! Object envelope.

use prikk_error::{PrikkError, Result};

use crate::{CanonicalEncode, CanonicalWriter, ObjectId, ObjectType, Signature};

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

    /// Validate envelope metadata and signatures structurally.
    pub fn validate(&self) -> Result<()> {
        if self.schema_version == 0 {
            return Err(PrikkError::UnsupportedFormatVersion(0));
        }
        for signature in &self.signatures {
            signature.validate()?;
        }
        Ok(())
    }

    /// Append a signature. This does not recompute or change the object ID.
    pub fn add_signature(&mut self, signature: Signature) -> Result<()> {
        signature.validate()?;
        self.signatures.push(signature);
        self.signatures.sort_by(|a, b| {
            (&a.key_id, a.signer_role, a.created_at).cmp(&(&b.key_id, b.signer_role, b.created_at))
        });
        Ok(())
    }
}

impl CanonicalEncode for ObjectEnvelope {
    fn encode_canonical(&self, writer: &mut CanonicalWriter) -> Result<()> {
        writer.field_u32(1, self.object_type.code() as u32)?;
        writer.field_u32(2, self.schema_version)?;
        writer.field_bytes(3, &self.canonical_payload)?;
        writer.repeated_record(4, &self.signatures)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectEnvelope;
    use crate::{ObjectType, Signature, SignatureAlgorithm, SignerRole};

    #[test]
    fn signature_does_not_change_object_id() {
        let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, b"payload".to_vec());
        let before = envelope.object_id();
        let signature = Signature {
            algorithm: SignatureAlgorithm::Ed25519,
            key_id: "k1".to_string(),
            signature_bytes: vec![1, 2, 3],
            created_at: 1,
            signer_role: SignerRole::Author,
        };
        assert!(envelope.add_signature(signature).is_ok());
        assert_eq!(before, envelope.object_id());
    }
}
