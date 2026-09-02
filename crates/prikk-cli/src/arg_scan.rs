//! RFC 121 §3: one shared mechanism for "reject unknown, refuse duplicate" — every argument
//! parser in this crate builds on this rather than each re-deciding the same absence differently.
//! Before this module existed, every parser had its own hand-rolled `match arg.as_str() { ... }`
//! loop: unknown arguments were refused inconsistently (some parsers, some not at all -- `status`
//! and `init`'s extra positionals had no parser to refuse anything), and every value-carrying flag
//! silently took its *last* occurrence with no refusal at all (`bundle export --ref X --ref Y`
//! exports `Y`, silently -- the handoff's own named victim, but not the only one: every `--ref`,
//! `--limit`, `--format`, and `-m`/`--message` flag in this crate had the identical shape).
//!
//! No new dependency (`prikk-cli` stays at zero third-party crates, `placement.rs` enforces it) --
//! this is a same-crate helper, not a parser library.

use crate::commands::CliError;

/// Mark a value-carrying flag as seen exactly once, refusing a second occurrence. `Option<T>` is
/// the "not yet seen" state regardless of what the flag's own resolved default will be, so a
/// flag with a non-`None` default (e.g. `--ref`'s `heads/main`) still refuses a literal repeat
/// rather than reading the first occurrence as "no value supplied yet."
pub(crate) trait SetOnce<T> {
    fn set_once(&mut self, flag: &str, value: T) -> Result<(), CliError>;
}

impl<T> SetOnce<T> for Option<T> {
    fn set_once(&mut self, flag: &str, value: T) -> Result<(), CliError> {
        if self.is_some() {
            return Err(CliError::Usage(format!("duplicate {flag} flag")));
        }
        *self = Some(value);
        Ok(())
    }
}

/// As [`SetOnce`], for a boolean presence flag (`--yes`, `--stop-on-first-error`, ...) that carries
/// no value of its own -- for these, "seen" and "value" are the same fact, so the flag's own `bool`
/// doubles as the seen-tracker.
pub(crate) fn mark_seen(seen: &mut bool, flag: &str) -> Result<(), CliError> {
    if *seen {
        return Err(CliError::Usage(format!("duplicate {flag} flag")));
    }
    *seen = true;
    Ok(())
}

/// Consume the next token as a flag's required value, refusing if the flag was the last argument.
pub(crate) fn flag_value(
    iter: &mut std::vec::IntoIter<String>,
    description: &str,
) -> Result<String, CliError> {
    iter.next()
        .ok_or_else(|| CliError::Usage(format!("{description} requires a value")))
}

/// An unrecognized argument -- one place to change the wording, and every call site gets
/// `CliError::Usage` rather than each site choosing (or forgetting to choose) that.
pub(crate) fn unknown_argument(command: &str, argument: &str) -> CliError {
    CliError::Usage(format!("unknown {command} argument: {argument}"))
}

#[cfg(test)]
mod tests;
