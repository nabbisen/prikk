//! RFC 116 stage 2 handoff §5, `PSYNCSU1`'s own rows: round trip, row 2 (declared ref count bounded
//! before allocation), row 4 (branches only).

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use prikk_error::Result;
use prikk_object::ObjectId;

use super::{
    DEFAULT_SYNC_SUMMARY_MAX_REF_COUNT, DEFAULT_SYNC_SUMMARY_MAX_TOTAL_BYTES, build_sync_summary,
    decode_sync_summary,
};
use crate::patch_set_digest::compute_patch_set_digest;
use crate::sync_negotiation::sync_test_support::{
    cleanup, fresh_repo, publish_branch, publish_received, publish_tag,
};

const REF_COUNT: usize = DEFAULT_SYNC_SUMMARY_MAX_REF_COUNT;
const TOTAL_BYTES: usize = DEFAULT_SYNC_SUMMARY_MAX_TOTAL_BYTES;

#[test]
fn an_empty_repository_encodes_a_summary_declaring_zero_refs() -> Result<()> {
    let layout = fresh_repo("sync-summary-empty")?;
    let bytes = build_sync_summary(&layout)?;
    let entries = decode_sync_summary(&bytes, TOTAL_BYTES, REF_COUNT)?;
    assert!(entries.is_empty());
    cleanup(&layout);
    Ok(())
}

#[test]
fn a_summary_round_trips_one_branchs_digest_and_count() -> Result<()> {
    let layout = fresh_repo("sync-summary-round-trip")?;
    let p1 = ObjectId::from_bytes([0x11; 32]);
    let p2 = ObjectId::from_bytes([0x12; 32]);
    publish_branch(&layout, "heads/main", vec![p1, p2])?;

    let bytes = build_sync_summary(&layout)?;
    let entries = decode_sync_summary(&bytes, TOTAL_BYTES, REF_COUNT)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].ref_name, "heads/main");
    assert_eq!(entries[0].patch_count, 2);
    let expected_digest = compute_patch_set_digest(&{
        let mut sorted = vec![p1, p2];
        sorted.sort_unstable();
        sorted
    })?;
    assert_eq!(entries[0].digest, expected_digest);
    cleanup(&layout);
    Ok(())
}

/// §5 row 2: a declared ref count over the limit is rejected on the integer, before any allocation
/// or per-entry read is attempted -- the crafted bytes below contain no entries at all, so a decode
/// that pressed on past the check would fail differently (running out of bytes), not with the
/// "over the configured limit" message this test asserts.
#[test]
fn a_ref_count_over_the_limit_is_rejected_before_allocating() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"PSYNCSU1");
    bytes.extend_from_slice(&(DEFAULT_SYNC_SUMMARY_MAX_REF_COUNT as u64 + 1).to_be_bytes());
    let result = decode_sync_summary(&bytes, TOTAL_BYTES, DEFAULT_SYNC_SUMMARY_MAX_REF_COUNT);
    let message = match result {
        Ok(_) => panic!("an over-limit declared ref count must be rejected"),
        Err(error) => error.to_string(),
    };
    assert!(
        message.contains("over the configured limit"),
        "expected the declared-count refusal, got: {message}"
    );
}

/// §5 row 7: total byte length is bounded before decoding starts. Built from a genuinely valid,
/// decodable summary -- not arbitrary bytes that would also fail the magic check downstream -- so
/// a decode that pressed on past the length check would actually *succeed*, not merely fail for
/// some other reason.
#[test]
fn oversized_total_bytes_is_refused_before_parsing() -> Result<()> {
    let layout = fresh_repo("sync-summary-oversized")?;
    publish_branch(
        &layout,
        "heads/main",
        vec![ObjectId::from_bytes([0x22; 32])],
    )?;
    let bytes = build_sync_summary(&layout)?;
    let result = decode_sync_summary(&bytes, bytes.len() - 1, REF_COUNT);
    let message = match result {
        Ok(_) => panic!("an oversized summary must be refused before decoding"),
        Err(error) => error.to_string(),
    };
    assert!(
        message.contains("over the configured limit"),
        "expected the total-byte-length refusal, got: {message}"
    );
    cleanup(&layout);
    Ok(())
}

/// §5 row 4: a repository with a `heads/*`, a `tags/*`, and a `remotes/*` ref reports only the
/// branch.
#[test]
fn a_summary_omits_tags_and_remotes() -> Result<()> {
    let layout = fresh_repo("sync-summary-branches-only")?;
    let block_id = publish_branch(
        &layout,
        "heads/main",
        vec![ObjectId::from_bytes([0x21; 32])],
    )?;
    publish_tag(&layout, "tags/v1", "v1", block_id)?;
    publish_received(&layout, "remotes/origin/heads/main", block_id)?;

    let bytes = build_sync_summary(&layout)?;
    let entries = decode_sync_summary(&bytes, TOTAL_BYTES, REF_COUNT)?;
    assert_eq!(entries.len(), 1, "only the branch ref must appear");
    assert_eq!(entries[0].ref_name, "heads/main");
    cleanup(&layout);
    Ok(())
}
