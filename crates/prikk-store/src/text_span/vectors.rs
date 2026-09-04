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
    TextSpanResolutionFailure, compute_span_id, left_anchor, locate_text_span, locate_text_span_v2,
    occurrences, plan_authored_text_span, right_anchor, splice_text, text_blob_id,
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

#[allow(clippy::too_many_arguments)]
fn check_plan_vector(
    old: &[u8],
    new: &[u8],
    node_byte: u8,
    exp_old_range: (usize, usize),
    exp_new_range: (usize, usize),
    exp_old_span_hex: &str,
    exp_replacement_hex: &str,
    exp_left: &str,
    exp_right: &str,
    exp_old_span_hash: &str,
    exp_left_len: u32,
    exp_right_len: u32,
    exp_span_id: &str,
    exp_blob: &str,
) {
    let node = nid(node_byte);
    let plan = plan_authored_text_span(old, new, node)
        .expect("selection succeeds")
        .expect("changed text yields a span");
    assert_eq!((plan.old_start, plan.old_end), exp_old_range, "old range");
    assert_eq!((plan.new_start, plan.new_end), exp_new_range, "new range");
    assert_eq!(hex(&plan.old_span_text), exp_old_span_hex, "old span bytes");
    assert_eq!(
        hex(&plan.replacement_text),
        exp_replacement_hex,
        "replacement bytes"
    );
    assert_eq!(hex(&plan.left_anchor_hash), exp_left, "left anchor");
    assert_eq!(hex(&plan.right_anchor_hash), exp_right, "right anchor");
    assert_eq!(hex(&plan.old_span_hash), exp_old_span_hash, "old span hash");
    assert_eq!(plan.left_anchor_len, exp_left_len, "left anchor length");
    assert_eq!(plan.right_anchor_len, exp_right_len, "right anchor length");
    assert_eq!(hex(&plan.span_id), exp_span_id, "span id");

    let (start, end) = locate_text_span_v2(
        old,
        &plan.old_span_text,
        &plan.left_anchor_hash,
        &plan.right_anchor_hash,
        &plan.span_id,
        node,
        &plan.old_span_hash,
        plan.left_anchor_len,
        plan.right_anchor_len,
    )
    .expect("selection localizes");
    assert_eq!((start, end), exp_old_range, "localized range");
    let spliced = splice_text(old, start, end, &plan.replacement_text).expect("splice");
    assert_eq!(spliced, new, "spliced bytes");
    assert_eq!(
        text_blob_id(&spliced).expect("blob id").to_hex(),
        exp_blob,
        "resulting text blob id"
    );
}

#[test]
fn dc12_span_selection_replacement_middle_pins_bytes() {
    check_plan_vector(
        b"hello world\n",
        b"hello prikk\n",
        0x50,
        (6, 11),
        (6, 11),
        "776f726c64",
        "7072696b6b",
        "acdae73d4309661c1c137229106fdf00771acdcc3420eb2d0e8b8c7df83c7f07",
        "5cc28673903ef8d8769649f1c6579ab153c2a4d045c14bdec1895e73e86a3c49",
        "486ea46224d1bb4fb680f34f7c9ad96a8f24ec88be73ea8e5a6c65260e9cb8a7",
        64,
        64,
        "1eb17073a6c2cb812067f5bf59f51b0e39c027d243cb7798078f9569b79fd03d",
        "1b05e8e870004a5852990d93a5610d80e56507b129e6bc90a82bd050c8a4f878",
    );
}

#[test]
fn dc12_span_selection_insertion_and_deletion_pin_empty_sides() {
    check_plan_vector(
        b"abc",
        b"aXYbc",
        0x51,
        (1, 1),
        (1, 3),
        "",
        "5859",
        "fe79445a886a85a5f86aef1913b10c0806b77f11ad1064250e528df179bf8412",
        "185b724858ff64f330958f185830e4c7b4ac519969020016e4c4a96f6db467d9",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        64,
        64,
        "fda82b90d738bfee48dfdea54b1899efc89d5db06280716752733b8bab96741f",
        "0c476748d3996e6ba30485cbb253fc9546107f0c4caefcff2c46e08a2e543614",
    );
    check_plan_vector(
        b"aXYbc",
        b"abc",
        0x52,
        (1, 3),
        (1, 1),
        "5859",
        "",
        "fe79445a886a85a5f86aef1913b10c0806b77f11ad1064250e528df179bf8412",
        "185b724858ff64f330958f185830e4c7b4ac519969020016e4c4a96f6db467d9",
        "c07a3de039fbc0914689549f041eae295d621de7f7f647fd863f6d2f8db2080e",
        64,
        64,
        "10daa2efd2e8a1e99743816e084667aef9693672c4d657de9c7b1dc11c0ebefe",
        "5b20e6b32faaac39492a52d9ae441b8b020f994af260a9093dbae92c698ff1f0",
    );
}

#[test]
fn dc12_span_selection_widens_subcharacter_edits() {
    check_plan_vector(
        "é\n".as_bytes(),
        "è\n".as_bytes(),
        0x53,
        (0, 2),
        (0, 2),
        "c3a9",
        "c3a8",
        "3f0f58a045b7b4f476227697ab5575be5b5660d777bc8f2c604bd547963e0045",
        "5cc28673903ef8d8769649f1c6579ab153c2a4d045c14bdec1895e73e86a3c49",
        "4a99557e4033c3539de2eb65472017cad5f9557f7a0625a09f1c3f6e2ba69c4c",
        64,
        64,
        "44089ddc36c53b96b8a7deb8c4268a2a232559477b1182e85ace6f5d7d3dc0c4",
        "9a1a64b30545e1d1bcccb3c51046b8147b17d3c30fa7f40751737954b717d90e",
    );
    check_plan_vector(
        "漢字\n".as_bytes(),
        "漢文\n".as_bytes(),
        0x54,
        (3, 6),
        (3, 6),
        "e5ad97",
        "e69687",
        "8975618fee2c4590e07a2bb6159f287d14f1c7447a8dade87518a50a8e9e7b79",
        "5cc28673903ef8d8769649f1c6579ab153c2a4d045c14bdec1895e73e86a3c49",
        "c55038b272b109b8bfdb6b59dd1b1048ffa58361caa3d16f56ee881f34ce34f0",
        64,
        64,
        "81eb40cd1e0b9d1388acab38ddde5a3c4b877ea0a0d0e9df006d9121f4e97d92",
        "46dc575d0a66126bf87bdaede2e6a811782641189f9dcea83e27772185ca509c",
    );
}

#[test]
fn dc12_span_selection_crlf_and_multihunk_enclosing_span() {
    check_plan_vector(
        b"a\r\nb\r\nc\r\n",
        b"A\r\nb\r\nC\r\n",
        0x55,
        (0, 7),
        (0, 7),
        "610d0a620d0a63",
        "410d0a620d0a43",
        "3f0f58a045b7b4f476227697ab5575be5b5660d777bc8f2c604bd547963e0045",
        "7b45ea78cdf9373e9bb14975001f08ab98e0e0f6bed9c6c7e88849b47aec450b",
        "d37a6c0b581046eec04a3d815bcd9fadbce89bd21784279deff41836a766d570",
        64,
        64,
        "3c8ad5ff6964a0e07520a39800a1764a172e92d207ee2b209a1f1524259e10f1",
        "bafa20d059386e336f6e812afe698233fa8fe9b1a5608f8d9919c3100d682a26",
    );
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
