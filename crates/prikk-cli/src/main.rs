#![forbid(unsafe_code)]

//! PRIKK command-line entry point.
//!
//! PR-021 exposes minimal repository layout commands, active WAL status, empty and snapshot-baseline
//! worktree commit scaffolds, supported file-level patch replay planning and materialization, a
//! local no-audit seal scaffold, read-only history inspection, checkout and snapshot-manifest
//! planning, conservative snapshot materialization, read-only worktree status,
//! deeper repository verification, and doctor diagnostics with opt-in repairs. Patch application,
//! audit plugins, and sync remain later increments.

use std::path::PathBuf;
use std::process::ExitCode;

mod args;
mod commit;
mod output;
mod seal;

use args::{
    current_dir, optional_path_or_current, parse_checkout_args, parse_commit_args, parse_doctor_args,
    parse_log_args, parse_worktree_status_args, CheckoutMode, CommitMode,
};
use commit::empty_patch_envelope;
use output::{
    print_checkout_plan, print_doctor_report, print_help, print_history,
    print_patch_materialization_report, print_patch_replay_plan, print_snapshot_checkout_plan,
    print_snapshot_materialization_report, print_verify_report, print_worktree_status,
};
use prikk_store::{
    commit_worktree_changes, doctor_repository, load_ref_history, materialize_patch_checkout,
    materialize_snapshot_checkout, prepare_checkout_plan, prepare_patch_replay_plan,
    prepare_snapshot_checkout_plan, repair_repository, verify_repository, worktree_status,
    ActiveSession, DoctorRepairOptions, RefStore, RepositoryLayout, Wal,
};

const VERSION: &str = "0.1.0-pr021";

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
            print_help(VERSION);
            Ok(())
        }
        Some("--version") | Some("-V") => {
            println!("prikk {VERSION}");
            Ok(())
        }
        Some("init") => run_init(args.next()),
        Some("commit") => run_commit(args.collect()),
        Some("seal") => run_seal(args.collect()),
        Some("status") => run_status(),
        Some("log") => run_log(args.collect()),
        Some("checkout") => run_checkout(args.collect()),
        Some("worktree-status") => run_worktree_status(args.collect()),
        Some("verify") => run_verify(args.next()),
        Some("doctor") => run_doctor(args.collect()),
        Some(other) => Err(format!("unknown command: {other}")),
    }
}

fn run_init(path: Option<String>) -> std::result::Result<(), String> {
    let root = match path {
        Some(path) => PathBuf::from(path),
        None => current_dir()?,
    };
    RepositoryLayout::init(root.clone()).map_err(|err| err.to_string())?;
    println!("initialized PRIKK repository at {}", root.join(".prikk").display());
    Ok(())
}

fn run_commit(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_commit_args(args)?;
    let root = current_dir()?;
    let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
    match args.mode {
        CommitMode::AllowEmpty => {
            let envelope = empty_patch_envelope(&args.message)?;
            let patch_id = envelope.object_id();
            let session = ActiveSession::new(layout);
            let result = session.append_patch(&envelope).map_err(|err| err.to_string())?;
            println!("recorded empty patch in active WAL");
            println!("patch id: {patch_id}");
            println!("WAL sequence: {}", result.wal_sequence);
        }
        CommitMode::FromWorktree => {
            let report = commit_worktree_changes(&layout, &args.ref_name, &args.message)
                .map_err(|err| err.to_string())?;
            println!("recorded worktree patch in active WAL");
            println!("baseline ref: {}", report.ref_name);
            println!("patch id: {}", report.patch_id);
            println!("WAL sequence: {}", report.wal_sequence);
            println!("operations: {}", report.operation_count);
            println!("referenced blobs: {}", report.referenced_blob_count);
            for change in &report.changes {
                println!("  {} {}", change.operation.as_str(), change.path);
            }
        }
    }
    println!("note: patch replay/algebra, rename detection, audit plugins, and sync remain later PRs");
    Ok(())
}

fn run_seal(args: Vec<String>) -> std::result::Result<(), String> {
    let root = current_dir()?;
    let result = seal::run_seal(root, args)?;
    println!("sealed active WAL into block");
    println!("patches: {}", result.patch_count);
    println!("block id: {}", result.block_id);
    println!("heads/main RefState: {}", result.ref_state_id);
    println!("note: audit plugins and patch-based worktree materialization remain later PRs");
    Ok(())
}

