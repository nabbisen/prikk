//! RFC 121 §2.5 (command-discovery-handoff-v1.md control 5): a static guard that every registry
//! entry actually has help text to route to. `main.rs::run`'s `--help`/`-h` routing prints
//! `command.help_lines` verbatim (`output::print_command_help`) -- it cannot detect an empty slice
//! at runtime and refuse, since an empty slice is a perfectly valid (if useless) value for the
//! field. This is the check that catches a future `Command` added to [`super::COMMANDS`] with
//! `help_lines: &[]`, so it fails a test rather than silently shipping a command whose `--help`
//! prints nothing.
//!
//! End-to-end routing itself (`prikk <command> --help` actually reaching this text, for a plain, a
//! multi-word, and a subcommand-dispatching command) is driven against the real binary in
//! `tests/rfc121_command_help.rs` -- this file only guards the data `COMMANDS` itself carries.

use super::COMMANDS;

#[test]
fn every_command_has_non_empty_help_lines() {
    for command in COMMANDS {
        assert!(
            !command.help_lines.is_empty(),
            "`{}` has no help_lines -- `prikk {} --help` would print nothing",
            command.name,
            command.name
        );
    }
}

/// At least one line per command must actually show `prikk <name>` usage, not only a wrapped
/// continuation or a `note:` aside -- so a command whose `help_lines` accidentally holds only
/// commentary (no usage line at all) is still caught, even though it is technically non-empty.
#[test]
fn every_command_has_at_least_one_usage_line_naming_itself() {
    let prefix_for = |name: &str| format!("  prikk {name}");
    for command in COMMANDS {
        let prefix = prefix_for(command.name);
        assert!(
            command
                .help_lines
                .iter()
                .any(|line| line.starts_with(&prefix)),
            "`{}`'s help_lines has no line starting with {prefix:?}",
            command.name
        );
    }
}
