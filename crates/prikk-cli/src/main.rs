#![forbid(unsafe_code)]

//! PRIKK command-line entry point.
//!
//! PR-007 exposes minimal repository layout commands, active WAL status, ref pointer counts, and
//! read-only repository verification. WAL/ref support is present in the storage crate but not yet
//! exposed as an end-user commit or seal workflow.

use std::path::PathBuf;
use std::process::ExitCode;

use prikk_store::{verify_repository, RefStore, RepositoryLayout, Wal};

const VERSION: &str = "0.1.0-pr007";

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
            let root = match args.next() {
                Some(path) => PathBuf::from(path),
                None => current_dir()?,
            };
            RepositoryLayout::init(root.clone()).map_err(|err| err.to_string())?;
            println!("initialized PRIKK repository at {}", root.join(".prikk").display());
            Ok(())
        }
        Some("status") => {
            let root = current_dir()?;
            let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
            let wal = Wal::new(layout.default_queue_wal_path());
            let replay = wal.replay().map_err(|err| err.to_string())?;
            println!("prikk repository: {}", layout.prikk_dir().display());
            let ref_store = RefStore::new(layout.clone());
            let main_ref = ref_store.read_current_ref_state_id("heads/main").map_err(|err| err.to_string())?;
            println!("active WAL records: {}", replay.records.len());
            println!("trailing partial WAL bytes: {}", replay.trailing_partial_bytes);
            match main_ref {
                Some(id) => println!("heads/main RefState: {id}"),
                None => println!("heads/main RefState: <not published>"),
            }
            println!("status: commit, seal, patch algebra, plugins, and sync not implemented in PR-007");
            Ok(())
        }
        Some("verify") => {
            let root = match args.next() {
                Some(path) => PathBuf::from(path),
                None => current_dir()?,
            };
            let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
            let report = verify_repository(&layout).map_err(|err| err.to_string())?;
            println!("verified repository: {}", layout.prikk_dir().display());
            println!("checked objects: {}", report.checked_objects);
            println!("checked WAL records: {}", report.checked_wal_records);
            println!("checked refs: {}", report.checked_refs);
            println!("checked ref-log records: {}", report.checked_ref_log_records);
            println!("trailing partial WAL bytes: {}", report.trailing_partial_wal_bytes);
            if report.has_trailing_partial_wal() {
                println!("warning: active WAL contains an incomplete trailing record");
            }
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
    println!("  prikk verify [path]   Verify persisted objects and active WAL records");
    println!("  prikk --version       Print version");
}
