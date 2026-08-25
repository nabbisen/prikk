#![allow(clippy::unwrap_used)]

use super::escape_json_string;

/// Windows hostile-test fix handoff Sec 2: the hostile-input proof used to live only in an
/// integration test that planted a directory whose name carried these exact bytes -- a double
/// quote, a backslash, and every C0 control are forbidden in a Win32 filename, so the directory
/// could never be created on that platform and the control never reached the escaper at all.
/// `escape_json_string` is a pure `&str -> String` function; proving it here needs no filesystem
/// and accepts input no filesystem would ever allow.
#[test]
fn hostile_input_is_escaped_correctly() {
    let hostile = "quote\"back\\slash\nnewline\ttab\u{1}control";
    let escaped = escape_json_string(hostile);
    assert_eq!(
        escaped,
        r#""quote\"back\\slash\nnewline\ttab\u0001control""#
    );
}
