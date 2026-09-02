//! RFC 121 §6a's ruled exit-code contract: `CliError`'s own mapping, unit-tested directly; the
//! real end-to-end exit codes (0/1/2 on real commands) are covered by
//! `tests/rfc121_exit_code_contract.rs`, which drives the compiled binary -- `main()`'s own
//! `ExitCode::from(err.exit_code())` is a one-line function this crate cannot invoke a process
//! from, so the two live at different levels rather than duplicating each other.

use super::CliError;

#[test]
fn usage_maps_to_exit_code_two() {
    assert_eq!(CliError::Usage("bad flag".to_string()).exit_code(), 2);
}

#[test]
fn failure_maps_to_exit_code_one() {
    assert_eq!(CliError::Failure("did not work".to_string()).exit_code(), 1);
}

#[test]
fn from_string_defaults_to_failure_not_usage() {
    // Every error this crate produced before this contract existed was a bare `String`, and every
    // one of them exited `1` -- `From<String>` must keep that default so changing `Command.run`'s
    // return type is pure plumbing, not a silent reclassification of any existing error.
    let error: CliError = "something went wrong".to_string().into();
    assert_eq!(error.exit_code(), 1);
    assert!(matches!(error, CliError::Failure(_)));
}

#[test]
fn message_is_preserved_through_the_conversion() {
    let error: CliError = "exact wording".to_string().into();
    assert_eq!(error.message(), "exact wording");
}
