use super::{ObjectEnvelope, SignatureEnvelopeIssues};
use crate::{
    CanonicalEncode, CanonicalWriter, ObjectType, Signature, SignatureAlgorithm, SignerRole,
};

fn signature(key_id: &str, byte: u8, created_at: u64) -> Signature {
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: key_id.to_string(),
        signature_bytes: vec![byte; 64],
        created_at,
        signer_role: SignerRole::Author,
    }
}

#[test]
fn signature_does_not_change_object_id() {
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, b"payload".to_vec());
    let before = envelope.object_id();
    assert!(envelope.add_signature(signature("k1", 1, 1)).is_ok());
    assert_eq!(before, envelope.object_id());
}

#[test]
fn strict_shape_matrix_preserves_structural_legacy_boundary() {
    for (length, structural, strict) in [
        (0, false, false),
        (1, true, false),
        (63, true, false),
        (64, true, true),
        (65, true, false),
    ] {
        let mut candidate = signature("shape", 1, 0);
        candidate.signature_bytes.resize(length, 1);
        let envelope = ObjectEnvelope {
            object_type: ObjectType::Patch,
            schema_version: 1,
            canonical_payload: Vec::new(),
            signatures: vec![candidate],
        };
        assert_eq!(envelope.validate().is_ok(), structural, "length {length}");
        assert_eq!(
            envelope.validate_strict().is_ok(),
            strict,
            "length {length}"
        );
    }
}

#[test]
fn issues_distinguish_duplicate_from_descending_order() {
    let duplicate = signature("a", 1, 1);
    let envelope = ObjectEnvelope {
        object_type: ObjectType::Patch,
        schema_version: 1,
        canonical_payload: Vec::new(),
        signatures: vec![
            duplicate.clone(),
            Signature {
                created_at: 2,
                ..duplicate
            },
        ],
    };
    assert_eq!(
        envelope.signature_issues(),
        Ok(SignatureEnvelopeIssues {
            malformed_shape: false,
            duplicate: true,
            noncanonical_order: false,
        })
    );

    let envelope = ObjectEnvelope {
        signatures: vec![signature("b", 2, 0), signature("a", 1, 0)],
        ..envelope
    };
    assert_eq!(
        envelope.signature_issues(),
        Ok(SignatureEnvelopeIssues {
            malformed_shape: false,
            duplicate: false,
            noncanonical_order: true,
        })
    );

    let first = signature("a", 1, 1);
    let envelope = ObjectEnvelope {
        signatures: vec![
            first.clone(),
            signature("b", 2, 0),
            Signature {
                created_at: 99,
                ..first
            },
        ],
        ..envelope
    };
    assert_eq!(
        envelope.signature_issues(),
        Ok(SignatureEnvelopeIssues {
            malformed_shape: false,
            duplicate: true,
            noncanonical_order: true,
        })
    );
    assert!(matches!(
        envelope.validate_strict(),
        Err(error) if error.to_string().contains("duplicate signature tuple")
    ));
}

#[test]
fn add_signature_rejects_every_invalid_predecessor_without_mutation() {
    let duplicate = signature("same", 1, 0);
    let mut repeated = duplicate.clone();
    repeated.created_at = 99;
    let mut malformed = signature("shape", 1, 0);
    malformed.signature_bytes.truncate(1);
    for signatures in [
        vec![signature("b", 2, 0), signature("a", 1, 0)],
        vec![duplicate, signature("z", 2, 0), repeated],
        vec![malformed],
    ] {
        let mut envelope = ObjectEnvelope {
            object_type: ObjectType::Patch,
            schema_version: 1,
            canonical_payload: Vec::new(),
            signatures,
        };
        let before = envelope.clone();
        assert!(envelope.add_signature(signature("c", 3, 0)).is_err());
        assert_eq!(envelope, before);
    }
}

#[test]
fn insertion_order_has_one_canonical_encoding() -> prikk_error::Result<()> {
    let mut left = ObjectEnvelope::unsigned(ObjectType::Patch, 1, b"payload".to_vec());
    left.add_signature(signature("b", 2, 8))?;
    left.add_signature(signature("a", 1, 9))?;
    let mut right = ObjectEnvelope::unsigned(ObjectType::Patch, 1, b"payload".to_vec());
    right.add_signature(signature("a", 1, 9))?;
    right.add_signature(signature("b", 2, 8))?;
    assert_eq!(left.signatures, right.signatures);
    assert_eq!(left.to_canonical_bytes(), right.to_canonical_bytes());
    Ok(())
}

#[test]
fn canonical_serializer_rejects_every_invalid_class_before_output() -> prikk_error::Result<()> {
    let duplicate = signature("a", 1, 0);
    let mut malformed = signature("shape", 1, 0);
    malformed.signature_bytes.truncate(1);
    for signatures in [
        vec![
            duplicate.clone(),
            signature("b", 2, 0),
            Signature {
                created_at: 99,
                ..duplicate
            },
        ],
        vec![signature("z", 2, 0), signature("a", 1, 0)],
        vec![malformed],
    ] {
        let envelope = ObjectEnvelope {
            object_type: ObjectType::Patch,
            schema_version: 1,
            canonical_payload: Vec::new(),
            signatures,
        };
        let mut writer = CanonicalWriter::new();
        writer.field_u32(1, 7)?;
        let mut expected = CanonicalWriter::new();
        expected.field_u32(1, 7)?;
        assert!(envelope.encode_canonical(&mut writer).is_err());
        assert_eq!(writer.finish(), expected.finish());
    }
    Ok(())
}

#[test]
fn canonical_serializer_pins_shape_matrix_before_output() -> prikk_error::Result<()> {
    for length in [0, 1, 63, 64, 65] {
        let mut candidate = signature("shape", 1, 0);
        candidate.signature_bytes.resize(length, 1);
        let envelope = ObjectEnvelope {
            object_type: ObjectType::Patch,
            schema_version: 1,
            canonical_payload: Vec::new(),
            signatures: vec![candidate],
        };
        let mut writer = CanonicalWriter::new();
        writer.field_u32(1, 7)?;
        let mut expected = CanonicalWriter::new();
        expected.field_u32(1, 7)?;
        let before = expected.finish();
        let result = envelope.encode_canonical(&mut writer);
        assert_eq!(result.is_ok(), length == 64, "length {length}");
        if length != 64 {
            assert_eq!(writer.finish(), before, "length {length}");
        }
    }
    Ok(())
}
