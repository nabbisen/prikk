//! FDD-01 §5.1 text-span conformance vectors (DC-09 Phase 4.4-2c-4, carry-forward C2).
//!
//! These are the public conformance fixtures for the shared text-span identity rules. They pin the
//! identity-bearing outputs as **literals** (the frozen contract); each test recomputes the value
//! from the implementation and asserts equality to the literal. A change to any anchor preimage,
//! the span-id formula, occurrence enumeration, the splice, or the derived blob-id encoding would
//! change a pinned literal and fail here. The same values back the public §5.1 vector deliverable;
//! they are not replay-specific.

#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::unwrap_used)]

use prikk_object::{NodeId, text_span_hash};

use super::{
    TextSpanResolutionFailure, compute_span_id, left_anchor, locate_text_span, occurrences,
    right_anchor, splice_text, text_blob_id,
};

fn nid(b: u8) -> NodeId {
    NodeId::from_bytes([b; 32])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Zero-based index of the `(start, end)` occurrence within the anchor-matching occurrence list.
fn anchor_filtered_dup_index(text: &[u8], old_span: &[u8], start: usize, end: usize) -> u32 {
    let left = left_anchor(text, start);
    let right = right_anchor(text, end);
    let span_len = old_span.len();
    let mut idx = 0u32;
    for s in occurrences(text, old_span) {
        let e = s + span_len;
        if left_anchor(text, s) == left && right_anchor(text, e) == right {
            if s == start && e == end {
                return idx;
            }
            idx += 1;
        }
    }
    panic!("span ({start},{end}) is not anchor-matching");
}

/// One positive conformance vector: recompute the §5.1 identity chain and assert each stage against
/// the pinned literals, then assert the localization round-trips and the derived blob id matches.
#[allow(clippy::too_many_arguments)]
fn check_vector(
    text: &[u8],
    node_byte: u8,
    start: usize,
    end: usize,
    replacement: &[u8],
    exp_left: &str,
    exp_right: &str,
    exp_dup: u32,
    exp_span_id: &str,
    exp_new_text: &[u8],
    exp_blob: &str,
) {
    let node = nid(node_byte);
    let old_span = &text[start..end];
    let old_span_hash = text_span_hash(old_span);

    let left = left_anchor(text, start);
    let right = right_anchor(text, end);
    assert_eq!(hex(&left), exp_left, "left_anchor_hash");
    assert_eq!(hex(&right), exp_right, "right_anchor_hash");

    let dup = anchor_filtered_dup_index(text, old_span, start, end);
    assert_eq!(dup, exp_dup, "duplicate index");

    let span_id = compute_span_id(node, &old_span_hash, &left, &right, dup);
    assert_eq!(hex(&span_id), exp_span_id, "span_id");

    // Localization round-trips to the chosen byte range.
    let located = locate_text_span(
        text,
        old_span,
        &left,
        &right,
        &span_id,
        node,
        &old_span_hash,
    )
    .expect("localization");
    assert_eq!(located, (start, end), "localized byte range");

    // Splice (definitional) and derived content identity.
    let new_text = splice_text(text, start, end, replacement).expect("splice");
    assert_eq!(new_text, exp_new_text, "resulting text bytes");
    let blob = text_blob_id(&new_text).expect("blob id");
    assert_eq!(blob.to_hex(), exp_blob, "resulting text blob id");
}

#[test]
fn fdd01_text_span_v1_left_boundary_clamp() {
    check_vector(
        b"hello world",
        0x40,
        0,
        5,
        b"HELLO",
        "26a405baea8556d018906cea0e268b6f7d78044689f09830bcdc652f211d6ace",
        "a275c62adc35d21b3d829357fbc82d35f8442dc56a0888386245a2ffe22d25db",
        0,
        "aa23f1ad1acf5f51b477c4aec45c51259385b0217efe725bb9308f05d65f244d",
        b"HELLO world",
        "336d4bd0dc415a614fba770ba361b7fefcd272bbf8ed7b000ea35d75496465a8",
    );
}

#[test]
fn fdd01_text_span_v2_right_boundary_clamp() {
    check_vector(
        b"hello world",
        0x41,
        6,
        11,
        b"WORLD",
        "62136ffd5ad9a2e057700bdc6d652bb27f57cb10c23c37cf762c11c42f0ff0e9",
        "4eaa7fee2196ceb5ce372f8e7e90c1fb23636908c5adecae80ae8ff67c0d3e47",
        0,
        "0051a3f4746505f306753f06f0a7edd3bba413830d49ec0c419e5aa9a0c2aa3a",
        b"hello WORLD",
        "226bcccdf45fb6ca1a895792741a23ad0197af5dfd1a3ceca01782512587dc3b",
    );
}

#[test]
fn fdd01_text_span_v3_empty_file_insertion() {
    check_vector(
        b"",
        0x42,
        0,
        0,
        b"X",
        "26a405baea8556d018906cea0e268b6f7d78044689f09830bcdc652f211d6ace",
        "4eaa7fee2196ceb5ce372f8e7e90c1fb23636908c5adecae80ae8ff67c0d3e47",
        0,
        "b88b2d99f3bd4f236b05337fcab0f08c366de851abc93ca57fd8a338cc4e8156",
        b"X",
        "e912606b8ebb5cb3d4353d9660e342d4f2959ff3e3f2e6a9265b8c515584be5f",
    );
}

#[test]
fn fdd01_text_span_v4_zero_length_insertion() {
    check_vector(
        b"abc",
        0x43,
        1,
        1,
        b"XY",
        "fed24d8f3610cfc6192abe666a9fb6ef2e9a5d7f97ab9d5631371a982097d49a",
        "e9a3d04a23647053c5bb7fc2ad48982b6cacef1079d96347a5223ab1b1a35728",
        0,
        "ded854850fdabd03a59f0c9c4826447b71ae63bbb99eb012edf6e41fad8b8fb5",
        b"aXYbc",
        "0c476748d3996e6ba30485cbb253fc9546107f0c4caefcff2c46e08a2e543614",
    );
}

#[test]
fn fdd01_text_span_v5_overlapping_occurrences() {
    // "aaa" with span "aa": overlapping occurrences at 0 and 1; this vector pins occurrence (1,3).
    check_vector(
        b"aaa",
        0x44,
        1,
        3,
        b"bb",
        "fed24d8f3610cfc6192abe666a9fb6ef2e9a5d7f97ab9d5631371a982097d49a",
        "4eaa7fee2196ceb5ce372f8e7e90c1fb23636908c5adecae80ae8ff67c0d3e47",
        0,
        "3e8bd46cd1111b1a6397c3a961b6884223f63204e81c6217de886b61d0b46ac0",
        b"abb",
        "4b0c41a998cba378530278e6ce450a40a5e93312cb976fc786708e9e94773650",
    );
}

#[test]
fn fdd01_text_span_v6_duplicate_raw_occurrences_different_anchors() {
    // "Xab Yab": "ab" occurs twice with different anchors; the record for (5,7) selects it uniquely.
    check_vector(
        b"Xab Yab",
        0x45,
        5,
        7,
        b"QQ",
        "a0f2a84fb603e6185d4a260829086c791595d55ac0ae35621b840c34532bb247",
        "4eaa7fee2196ceb5ce372f8e7e90c1fb23636908c5adecae80ae8ff67c0d3e47",
        0,
        "133bd71a1ca706a3c6d2e4f69fbd0b57778bb43987fc5ea4649226eaedacee90",
        b"Xab YQQ",
        "ceeef71a0de1948db93039107a41932d7bf28287da81b8bd5cc8ba6c64cea6f8",
    );
}

/// Build the v7 text: two identical segments `('p'*64) ‖ "ab" ‖ ('q'*64)`, so both "ab"
/// occurrences share the same 64-byte left/right anchors and are distinguished only by duplicate
/// index. Returns `(text, expected_new_text)` for the edit of the **second** occurrence (194,196).
fn v7_texts() -> (Vec<u8>, Vec<u8>) {
    let seg = {
        let mut s = vec![b'p'; 64];
        s.extend_from_slice(b"ab");
        s.extend_from_slice(&[b'q'; 64]);
        s
    };
    let mut text = seg.clone();
    text.extend_from_slice(&seg);
    // Edit the second segment's "ab" (at 194..196) -> "ZZ".
    let mut expected = seg.clone();
    let mut seg2 = vec![b'p'; 64];
    seg2.extend_from_slice(b"ZZ");
    seg2.extend_from_slice(&[b'q'; 64]);
    expected.extend_from_slice(&seg2);
    (text, expected)
}

#[test]
fn fdd01_text_span_v7_duplicate_anchor_filtered_indices() {
    let (text, expected_new_text) = v7_texts();
    check_vector(
        &text,
        0x46,
        194,
        196,
        b"ZZ",
        "f996e008727c550b8622fc755ee80efc3f6eb2000ef2bf7052ee6f65ecc345d5",
        "e7dffe965215300976a6408e484134932a756d1b8052eda9aa352f9ed4506044",
        1, // second of two anchor-identical occurrences
        "ecdd21dc10bd0164cd578bb1d13b4ce813bcf908fdbcb6b98aa93ef4ee41bca4",
        &expected_new_text,
        "68153c238ae9d4128112cb1ea82dd1a5fb7e767e8843561fdba4b40508d4bfcc",
    );
}

#[test]
fn fdd01_text_span_negative_anchor_mismatch() {
    // A record whose left anchor matches no occurrence in the text -> AnchorMismatch.
    let text = b"hello world";
    let node = nid(0x40);
    let old_span = &text[0..5];
    let old_span_hash = text_span_hash(old_span);
    let right = right_anchor(text, 5);
    let bad_left = [0xab_u8; 32];
    let span_id = compute_span_id(node, &old_span_hash, &bad_left, &right, 0);
    let err = locate_text_span(
        text,
        old_span,
        &bad_left,
        &right,
        &span_id,
        node,
        &old_span_hash,
    )
    .expect_err("anchor mismatch");
    assert_eq!(err, TextSpanResolutionFailure::AnchorMismatch);
}

#[test]
fn fdd01_text_span_negative_wrong_span_id() {
    // Correct anchors (one occurrence), but a span_id that no anchor-filtered candidate reproduces.
    let text = b"hello world";
    let node = nid(0x40);
    let old_span = &text[0..5];
    let old_span_hash = text_span_hash(old_span);
    let left = left_anchor(text, 0);
    let right = right_anchor(text, 5);
    let bad_span_id = [0xcd_u8; 32];
    let err = locate_text_span(
        text,
        old_span,
        &left,
        &right,
        &bad_span_id,
        node,
        &old_span_hash,
    )
    .expect_err("wrong span id");
    assert_eq!(err, TextSpanResolutionFailure::NoMatchingSpanId);
}

// ---- splice_text invalid-range guard (2c-4 carry #1, pulled into 4.4a-1 per review E6) ----

#[test]
fn splice_rejects_start_after_end() {
    let err = splice_text(b"hello", 3, 2, b"x").expect_err("start > end");
    assert_eq!(err.start, 3);
    assert_eq!(err.end, 2);
}

#[test]
fn splice_rejects_end_past_text_len() {
    let err = splice_text(b"hello", 0, 6, b"x").expect_err("end > len");
    assert_eq!(err.end, 6);
    assert_eq!(err.text_len, 5);
}
