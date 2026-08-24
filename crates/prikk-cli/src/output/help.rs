//! Top-level help output.
//!
//! RFC 118 stage 1: this holds no literal command text of its own. It renders
//! [`crate::commands::COMMANDS`] -- the single declaration of the command surface dispatch also
//! derives from -- plus the fixed header/`Usage:` lines and the `--version` meta line, which is a
//! meta-arm (`main.rs::run`'s own match still handles `--help`/`-h`/`--version`/`-V` directly), not
//! a registry entry.

/// Print top-level help.
pub(crate) fn print_help(version: &str) {
    println!("prikk {version}");
    println!();
    println!("Usage:");
    for command in crate::commands::COMMANDS {
        for line in command.help_lines {
            println!("{line}");
        }
    }
    println!("  prikk --version                           Print version");
}
