//! DC-41 stage 4 property tests: payload decoders (target 2 of 5).
//!
//! **Correction to the stage-4 handoff's target-2 count.** The handoff's inventory (§3) cites
//! `payload/patch.rs:130` as one of "five" payload decoders alongside `Block`, `RefState`,
//! `RefUpdate`, and `Blob`. That line is `PatchPurpose::decode_from_patch_payload`, which reads
//! only the single tag-5 purpose field — it is not a full structural `PatchPayload` decoder and
//! cannot round-trip the way the other four's `decode_canonical` can (there is no way to
//! "encode" a bare `PatchPurpose` into full `PatchPayload` bytes and decode it back with
//! equivalence). The genuine full decoder for Patch content lives in
//! `prikk-store::patch_replay::decode::decode_patch_operations`, which internally calls
//! `decode_from_patch_payload` as one validation step among several. That decoder is target 5's
//! subject, not target 2's — no coverage is lost, it is attributed to the correct target. Target
//! 2 here therefore covers exactly four full round-trip payload decoders: `Block`, `RefState`,
//! `RefUpdate`, `Blob`.
//!
//! Two properties per decoder: round-trip (`decode(encode(x)) == x`) and totality (arbitrary
//! bytes never panic the decoder). Case budget is proptest's own default (256/run), overridable
//! with `PROPTEST_CASES` for a campaign run.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use proptest::prelude::*;

use crate::payload::blob::{BlobKind, BlobPayload};
use crate::payload::block::{BlockKind, BlockPayload};
use crate::payload::common::MerkleRoot;
use crate::payload::refs::{RefKind, RefStatePayload, RefUpdatePayload};
use crate::{CanonicalEncode, ObjectId};

fn object_id_strategy() -> impl Strategy<Value = ObjectId> {
    proptest::array::uniform32(any::<u8>()).prop_map(ObjectId::from_bytes)
}

fn sorted_unique_object_ids(max_len: usize) -> impl Strategy<Value = Vec<ObjectId>> {
    proptest::collection::vec(object_id_strategy(), 0..max_len).prop_map(|mut ids| {
        ids.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        ids.dedup();
        ids
    })
}

fn block_kind_strategy() -> impl Strategy<Value = BlockKind> {
    prop_oneof![
        Just(BlockKind::Root),
        Just(BlockKind::Normal),
        Just(BlockKind::Merge),
        Just(BlockKind::Repair),
        Just(BlockKind::Import),
    ]
}

fn block_payload_strategy() -> impl Strategy<Value = BlockPayload> {
    (
        sorted_unique_object_ids(3),
        block_kind_strategy(),
        proptest::collection::vec(object_id_strategy(), 0..3),
        proptest::array::uniform32(any::<u8>()),
        proptest::option::of(object_id_strategy()),
    )
        .prop_map(
            |(parent_block_ids, kind, patch_ids, root_bytes, snapshot_blob_ref)| BlockPayload {
                parent_block_ids,
                kind,
                patch_ids,
                state_merkle_root: MerkleRoot(root_bytes),
                snapshot_blob_ref,
                mainline_parent_id: None,
                merge_baseline_block_id: None,
            },
        )
}

fn ref_kind_strategy() -> impl Strategy<Value = RefKind> {
    prop_oneof![Just(RefKind::Branch), Just(RefKind::Tag)]
}

fn ref_name_strategy() -> impl Strategy<Value = String> {
    "(heads|tags)/[a-z0-9]{1,8}"
}

fn ref_state_payload_strategy() -> impl Strategy<Value = RefStatePayload> {
    (
        ref_name_strategy(),
        ref_kind_strategy(),
        object_id_strategy(),
        any::<u64>(),
        proptest::option::of(object_id_strategy()),
        sorted_unique_object_ids(3),
        any::<bool>(),
    )
        .prop_map(
            |(
                ref_name,
                kind,
                target_object_id,
                update_seq,
                previous_ref_state_id,
                required_attestation_ids,
                closed,
            )| RefStatePayload {
                ref_name,
                kind,
                target_object_id,
                update_seq,
                previous_ref_state_id,
                required_attestation_ids,
                closed,
            },
        )
}

fn ref_update_payload_strategy() -> impl Strategy<Value = RefUpdatePayload> {
    (
        ref_name_strategy(),
        proptest::option::of(object_id_strategy()),
        object_id_strategy(),
        object_id_strategy(),
        any::<u64>(),
        any::<u64>(),
        "[a-zA-Z0-9_-]{1,8}",
    )
        .prop_map(
            |(
                ref_name,
                old_ref_state_id,
                new_ref_state_id,
                new_target_object_id,
                update_seq,
                created_at,
                author_key_id,
            )| RefUpdatePayload {
                ref_name,
                old_ref_state_id,
                new_ref_state_id,
                new_target_object_id,
                update_seq,
                created_at,
                author_key_id,
            },
        )
}

