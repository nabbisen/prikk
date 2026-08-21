//! RFC 116 stage 2 handoff §5, `PSYNCHV1`'s own rows: round trip, row 2 (declared patch count
//! bounded before allocation), row 3 (self-consistency).

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use prikk_error::Result;
use prikk_object::ObjectId;

use super::{
    DEFAULT_HAVE_LIST_MAX_PATCH_COUNT, DEFAULT_HAVE_LIST_MAX_TOTAL_BYTES, build_have_list,
    decode_have_list,
};
use crate::patch_set_digest::compute_patch_set_digest;
use crate::sync_negotiation::sync_test_support::{cleanup, fresh_repo, publish_branch};

const PATCH_COUNT: usize = DEFAULT_HAVE_LIST_MAX_PATCH_COUNT;
const TOTAL_BYTES: usize = DEFAULT_HAVE_LIST_MAX_TOTAL_BYTES;

#[test]
fn a_have_list_round_trips_a_branchs_patch_ids_and_digest() -> Result<()> {
    let layout = fresh_repo("have-list-round-trip")?;
    let p1 = ObjectId::from_bytes([0x31; 32]);
    let p2 = ObjectId::from_bytes([0x32; 32]);
    publish_branch(&layout, "heads/main", vec![p1, p2])?;

    let bytes = build_have_list(&layout, "heads/main")?;
    let have_list = decode_have_list(&bytes, TOTAL_BYTES, PATCH_COUNT)?;
    assert_eq!(have_list.ref_name, "heads/main");
    let mut expected = vec![p1, p2];
    expected.sort_unstable();
    assert_eq!(have_list.patch_ids, expected);
    assert_eq!(have_list.digest, compute_patch_set_digest(&expected)?);
    cleanup(&layout);
    Ok(())
}

/// A ref this repository does not hold locally still builds -- an empty patch list, not a refusal
/// (design §5 item 6 / N5 item 6).
#[test]
fn a_have_list_for_an_absent_ref_is_empty_not_refused() -> Result<()> {
    let layout = fresh_repo("have-list-absent-ref")?;
    let bytes = build_have_list(&layout, "heads/never-existed")?;
    let have_list = decode_have_list(&bytes, TOTAL_BYTES, PATCH_COUNT)?;
    assert_eq!(have_list.ref_name, "heads/never-existed");
    assert!(have_list.patch_ids.is_empty());
    assert_eq!(have_list.digest, compute_patch_set_digest(&[])?);
    cleanup(&layout);
    Ok(())
}

/// §5 row 2: a declared patch count over the limit is rejected on the integer, before any
/// allocation or per-id read is attempted.
#[test]
fn a_patch_count_over_the_limit_is_rejected_before_allocating() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"PSYNCHV1");
    bytes.extend_from_slice(&0_u16.to_be_bytes()); // empty ref name
    bytes.extend_from_slice(&[0_u8; 32]); // digest (unchecked before the count refusal)
    bytes.extend_from_slice(&(DEFAULT_HAVE_LIST_MAX_PATCH_COUNT as u64 + 1).to_be_bytes());
    let result = decode_have_list(&bytes, TOTAL_BYTES, DEFAULT_HAVE_LIST_MAX_PATCH_COUNT);
    let message = match result {
        Ok(_) => panic!("an over-limit declared patch count must be rejected"),
        Err(error) => error.to_string(),
    };
    assert!(
        message.contains("over the configured limit"),
        "expected the declared-count refusal, got: {message}"
    );
}

/// §5 row 7: total byte length is bounded before decoding starts.
#[test]
fn oversized_total_bytes_is_refused_before_parsing() {
    let bytes = vec![0_u8; 100];
    let result = decode_have_list(&bytes, 10, PATCH_COUNT);
    assert!(result.is_err(), "an oversized have-list must be refused");
}

/// §5 row 3: a have-list whose declared digest disagrees with its own carried list is refused.
/// Hand-built directly at the byte level -- the declared digest is computed over the FULL two-patch
/// list, but the encoded list itself is truncated to one, exactly the shape `build_have_list` could
/// never itself produce (its own digest and its own list always agree by construction).
#[test]
fn a_truncated_list_under_an_unchanged_digest_is_refused() -> Result<()> {
    let p1 = ObjectId::from_bytes([0x41; 32]);
    let p2 = ObjectId::from_bytes([0x42; 32]);
    let full_digest = compute_patch_set_digest(&[p1, p2])?;

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"PSYNCHV1");
    let ref_name = b"heads/main";
    bytes.extend_from_slice(&(ref_name.len() as u16).to_be_bytes());
    bytes.extend_from_slice(ref_name);
    bytes.extend_from_slice(&full_digest.0);
    bytes.extend_from_slice(&1_u64.to_be_bytes()); // declares one id, not two
    bytes.extend_from_slice(p1.as_bytes());

    let result = decode_have_list(&bytes, TOTAL_BYTES, PATCH_COUNT);
    assert!(
        result.is_err(),
        "a truncated list under an unchanged digest must be refused"
    );
    Ok(())
}
