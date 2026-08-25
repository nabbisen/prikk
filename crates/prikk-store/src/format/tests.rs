use prikk_object::{ObjectEnvelope, ObjectType, Signature, SignatureAlgorithm, SignerRole};

use super::{validate_format2_schema, validate_read_schema};
use crate::layout::RepositoryFormat;

fn test_signature(key_id: &str, byte: u8) -> Signature {
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: key_id.to_string(),
        signature_bytes: vec![byte; 64],
        created_at: 0,
        signer_role: SignerRole::Author,
    }
}

fn strict_read_failures() -> [ObjectEnvelope; 3] {
    let mut malformed = test_signature("malformed", 1);
    malformed.signature_bytes.truncate(63);
    let duplicate = test_signature("duplicate", 2);
    [
        ObjectEnvelope {
            object_type: ObjectType::Blob,
            schema_version: 1,
            canonical_payload: b"strict-read".to_vec(),
            signatures: vec![malformed],
        },
        ObjectEnvelope {
            object_type: ObjectType::Blob,
            schema_version: 1,
            canonical_payload: b"strict-read".to_vec(),
            signatures: vec![duplicate.clone(), duplicate],
        },
        ObjectEnvelope {
            object_type: ObjectType::Blob,
            schema_version: 1,
            canonical_payload: b"strict-read".to_vec(),
            signatures: vec![test_signature("z", 3), test_signature("a", 1)],
        },
    ]
}

#[test]
fn format2_allowlist_covers_every_registered_type() {
    for (object_type, schema, allowed) in [
        (ObjectType::Patch, 1, true),
        // Patch schema 2 handoff: `PATCH_PARENT_IDS_RETIRED_SCHEMA` retires tag 2
        // (`parent_patch_ids`) outright. Both must be accepted.
        (ObjectType::Patch, 2, true),
        (ObjectType::Block, 2, true),
        (ObjectType::RefState, 1, true),
        // DC-61: RefState and Patch are now the only types with more than one live format-2
        // schema — RefState's schema 2 carries the `closed` field (tag 7); Patch's schema 2
        // retires `parent_patch_ids`. Both of RefState's schemas must be accepted.
        (ObjectType::RefState, 2, true),
        (ObjectType::RefUpdate, 1, true),
        (ObjectType::Tag, 1, true),
        (ObjectType::Attestation, 1, true),
        (ObjectType::Blob, 1, true),
        (ObjectType::RecognitionClaim, 1, true),
        (ObjectType::BlockSummaryCache, 1, false),
        (ObjectType::RecoveryNote, 1, false),
    ] {
        let envelope = ObjectEnvelope::unsigned(object_type, schema, Vec::new());
        assert_eq!(validate_format2_schema(&envelope).is_ok(), allowed);
    }
}

#[test]
fn format2_rejects_wrong_schema_for_every_allowed_type() {
    for object_type in [
        ObjectType::Patch,
        ObjectType::Block,
        ObjectType::RefState,
        ObjectType::RefUpdate,
        ObjectType::Tag,
        ObjectType::Attestation,
        ObjectType::Blob,
        ObjectType::RecognitionClaim,
    ] {
        // RefState and Patch each accept two schemas (RefState: 1 and REF_STATE_CLOSED_SCHEMA = 2,
        // DC-61; Patch: 1 and PATCH_PARENT_IDS_RETIRED_SCHEMA = 2), so a single "required + 1"
        // probe is not wrong for them the way it is for every other type; schema 3 is outside
        // every type's accepted set, including theirs.
        let wrong = match object_type {
            ObjectType::Block | ObjectType::RefState | ObjectType::Patch => 3,
            _ => 2,
        };
        let envelope = ObjectEnvelope::unsigned(object_type, wrong, Vec::new());
        assert!(validate_format2_schema(&envelope).is_err());
    }
}

#[test]
fn format2_read_rejects_every_strict_envelope_failure() {
    for envelope in strict_read_failures() {
        assert!(validate_read_schema(RepositoryFormat::CurrentV6, &envelope).is_err());
    }
}