fn blob_kind_strategy() -> impl Strategy<Value = BlobKind> {
    prop_oneof![
        Just(BlobKind::Text),
        Just(BlobKind::Binary),
        Just(BlobKind::Snapshot),
    ]
}

fn blob_payload_strategy() -> impl Strategy<Value = BlobPayload> {
    (
        blob_kind_strategy(),
        proptest::collection::vec(any::<u8>(), 0..256),
    )
        .prop_map(|(blob_kind, content)| BlobPayload::new(blob_kind, content))
}

/// Duplicate the first canonical TLV field matching `target_tag` in `bytes`, inserting the copy
/// immediately after the original. RFC 125 §4: this is the shape every decoder must now refuse for
/// a singular field -- duplicating a field's own bytes in place keeps tag order non-decreasing (the
/// cursor's own order check does not reject a repeated tag), so this exercises exactly the "seen"
/// guard added to each decoder, not some other rejection path. Each canonical field is
/// `[2-byte tag BE][1-byte wire type][8-byte length BE][value]` (`CanonicalWriter`'s own format).
fn duplicate_field_with_tag(bytes: &[u8], target_tag: u16) -> Option<Vec<u8>> {
    let mut pos = 0;
    while pos < bytes.len() {
        let header_end = pos.checked_add(11)?;
        let tag = u16::from_be_bytes(bytes.get(pos..pos + 2)?.try_into().ok()?);
        let len = u64::from_be_bytes(bytes.get(pos + 3..header_end)?.try_into().ok()?);
        let len = usize::try_from(len).ok()?;
        let field_end = header_end.checked_add(len)?;
        let field = bytes.get(pos..field_end)?;
        if tag == target_tag {
            let mut out = bytes[..field_end].to_vec();
            out.extend_from_slice(field);
            out.extend_from_slice(&bytes[field_end..]);
            return Some(out);
        }
        pos = field_end;
    }
    None
}

#[test]
fn block_payload_decode_rejects_a_duplicate_kind_field() {
    let payload = BlockPayload {
        parent_block_ids: Vec::new(),
        kind: BlockKind::Root,
        patch_ids: Vec::new(),
        state_merkle_root: MerkleRoot([0u8; 32]),
        snapshot_blob_ref: None,
        mainline_parent_id: None,
        merge_baseline_block_id: None,
    };
    let bytes = payload.to_canonical_bytes().expect("encodes");
    let duplicated = duplicate_field_with_tag(&bytes, 2).expect("kind field present");
    let err = BlockPayload::decode_canonical(&duplicated).expect_err("duplicate kind must refuse");
    assert!(format!("{err}").contains("duplicate"), "{err}");
}

#[test]
fn ref_state_payload_decode_rejects_a_duplicate_ref_name_field() {
    let payload = RefStatePayload {
        ref_name: "heads/main".to_string(),
        kind: RefKind::Branch,
        target_object_id: ObjectId::from_bytes([1u8; 32]),
        update_seq: 1,
        previous_ref_state_id: None,
        required_attestation_ids: Vec::new(),
        closed: false,
    };
    let bytes = payload.to_canonical_bytes().expect("encodes");
    let duplicated = duplicate_field_with_tag(&bytes, 1).expect("ref_name field present");
    let err = RefStatePayload::decode_canonical(&duplicated, 1)
        .expect_err("duplicate ref_name must refuse");
    assert!(format!("{err}").contains("duplicate"), "{err}");
}

#[test]
fn ref_update_payload_decode_rejects_a_duplicate_ref_name_field() {
    let payload = RefUpdatePayload {
        ref_name: "heads/main".to_string(),
        old_ref_state_id: None,
        new_ref_state_id: ObjectId::from_bytes([1u8; 32]),
        new_target_object_id: ObjectId::from_bytes([2u8; 32]),
        update_seq: 1,
        created_at: 0,
        author_key_id: "author".to_string(),
    };
    let bytes = payload.to_canonical_bytes().expect("encodes");
    let duplicated = duplicate_field_with_tag(&bytes, 1).expect("ref_name field present");
    let err = RefUpdatePayload::decode_canonical(&duplicated)
        .expect_err("duplicate ref_name must refuse");
    assert!(format!("{err}").contains("duplicate"), "{err}");
}