fn run_status() -> std::result::Result<(), String> {
    let root = current_dir()?;
    let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
    let wal = Wal::new(layout.default_queue_wal_path());
    let replay = wal.replay().map_err(|err| err.to_string())?;
    println!("prikk repository: {}", layout.prikk_dir().display());
    let ref_store = RefStore::new(layout.clone());
    let main_ref = ref_store
        .read_current_ref_state_id("heads/main")
        .map_err(|err| err.to_string())?;
    println!("active WAL records: {}", replay.records.len());
    println!("trailing partial WAL bytes: {}", replay.trailing_partial_bytes);
    match main_ref {
        Some(id) => println!("heads/main RefState: {id}"),
        None => println!("heads/main RefState: <not published>"),
    }
    println!(
        "status: patch algebra, patch-based worktree materialization, plugins, and sync not \
         implemented in PR-021"
    );
    Ok(())
}

fn run_log(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_log_args(args)?;
    let layout = RepositoryLayout::open(args.root).map_err(|err| err.to_string())?;
    let history = load_ref_history(&layout, &args.ref_name, args.limit)
        .map_err(|err| err.to_string())?;
    print_history(&layout, &history);
    Ok(())
}

fn run_checkout(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_checkout_args(args)?;
    let layout = RepositoryLayout::open(args.root).map_err(|err| err.to_string())?;
    match args.mode {
        CheckoutMode::PlanOnly => {
            let plan = prepare_checkout_plan(&layout, &args.ref_name)
                .map_err(|err| err.to_string())?;
            print_checkout_plan(&layout, &plan);
        }
        CheckoutMode::SnapshotPlan => {
            let plan = prepare_snapshot_checkout_plan(&layout, &args.ref_name)
                .map_err(|err| err.to_string())?;
            print_snapshot_checkout_plan(&layout, &plan);
        }
        CheckoutMode::SnapshotMaterialize => {
            let report = materialize_snapshot_checkout(&layout, &args.ref_name)
                .map_err(|err| err.to_string())?;
            print_snapshot_materialization_report(&layout, &report);
        }
        CheckoutMode::PatchPlan => {
            let plan = prepare_patch_replay_plan(&layout, &args.ref_name)
                .map_err(|err| err.to_string())?;
            print_patch_replay_plan(&layout, &plan);
        }
        CheckoutMode::PatchMaterialize => {
            let report = materialize_patch_checkout(&layout, &args.ref_name)
                .map_err(|err| err.to_string())?;
            print_patch_materialization_report(&layout, &report);
        }
    }
    Ok(())
}

fn run_worktree_status(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_worktree_status_args(args)?;
    let layout = RepositoryLayout::open(args.root).map_err(|err| err.to_string())?;
    let report = worktree_status(&layout, &args.ref_name).map_err(|err| err.to_string())?;
    print_worktree_status(&layout, &report);
    if report.is_clean() {
        Ok(())
    } else {
        Err("worktree has snapshot-baseline changes".to_string())
    }
}

fn run_verify(path: Option<String>) -> std::result::Result<(), String> {
    let root = optional_path_or_current(path)?;
    let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
    let report = verify_repository(&layout).map_err(|err| err.to_string())?;
    print_verify_report(&layout, &report);
    Ok(())
}

fn run_doctor(args: Vec<String>) -> std::result::Result<(), String> {
    let doctor_args = parse_doctor_args(args)?;
    let layout = RepositoryLayout::open(doctor_args.root).map_err(|err| err.to_string())?;
    if doctor_args.repair_wal_tail || doctor_args.repair_main_ref {
        let options = DoctorRepairOptions {
            truncate_wal_tail: doctor_args.repair_wal_tail,
            reconstruct_main_ref: doctor_args.repair_main_ref,
        };
        let repair = repair_repository(&layout, options).map_err(|err| err.to_string())?;
        println!("doctor repository: {}", layout.prikk_dir().display());
        println!(
            "repair: truncated {} trailing WAL byte(s); preserved {} record(s)",
            repair.wal_repair.truncated_bytes, repair.wal_repair.preserved_records
        );
        if let Some(ref_repair) = &repair.ref_repair {
            println!(
                "repair: {} heads/main pointer for RefState {}",
                if ref_repair.wrote_pointer { "reconstructed" } else { "kept existing" },
                ref_repair.ref_state_id
            );
        }
        print_doctor_report(&layout, &repair.after);
        if repair.after.is_healthy() {
            Ok(())
        } else {
            Err("doctor repair finished but repository health errors remain".to_string())
        }
    } else {
        let report = doctor_repository(&layout);
        println!("doctor repository: {}", layout.prikk_dir().display());
        print_doctor_report(&layout, &report);
        if report.is_healthy() {
            Ok(())
        } else {
            Err("doctor found repository health errors".to_string())
        }
    }
}
