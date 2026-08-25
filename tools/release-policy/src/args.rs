use std::path::PathBuf;

use crate::boundary;
use crate::error::{Error, Result};
use crate::oracle;
use crate::policy;
use crate::reference;
use crate::release_notes;

pub(crate) fn run(arguments: Vec<String>) -> Result<()> {
    let (command, rest) = arguments.split_first().ok_or_else(|| Error::new(usage()))?;
    let root = repository_root()?;
    match command.as_str() {
        "check" if rest.is_empty() => policy::run_check(&root),
        "oracle-check" => oracle_check(&root, rest),
        "boundary-check" => boundary_check(&root, rest),
        "reference-check" => reference_check(&root, rest),
        "release-notes" => release_notes_command(&root, rest),
        "-h" | "--help" | "help" if rest.is_empty() => {
            println!("{}", usage());
            Ok(())
        }
        _ => Err(Error::new(usage())),
    }
}

fn oracle_check(root: &std::path::Path, arguments: &[String]) -> Result<()> {
    let self_test = parse_json_mode(arguments, true)?;
    let report = oracle::verify_repository(root, self_test)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report.valid {
        Ok(())
    } else {
        Err(Error::new("oracle verification failed"))
    }
}

fn boundary_check(root: &std::path::Path, arguments: &[String]) -> Result<()> {
    parse_json_mode(arguments, false)?;
    let report = boundary::run(root)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report.valid {
        Ok(())
    } else {
        Err(Error::new("workspace boundary verification failed"))
    }
}

fn reference_check(root: &std::path::Path, arguments: &[String]) -> Result<()> {
    parse_json_mode(arguments, false)?;
    let report = reference::run(root)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report.valid {
        Ok(())
    } else {
        Err(Error::new("release-policy reference verification failed"))
    }
}

fn release_notes_command(root: &std::path::Path, arguments: &[String]) -> Result<()> {
    let [tag, dist_dir] = arguments else {
        return Err(Error::new(usage()));
    };
    let notes = release_notes::assemble(root, tag, std::path::Path::new(dist_dir))?;
    print!("{notes}");
    Ok(())
}

fn parse_json_mode(arguments: &[String], self_test_allowed: bool) -> Result<bool> {
    let mut self_test = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments.get(index).map(String::as_str) {
            Some("--format") if arguments.get(index + 1).map(String::as_str) == Some("json") => {
                index += 2;
            }
            Some("--self-test") if self_test_allowed => {
                self_test = true;
                index += 1;
            }
            _ => return Err(Error::new(usage())),
        }
    }
    Ok(self_test)
}

fn repository_root() -> Result<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or_else(|| Error::new("tool manifest is not below the repository root"))?
        .to_path_buf();
    Ok(root)
}

fn usage() -> &'static str {
    "usage: prikk-release-policy <check|oracle-check|boundary-check|reference-check> [--format json] [--self-test]\n       prikk-release-policy release-notes <tag> <dist-dir>"
}
