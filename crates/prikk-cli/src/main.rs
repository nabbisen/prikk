#![forbid(unsafe_code)]

//! PRIKK command-line entry point.
//!
//! PR-004 exposes minimal repository layout commands. WAL support is present in the storage crate
//! but not yet exposed as an end-user commit workflow.

use std::path::PathBuf;
use std::process::ExitCode;

use prikk_store::{RepositoryLayout, Wal};

const VERSION: &str = "0.1.0-pr004";

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
            println!("prikk {VERSION}");
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
            let wal = Wal::new(layout.default_queue_wal_path());
            let replay = wal.replay().map_err(|err| err.to_string())?;
            println!("prikk repository: {}", layout.prikk_dir().display());
            println!("active WAL records: {}", replay.records.len());
            println!("trailing partial WAL bytes: {}", replay.trailing_partial_bytes);
            println!("status: refs, patch algebra, plugins, and sync not implemented in PR-004");
            Ok(())
        }
        Some(other) => Err(format!("unknown command: {other}")),
    }
}

fn current_dir() -> std::result::Result<PathBuf, String> {
    std::env::current_dir().map_err(|err| err.to_string())
}

fn print_help() {
    println!("prikk {VERSION}");
    println!();
    println!("Usage:");
    println!("  prikk init [path]     Create a .prikk repository layout");
    println!("  prikk status          Check repository layout and active WAL status");
    println!("  prikk --version       Print version");
}
