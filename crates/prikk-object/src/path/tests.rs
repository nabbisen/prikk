//! RFC 125 §3: `validate_repo_path` previously had no length check of any kind, so an
//! over-long path entered signed history and only failed later, at checkout, as a raw OS error
//! (`NAME_MAX`/`MAX_PATH`) on a repository that had already verified clean.

#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::{MAX_COMPONENT_LEN, MAX_TOTAL_LEN, validate_repo_path};

fn repeat(byte: char, count: usize) -> String {
    std::iter::repeat_n(byte, count).collect()
}

#[test]
fn a_component_at_the_limit_is_accepted() {
    let path = repeat('a', MAX_COMPONENT_LEN);
    assert!(validate_repo_path(&path).is_ok(), "{path}");
}

#[test]
fn a_component_one_byte_over_the_limit_is_refused() {
    let path = repeat('a', MAX_COMPONENT_LEN + 1);
    let err = validate_repo_path(&path).expect_err("over-length component must be refused");
    assert!(format!("{err}").contains("exceeds"), "{err}");
}

/// The total-length cap must fire even when every individual component is well within
/// [`MAX_COMPONENT_LEN`] -- many short components summing past [`MAX_TOTAL_LEN`] is exactly the
/// shape a per-component-only check would miss.
#[test]
fn many_short_components_summing_past_the_total_cap_are_refused() {
    let segment = repeat('a', 8);
    let segment_count = (MAX_TOTAL_LEN / (segment.len() + 1)) + 2;
    let path = std::iter::repeat_n(segment, segment_count)
        .collect::<Vec<_>>()
        .join("/");
    assert!(path.len() > MAX_TOTAL_LEN, "test path must exceed the cap");
    let err = validate_repo_path(&path).expect_err("over-length total path must be refused");
    assert!(format!("{err}").contains("exceeds"), "{err}");
}

#[test]
fn many_short_components_within_the_total_cap_are_accepted() {
    let segment = repeat('a', 8);
    let segment_count = 100; // 100 * 9 - 1 = 899 bytes, comfortably under MAX_TOTAL_LEN.
    let path = std::iter::repeat_n(segment, segment_count)
        .collect::<Vec<_>>()
        .join("/");
    assert!(
        path.len() < MAX_TOTAL_LEN,
        "test path must stay under the cap"
    );
    assert!(validate_repo_path(&path).is_ok(), "{path}");
}
