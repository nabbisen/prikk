//! Unit coverage for the pure parsing helpers -- the crate-wide "reject unknown, refuse duplicate"
//! mechanism every `parse_*_args` function now routes through. End-to-end refusal through the
//! compiled binary (the `bundle export --ref` victim, `commit` with no `-m`, `status --nonsense`,
//! ...) is covered by `tests/rfc121_argument_hygiene.rs`; this file exercises the helpers directly
//! so a regression here is caught at the smallest possible unit.

#![allow(clippy::unwrap_used)]

use super::{SetOnce, flag_value, mark_seen, unknown_argument};
use crate::commands::CliError;

#[test]
fn set_once_accepts_the_first_value() {
    let mut slot: Option<String> = None;
    slot.set_once("--ref", "heads/main".to_string()).unwrap();
    assert_eq!(slot.as_deref(), Some("heads/main"));
}

#[test]
fn set_once_refuses_a_second_value() {
    let mut slot: Option<String> = None;
    slot.set_once("--ref", "heads/main".to_string()).unwrap();
    let err = slot
        .set_once("--ref", "heads/other".to_string())
        .unwrap_err();
    assert!(
        matches!(err, CliError::Usage(_)),
        "duplicate flag must be a usage error"
    );
    assert_eq!(err.message(), "duplicate --ref flag");
    // The first value is untouched -- a refused duplicate never silently becomes the new value.
    assert_eq!(slot.as_deref(), Some("heads/main"));
}

#[test]
fn mark_seen_accepts_the_first_occurrence() {
    let mut seen = false;
    mark_seen(&mut seen, "--force").unwrap();
    assert!(seen);
}

#[test]
fn mark_seen_refuses_a_second_occurrence() {
    let mut seen = false;
    mark_seen(&mut seen, "--force").unwrap();
    let err = mark_seen(&mut seen, "--force").unwrap_err();
    assert!(matches!(err, CliError::Usage(_)));
    assert_eq!(err.message(), "duplicate --force flag");
}

#[test]
fn flag_value_returns_the_next_token() {
    let mut iter = vec!["heads/main".to_string()].into_iter();
    assert_eq!(flag_value(&mut iter, "--ref").unwrap(), "heads/main");
}

#[test]
fn flag_value_refuses_a_flag_with_nothing_after_it() {
    let mut iter = Vec::<String>::new().into_iter();
    let err = flag_value(&mut iter, "--ref").unwrap_err();
    assert!(matches!(err, CliError::Usage(_)));
    assert_eq!(err.message(), "--ref requires a value");
}

#[test]
fn unknown_argument_is_a_usage_error_naming_the_command_and_the_argument() {
    let err = unknown_argument("verify", "--nonsense");
    assert!(matches!(err, CliError::Usage(_)));
    assert_eq!(err.message(), "unknown verify argument: --nonsense");
}