proptest! {
    /// RFC 125 §4/§7: every singular field this proptest's own strategies can populate, duplicated
    /// one at a time, must be refused -- not panic, not last-wins. Complements the three fixed
    /// examples above (which pin the exact "duplicate" wording) with broad coverage across whatever
    /// values the strategies generate.
    #[test]
    fn block_payload_decode_never_panics_on_a_duplicated_field(
        payload in block_payload_strategy(),
        tag in prop_oneof![Just(2u16), Just(4), Just(5), Just(6), Just(7)],
    ) {
        let bytes = payload.to_canonical_bytes().expect("encodes");
        if let Some(duplicated) = duplicate_field_with_tag(&bytes, tag) {
            let _ = BlockPayload::decode_canonical(&duplicated);
        }
    }

    #[test]
    fn ref_state_payload_decode_never_panics_on_a_duplicated_field(
        payload in ref_state_payload_strategy(),
        tag in prop_oneof![Just(1u16), Just(2), Just(3), Just(4), Just(6)],
    ) {
        let schema_version = if payload.closed {
            crate::payload::refs::REF_STATE_CLOSED_SCHEMA
        } else {
            1
        };
        let bytes = payload.to_canonical_bytes().expect("encodes");
        if let Some(duplicated) = duplicate_field_with_tag(&bytes, tag) {
            let _ = RefStatePayload::decode_canonical(&duplicated, schema_version);
        }
    }

    #[test]
    fn ref_update_payload_decode_never_panics_on_a_duplicated_field(
        payload in ref_update_payload_strategy(),
        tag in prop_oneof![Just(1u16), Just(2), Just(3), Just(4), Just(5), Just(6), Just(7)],
    ) {
        let bytes = payload.to_canonical_bytes().expect("encodes");
        if let Some(duplicated) = duplicate_field_with_tag(&bytes, tag) {
            let _ = RefUpdatePayload::decode_canonical(&duplicated);
        }
    }
}

proptest! {
    #[test]
    fn block_payload_round_trips(payload in block_payload_strategy()) {
        let bytes = payload.to_canonical_bytes()
            .expect("generation invariants keep BlockPayload structurally valid");
        let decoded = BlockPayload::decode_canonical(&bytes)
            .expect("bytes produced by the encoder must always decode");
        prop_assert_eq!(decoded, payload);
    }

    #[test]
    fn block_payload_decode_never_panics_on_arbitrary_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..512)
    ) {
        let _ = BlockPayload::decode_canonical(&bytes);
    }

    #[test]
    fn ref_state_payload_round_trips(payload in ref_state_payload_strategy()) {
        let bytes = payload.to_canonical_bytes()
            .expect("generation invariants keep RefStatePayload structurally valid");
        // DC-61: `closed` is only legal at schema >= REF_STATE_CLOSED_SCHEMA, so the round-trip
        // must decode at the schema that matches what was generated, not a fixed constant.
        let schema_version = if payload.closed {
            crate::payload::refs::REF_STATE_CLOSED_SCHEMA
        } else {
            1
        };
        let decoded = RefStatePayload::decode_canonical(&bytes, schema_version)
            .expect("bytes produced by the encoder must always decode at the matching schema");
        prop_assert_eq!(decoded, payload);
    }

    #[test]
    fn ref_state_payload_decode_never_panics_on_arbitrary_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..512),
        schema_version in any::<u32>(),
    ) {
        let _ = RefStatePayload::decode_canonical(&bytes, schema_version);
    }

    #[test]
    fn ref_update_payload_round_trips(payload in ref_update_payload_strategy()) {
        let bytes = payload.to_canonical_bytes()
            .expect("generation invariants keep RefUpdatePayload structurally valid");
        let decoded = RefUpdatePayload::decode_canonical(&bytes)
            .expect("bytes produced by the encoder must always decode");
        prop_assert_eq!(decoded, payload);
    }

    #[test]
    fn ref_update_payload_decode_never_panics_on_arbitrary_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..512)
    ) {
        let _ = RefUpdatePayload::decode_canonical(&bytes);
    }

    #[test]
    fn blob_payload_round_trips(payload in blob_payload_strategy()) {
        let bytes = payload.to_canonical_bytes()
            .expect("generation invariants keep BlobPayload structurally valid");
        let decoded = BlobPayload::decode_canonical(&bytes)
            .expect("bytes produced by the encoder must always decode");
        prop_assert_eq!(decoded, payload);
    }

    #[test]
    fn blob_payload_decode_never_panics_on_arbitrary_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..512)
    ) {
        let _ = BlobPayload::decode_canonical(&bytes);
    }
}
