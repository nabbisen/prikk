#![forbid(unsafe_code)]

//! PRIKK command-line entry point.
//!
//! PR-003 exposes a minimal `init` command for repository layout creation. WAL, refs, patch
//! algebra, plugins, and synchronization remain separate implementation increments.

use std::path::PathBuf;
use std::process::ExitCode;

use prikk_store::RepositoryLayout;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::from(1)
        }
    }
}

fn run() -> std::result::Result<(), String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None | Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some("--version") | Some("-V") => {
            println!("prikk 0.1.0-pr003");
            Ok(())
        }
        Some("init") => {
            let root = args.next().map_or_else(current_dir, PathBuf::from)?;
            RepositoryLayout::init(root.clone()).map_err(|err| err.to_string())?;
            println!("initialized PRIKK repository at {}", root.join(".prikk").display());
            Ok(())
        }
        Some("status") => {
            let root = current_dir()?;
            let layout = RepositoryLayout::open(root.clone()).map_err(|err| err.to_string())?;
            println!("prikk repository: {}", layout.prikk_dir().display());
            println!("status: repository layout present; WAL/refs not implemented in PR-003");
            Ok(())
        }
        Some(other) => Err(format!("unknown command: {other}")),
    }
}

fn current_dir() -> std::result::Result<PathBuf, String> {
    std::env::current_dir().map_err(|err| err.to_string())
}

fn print_help() {
    println!("prikk 0.1.0-pr003");
    println!();
    println!("Usage:");
    println!("  prikk init [path]     Create a .prikk repository layout");
    println!("  prikk status          Check that a repository layout exists");
    println!("  prikk --version       Print version");
}
