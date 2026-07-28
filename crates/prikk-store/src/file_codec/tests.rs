//! DC-41 stage 4 property tests: envelope framing (target 1 of 5).
//!
//! Covers all ten current `ObjectType` variants. Two properties, per the RFC: round-trip
//! (`decode(encode(x)) == x`) for structurally valid envelopes, and totality (never panics) for
//! arbitrary bytes fed to the decoder. A third check pins envelope-layer admission against the
//! DC-40 format-2 schema allowlist (`crate::format::validate_format2_schema`) so the three
//! variants excluded from format-2 identity positions (`BlockSummaryCache`, `RecoveryNote`,
//! `ProjectGenesis`) are exercised on the rejection path rather than skipped.
//!
//! Case budget is proptest's own default (256/run), overridable with `PROPTEST_CASES` for a
//! campaign run; no explicit `ProptestConfig` override is set here.

#![allow(clippy::expect_used)]

use proptest::prelude::*;

use prikk_object::{ObjectEnvelope, ObjectType, Signature, SignatureAlgorithm, SignerRole};

use super::{decode_envelope_file, encode_envelope_file_structural};
use crate::format::validate_format2_schema;

const ALL_OBJECT_TYPES: [ObjectType; 10] = [
    ObjectType::Patch,
    ObjectType::Block,
    ObjectType::RefState,
    ObjectType::RefUpdate,
    ObjectType::Tag,
    ObjectType::Attestation,
    ObjectType::Blob,
    ObjectType::BlockSummaryCache,
    ObjectType::RecoveryNote,
    ObjectType::ProjectGenesis,
];

fn object_type_strategy() -> impl Strategy<Value = ObjectType> {
    (0..ALL_OBJECT_TYPES.len()).prop_map(|index| {
        ALL_OBJECT_TYPES
            .get(index)
            .copied()
            .unwrap_or(ObjectType::Patch)
    })
}

fn key_id_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,8}"
}

fn signature_strategy() -> impl Strategy<Value = Signature> {
    (
        key_id_strategy(),
        proptest::collection::vec(any::<u8>(), 1..64),
        any::<u64>(),
    )
        .prop_map(|(key_id, signature_bytes, created_at)| Signature {
            algorithm: SignatureAlgorithm::Ed25519,
            key_id,
            signature_bytes,
            created_at,
            signer_role: SignerRole::Author,
        })
}

fn envelope_strategy() -> impl Strategy<Value = ObjectEnvelope> {
    (
        object_type_strategy(),
        1_u32..=4,
        proptest::collection::vec(any::<u8>(), 0..256),
        proptest::collection::vec(signature_strategy(), 0..3),
    )
        .prop_map(
            |(object_type, schema_version, canonical_payload, signatures)| ObjectEnvelope {
                object_type,
                schema_version,
                canonical_payload,
                signatures,
            },
        )
}

proptest! {
    #[test]
    fn envelope_round_trips_through_file_codec(envelope in envelope_strategy()) {
        let encoded = encode_envelope_file_structural(&envelope)
            .expect("generation invariants keep the envelope structurally valid");
        let decoded = decode_envelope_file(&encoded)
            .expect("bytes produced by the encoder must always decode");
        prop_assert_eq!(decoded, envelope);
    }

    #[test]
    fn decode_envelope_file_never_panics_on_arbitrary_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..512)
    ) {
        // Totality: arbitrary bytes must decode to Ok or Err, never panic. The call itself is
        // the assertion; a panic fails the test, an Ok/Err result passes it.
        let _ = decode_envelope_file(&bytes);
    }

    #[test]
    fn format2_schema_admission_matches_dc40_allowlist(
        object_type in object_type_strategy(),
        schema_version in 0_u32..=5
    ) {
        let envelope = ObjectEnvelope {
            object_type,
            schema_version,
            canonical_payload: Vec::new(),
            signatures: Vec::new(),
        };
        let expected_ok = match object_type {
            ObjectType::Block => schema_version == 2,
            ObjectType::Patch
            | ObjectType::RefState
            | ObjectType::RefUpdate
            | ObjectType::Tag
            | ObjectType::Attestation
            | ObjectType::Blob => schema_version == 1,
            ObjectType::BlockSummaryCache
            | ObjectType::RecoveryNote
            | ObjectType::ProjectGenesis => false,
        };
        prop_assert_eq!(validate_format2_schema(&envelope).is_ok(), expected_ok);
    }
}
