//! DC-86 property tests: the received-pointer wire format (magic, ref-name framing, raw object id).
//! `decode_received_pointer` is read whenever a received ref is listed, resolved, or re-imported, and
//! `import_bundle` is the path that writes its bytes from data a party the operator does not control
//! (`bundle.rs`'s own encoded objects come from the same source) — hardened the way DC-41 stage 4
//! hardened the object decoders.
//!
//! Two properties: round-trip for an arbitrary ref name and object id, and totality for arbitrary
//! bytes. Case budget is proptest's own default (256/run), overridable with `PROPTEST_CASES` for a
//! campaign run.

#![allow(clippy::expect_used)]

use proptest::prelude::*;

use prikk_object::ObjectId;

use super::super::{ReceivedPointer, decode_received_pointer, encode_received_pointer};

fn object_id_strategy() -> impl Strategy<Value = ObjectId> {
    proptest::array::uniform32(any::<u8>()).prop_map(ObjectId::from_bytes)
}

fn ref_name_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_/-]{0,32}"
}

proptest! {
    #[test]
    fn received_pointer_round_trips_an_arbitrary_ref_name_and_id(
        ref_name in ref_name_strategy(),
        ref_state_id in object_id_strategy()
    ) {
        let bytes = encode_received_pointer(&ref_name, ref_state_id)
            .expect("generation invariants keep the ref name encodable");
        let decoded = decode_received_pointer(&bytes)
            .expect("a pointer this small must decode");
        prop_assert_eq!(
            decoded,
            ReceivedPointer {
                ref_name,
                ref_state_id,
            }
        );
    }

    #[test]
    fn decode_received_pointer_never_panics_on_arbitrary_bytes(
        bytes in proptest::collection::vec(any::<u8>(), 0..256)
    ) {
        let _ = decode_received_pointer(&bytes);
    }
}
