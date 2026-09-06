//! Thin binary over [`prikk_corpus::extract_profile`] (RFC 139 handoff §2/§7): reads already-
//! captured `git log`/`git ls-tree` text and a small context file, and writes a profile. Never
//! spawns `git` itself -- the caller captures that text with the commands recorded in the context
//! file (and therefore in the profile's own `provenance.extraction_commands`), so a reader can
//! re-run them and get the same numbers back.
//!
//! ```text
//! extract-profile <log-file> <ls-tree-file> <context-file> [--out <profile-file>]
//! ```
//!
//! `<context-file>` is a small TOML document deserializing as
//! [`prikk_corpus::extract::ExtractionContext`] -- provenance and builder-input fields the log/
//! ls-tree text cannot supply on their own. Without `--out`, the profile is written to stdout.

use std::process::ExitCode;

use prikk_corpus::ExtractionContext;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut positional = Vec::new();
    let mut out_path = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--out" => {
                let value = iter
                    .next()
                    .ok_or_else(|| "--out requires a value".to_string())?;
                if out_path.is_some() {
                    return Err("duplicate --out flag".to_string());
                }
                out_path = Some(value);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown argument: {other}"));
            }
            other => positional.push(other.to_string()),
        }
    }
    let [log_path, ls_tree_path, context_path] = positional.as_slice() else {
        return Err(format!(
            "usage: extract-profile <log-file> <ls-tree-file> <context-file> [--out <profile-file>], \
             got {} positional argument(s)",
            positional.len()
        ));
    };

    let log_text =
        std::fs::read_to_string(log_path).map_err(|err| format!("reading {log_path}: {err}"))?;
    let ls_tree_text = std::fs::read_to_string(ls_tree_path)
        .map_err(|err| format!("reading {ls_tree_path}: {err}"))?;
    let context_text = std::fs::read_to_string(context_path)
        .map_err(|err| format!("reading {context_path}: {err}"))?;
    let context: ExtractionContext =
        toml::from_str(&context_text).map_err(|err| format!("parsing {context_path}: {err}"))?;

    let profile = prikk_corpus::extract_profile(&log_text, &ls_tree_text, context)
        .map_err(|err| err.to_string())?;
    let rendered =
        toml::to_string_pretty(&profile).map_err(|err| format!("rendering profile: {err}"))?;

    match out_path {
        Some(path) => {
            std::fs::write(&path, rendered).map_err(|err| format!("writing {path}: {err}"))?;
        }
        None => print!("{rendered}"),
    }
    Ok(())
}
