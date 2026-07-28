//! DC-41 stage 4 property tests: ref-log entry framing (target 4 of 5).
//!
//! Mirrors `wal::tests::proptest_framing` exactly (same magic/header/checksum framing shape,
//! same two properties), for the ref-log's inline `RefUpdate` records instead of WAL patch
//! records. Case budget is proptest's own default (256/run), overridable with `PROPTEST_CASES`
//! for a campaign run.

#![allow(clippy::expect_used)]

use proptest::prelude::*;

use prikk_object::{ObjectEnvelope, ObjectType, Signature, SignatureAlgorithm, SignerRole};

use super::{RefLogRecord, decode_log_records, encode_log_record_for_test};

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

/// `require_signed_type(envelope, ObjectType::RefUpdate)` demands the right type code, at least
/// one signature, and `envelope.validate()` (nonzero schema version, valid signatures).
fn ref_update_envelope_strategy() -> impl Strategy<Value = ObjectEnvelope> {
    (
        proptest::collection::vec(any::<u8>(), 0..128),
        proptest::collection::vec(signature_strategy(), 1..3),
    )
        .prop_map(|(canonical_payload, signatures)| ObjectEnvelope {
            object_type: ObjectType::RefUpdate,
            schema_version: 1,
            canonical_payload,
            signatures,
        })
}

fn records_strategy() -> impl Strategy<Value = Vec<RefLogRecord>> {
    proptest::collection::vec(ref_update_envelope_strategy(), 1..4).prop_map(|envelopes| {
        envelopes
            .into_iter()
            .map(|envelope| RefLogRecord { envelope })
            .collect()
    })
}

proptest! {
    #[test]
    fn ref_log_records_round_trip_with_optional_trailing_partial(
        records in records_strategy(),
        trailing_partial_len in 0_usize..50
    ) {
        let mut bytes = Vec::new();
        for record in &records {
            let encoded = encode_log_record_for_test(&record.envelope)
                .expect("generation invariants keep the RefUpdate envelope structurally valid");
            bytes.extend_from_slice(&encoded);
        }
        // Trailing partial bytes below REF_LOG_HEADER_LEN (50) must never be mistaken for a
        // record; decode_log_records reports them as trailing_partial_bytes.
        bytes.extend(vec![0xCD_u8; trailing_partial_len]);

        let replay = decode_log_records(&bytes)
            .expect("valid records plus a short suffix must decode");
        prop_assert_eq!(&replay.records, &records);
        prop_assert_eq!(replay.trailing_partial_bytes, trailing_partial_len);
    }

    #[test]
    fn decode_log_records_never_panics_on_arbitrary_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..512)
    ) {
        let _ = decode_log_records(&bytes);
    }
}
