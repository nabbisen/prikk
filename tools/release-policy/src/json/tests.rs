#![allow(clippy::expect_used, clippy::indexing_slicing)]

use super::{JsonErrorKind, parse};

#[test]
fn rejects_recursive_and_escaped_duplicate_names() {
    for bytes in [
        br#"{"a":1,"a":2}"#.as_slice(),
        br#"{"outer":{"a":1,"\u0061":2}}"#.as_slice(),
        br#"[{"a":1,"a":2}]"#.as_slice(),
    ] {
        let error = parse(bytes).expect_err("duplicate name must fail");
        assert_eq!(error.kind, JsonErrorKind::DuplicateName);
    }
}

#[test]
fn preserves_number_type_distinctions_used_by_schema() {
    let value = parse(br#"[true,1,1.0]"#).expect("valid JSON");
    assert!(value[0].is_boolean());
    assert!(value[1].is_i64());
    assert!(value[2].is_f64());
}

#[test]
fn rejects_bom_and_lone_surrogate() {
    assert!(parse(b"\xef\xbb\xbf{}").is_err());
    assert!(parse(br#""\ud800""#).is_err());
}
