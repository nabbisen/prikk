#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use super::{is_possibly_production, parse};

#[test]
fn plain_test_is_not_possibly_production() {
    let expr = parse("test").expect("parses");
    assert!(!is_possibly_production(Some(&expr)));
}

#[test]
fn plain_target_os_is_possibly_production() {
    let expr = parse("target_os = \"linux\"").expect("parses");
    assert!(is_possibly_production(Some(&expr)));
}

#[test]
fn all_test_and_target_os_is_not_possibly_production() {
    let expr = parse("all(test, target_os = \"windows\")").expect("parses");
    assert!(!is_possibly_production(Some(&expr)));
}

#[test]
fn any_of_test_and_target_os_is_possibly_production() {
    // Satisfiable via target_os = "linux" with test = false.
    let expr = parse("any(test, target_os = \"linux\")").expect("parses");
    assert!(is_possibly_production(Some(&expr)));
}

/// `fsutil/anchored.rs`'s own `none` module gate -- the exact case a naive substring check on the
/// word "test" gets wrong (RFC 130's own worked example).
#[test]
fn fsutil_none_modules_gate_is_possibly_production() {
    let expr = parse(
        "any(all(test, not(target_os = \"windows\")), not(any(target_os = \"linux\", \
         target_os = \"macos\", target_os = \"windows\")))",
    )
    .expect("parses");
    assert!(
        is_possibly_production(Some(&expr)),
        "satisfiable on an 'other' platform with test = false"
    );
}

/// `fsutil.rs`'s own `caller_tests` gate -- genuinely test-only, no witness world.
#[test]
fn fsutil_caller_tests_gate_is_not_possibly_production() {
    let expr =
        parse("all(test, any(target_os = \"linux\", target_os = \"macos\"))").expect("parses");
    assert!(!is_possibly_production(Some(&expr)));
}

#[test]
fn unix_alias_is_possibly_production() {
    let expr = parse("unix").expect("parses");
    assert!(is_possibly_production(Some(&expr)));
}

#[test]
fn not_test_is_possibly_production() {
    let expr = parse("not(test)").expect("parses");
    assert!(is_possibly_production(Some(&expr)));
}

#[test]
fn feature_flag_is_possibly_production() {
    let expr = parse("feature = \"test-support\"").expect("parses");
    assert!(is_possibly_production(Some(&expr)));
}

#[test]
fn test_and_feature_flag_is_not_possibly_production() {
    let expr = parse("all(test, feature = \"test-support\")").expect("parses");
    assert!(!is_possibly_production(Some(&expr)));
}

#[test]
fn unparseable_input_is_none() {
    assert_eq!(parse("all(test,"), None);
    assert_eq!(parse(""), None);
}

/// The fail-safe direction: a `cfg` shape this parser cannot classify must never be treated as
/// test-only, since that could silently drop a real production module from the graph.
#[test]
fn unrecognised_shape_defaults_to_possibly_production() {
    assert!(is_possibly_production(None));
}

#[test]
fn trailing_comma_before_close_paren_is_tolerated() {
    let expr = parse("any(test, target_os = \"linux\",)").expect("parses");
    assert!(is_possibly_production(Some(&expr)));
}

#[test]
fn nested_not_any_is_possibly_production_on_an_other_platform() {
    let expr =
        parse("not(any(target_os = \"linux\", target_os = \"macos\", target_os = \"windows\"))")
            .expect("parses");
    assert!(is_possibly_production(Some(&expr)));
}
