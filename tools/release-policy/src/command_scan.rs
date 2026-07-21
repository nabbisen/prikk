//! Bounded command recognition for repository documentation, shell, and YAML text.

mod lexer;
mod prefix;
mod procedure;

use lexer::{commands, logical_lines};
use prefix::{command_head, dynamic, dynamic_cargo, inert_head, opaque_execution};
use procedure::allowed as procedure_command;
pub(crate) use procedure::{shell as scan_shell, yaml as scan_yaml};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Invocation {
    PythonPolicy,
    RustPolicy,
    Publication { phase: String, argv: Vec<String> },
}

#[derive(Debug, Default)]
pub(crate) struct Scan {
    pub(crate) invocations: Vec<Invocation>,
    pub(crate) errors: Vec<&'static str>,
}

pub(crate) fn scan(text: &str) -> Scan {
    scan_mode(text, false)
}

fn scan_mode(text: &str, strict: bool) -> Scan {
    let mut result = Scan::default();
    for line in logical_lines(text) {
        match commands(&line) {
            Ok((commands, backtick)) => {
                if backtick {
                    result.errors.push("unsupported-backtick-substitution");
                }
                for command in commands {
                    scan_command(&command, strict, &mut result);
                }
            }
            Err(error) => result.errors.push(error),
        }
    }
    result
}

pub(crate) fn invocations(text: &str) -> Vec<Invocation> {
    scan(text).invocations
}

fn scan_command(tokens: &[String], strict: bool, result: &mut Scan) {
    let mut found_python = false;
    let mut found_rust = false;
    let mut found_publication = false;
    match command_head(tokens) {
        Ok(Some((index, token))) => {
            if dynamic(token) {
                result.errors.push("unsupported-dynamic-command-head");
            } else if opaque_execution(tokens, index, token) {
                result.errors.push("unsupported-opaque-shell-command");
            } else if !inert_head(token)
                && tokens
                    .get(index + 1..)
                    .unwrap_or_default()
                    .iter()
                    .any(|argument| dynamic(argument))
            {
                result.errors.push("unclassified-dynamic-command");
            }
            if strict && !procedure_command(tokens, index, token) {
                result.errors.push("unclassified-procedure-command");
            }
        }
        Err(error) => result.errors.push(error),
        _ => {}
    }
    for (index, token) in tokens.iter().enumerate() {
        let tail = tokens.get(index + 1..).unwrap_or_default();
        if is_python(token) && python_policy(tail) {
            found_python = true;
            result.invocations.push(Invocation::PythonPolicy);
        }
        if cargo(token) {
            if rust_policy(tail) {
                found_rust = true;
                result.invocations.push(Invocation::RustPolicy);
            }
            if tail.first().is_some_and(|token| dynamic(token)) {
                result.errors.push("unsupported-cargo-subcommand");
            }
            if let Some(phase) = publication(tail) {
                found_publication = true;
                let mut argv = vec!["cargo".to_owned()];
                argv.extend(tail.iter().cloned());
                result.invocations.push(Invocation::Publication {
                    phase: phase.to_owned(),
                    argv,
                });
            }
        }
    }
    if tokens.iter().any(|token| dynamic_cargo(token)) {
        result.errors.push("unsupported-cargo-executable");
    }
    if !found_python && has_python_interpreter(tokens) && has_python_policy_target(tokens) {
        result.errors.push("unsupported-python-invocation");
    }
    if !found_rust && rust_policy(tokens) {
        result.errors.push("unsupported-rust-policy-invocation");
    }
    if !found_publication && has_publication_phase(tokens) {
        result.errors.push("unsupported-publication-invocation");
    }
}

fn is_python(token: &str) -> bool {
    matches!(basename(token), "python" | "python3")
}

fn python_policy(tokens: &[String]) -> bool {
    for token in tokens {
        if token == "--" || token.starts_with('-') {
            continue;
        }
        return normalize_path(token) == "release/check-policy.py";
    }
    false
}

fn has_python_policy_target(tokens: &[String]) -> bool {
    tokens
        .iter()
        .any(|token| normalize_path(token) == "release/check-policy.py")
}

fn has_python_interpreter(tokens: &[String]) -> bool {
    tokens.iter().any(|token| {
        is_python(token)
            || (token.starts_with('$') && token.to_ascii_lowercase().contains("python"))
    })
}

fn cargo(token: &str) -> bool {
    basename(token) == "cargo"
}

fn rust_policy(tokens: &[String]) -> bool {
    let run = tokens.iter().position(|token| token == "run");
    let package = tokens
        .iter()
        .position(|token| token == "prikk-release-policy");
    let separator = tokens.iter().position(|token| token == "--");
    let check = tokens.iter().position(|token| token == "check");
    matches!((run, package, separator, check), (Some(a), Some(b), Some(c), Some(d)) if a < b && b < c && c < d)
}

fn publication(tokens: &[String]) -> Option<&str> {
    tokens
        .first()
        .filter(|token| matches!(token.as_str(), "package" | "publish"))
        .map(String::as_str)
}

fn has_publication_phase(tokens: &[String]) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token.as_str(), "package" | "publish"))
}

fn basename(token: &str) -> &str {
    token.rsplit('/').next().unwrap_or(token)
}

fn normalize_path(token: &str) -> &str {
    token.strip_prefix("./").unwrap_or(token)
}

#[cfg(test)]
#[path = "command_scan/tests.rs"]
mod tests;
