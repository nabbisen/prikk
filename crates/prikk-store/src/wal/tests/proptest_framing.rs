//! DC-41 stage 4 property tests: WAL record framing and replay reconstruction (target 3 of 5).
//!
//! This target also covers the RFC's "replay… reconstruction from WAL" bullet: `Wal::replay()`
//! is exactly `decode_records` over the WAL file's bytes, so a decode round-trip here is a
//! replay round-trip. It does **not** cover `lifecycle_cache.rs`'s separate
//! `DecodedLifecycleCache::decode` — that decoder, magic, and supporting types are entirely
//! `#[cfg(test)]`-gated (test-only scaffolding; per its own module doc, "replay
//! reconstruction/compare are later slices") and not reachable from production `Wal::replay()`
//! today. Recorded here rather than silently dropped: it is out of scope for this target because
//! it is not yet wired into the WAL replay path this property actually exercises.
//!
//! Two properties: round-trip for a sequence of validly-framed records (including a trailing
//! partial suffix, since that is WAL's documented recoverable case, not an error case), and
//! totality for arbitrary bytes. Case budget is proptest's own default (256/run), overridable
//! with `PROPTEST_CASES` for a campaign run.

#![allow(clippy::expect_used)]

use proptest::prelude::*;

use prikk_object::{ObjectEnvelope, ObjectType, Signature, SignatureAlgorithm, SignerRole};

use super::super::{WalRecord, decode_records, encode_record_for_test};

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
        proptest::collection::vec(any::<u8>(), 0..128),
        proptest::collection::vec(signature_strategy(), 0..2),
    )
        .prop_map(|(canonical_payload, signatures)| ObjectEnvelope {
            object_type: ObjectType::Patch,
            schema_version: 1,
            canonical_payload,
            signatures,
        })
}

/// One to three sequential records, `seq` starting at 1 (matching real WAL usage; `decode_records`
/// itself does not require contiguity, only per-record checksum/framing validity, but sequential
/// seq is the realistic shape and keeps the round-trip assertion meaningful).
fn records_strategy() -> impl Strategy<Value = Vec<WalRecord>> {
    proptest::collection::vec(envelope_strategy(), 1..4).prop_map(|envelopes| {
        envelopes
            .into_iter()
            .enumerate()
            .map(|(index, envelope)| WalRecord {
                seq: (index as u64) + 1,
                envelope,
            })
            .collect()
    })
}

proptest! {
    #[test]
    fn wal_records_round_trip_with_optional_trailing_partial(
        records in records_strategy(),
        trailing_partial_len in 0_usize..58
    ) {
        let mut bytes = Vec::new();
        for record in &records {
            let encoded = encode_record_for_test(record)
                .expect("generation invariants keep the envelope structurally valid");
            bytes.extend_from_slice(&encoded);
        }
        // Trailing partial bytes below WAL_HEADER_LEN (58) must never be mistaken for a record;
        // decode_records reports them as trailing_partial_bytes rather than erroring or panicking.
        bytes.extend(vec![0xAB_u8; trailing_partial_len]);

        let replay = decode_records(&bytes).expect("valid records plus a short suffix must decode");
        prop_assert_eq!(&replay.records, &records);
        prop_assert_eq!(replay.trailing_partial_bytes, trailing_partial_len);
    }

    #[test]
    fn decode_records_never_panics_on_arbitrary_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..512)
    ) {
        let _ = decode_records(&bytes);
    }
}
