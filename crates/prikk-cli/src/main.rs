#![forbid(unsafe_code)]

//! PRIKK command-line entry point.
//!
//! PR-013 exposes minimal repository layout commands, active WAL status, an empty-commit scaffold,
//! a local no-audit seal scaffold, ref pointer counts, deeper repository verification, and doctor
//! diagnostics with opt-in safe WAL tail and missing-ref-pointer repair. Real diff capture, patch application, audit
//! plugins, and sync remain later increments.

use std::path::PathBuf;
use std::process::ExitCode;

mod seal;

use prikk_hash::sha256;
use prikk_object::{
    CanonicalEncode, ObjectEnvelope, ObjectType, OperationCondition, OperationConditionEntry,
    PatchPayload, Signature, SignatureAlgorithm, SignerRole,
};
use prikk_store::{
    doctor_repository, repair_repository, verify_repository, ActiveSession, DoctorRepairOptions,
    DoctorSeverity, RefStore, RepositoryLayout, Wal,
};

const VERSION: &str = "0.1.0-pr013";

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
        Some("commit") => {
            let message = parse_empty_commit_message(args.collect())?;
            let root = current_dir()?;
            let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
            let envelope = empty_patch_envelope(&message)?;
            let patch_id = envelope.object_id();
            let session = ActiveSession::new(layout);
            let result = session.append_patch(&envelope).map_err(|err| err.to_string())?;
            println!("recorded empty patch in active WAL");
            println!("patch id: {patch_id}");
            println!("WAL sequence: {}", result.wal_sequence);
            println!("note: real diff capture and seal remain later PRs");
            Ok(())
        }
        Some("seal") => {
            let root = current_dir()?;
            let result = seal::run_seal(root, args.collect())?;
            println!("sealed active WAL into block");
            println!("patches: {}", result.patch_count);
            println!("block id: {}", result.block_id);
            println!("heads/main RefState: {}", result.ref_state_id);
            println!("note: audit plugins and real worktree materialization remain later PRs");
            Ok(())
        }
        Some("status") => {
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
            println!("status: patch algebra, plugins, and sync not implemented in PR-013");
            Ok(())
        }
        Some("verify") => {
            let root = optional_path_or_current(args.next())?;
            let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
            let report = verify_repository(&layout).map_err(|err| err.to_string())?;
            print_verify_report(&layout, &report);
            Ok(())
        }
        Some("doctor") => {
            let doctor_args = parse_doctor_args(args.collect())?;
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
                    repair.wal_repair.truncated_bytes,
                    repair.wal_repair.preserved_records
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
        Some(other) => Err(format!("unknown command: {other}")),
    }
}


struct DoctorArgs {
    root: PathBuf,
    repair_wal_tail: bool,
    repair_main_ref: bool,
}

fn parse_doctor_args(args: Vec<String>) -> std::result::Result<DoctorArgs, String> {
    let mut repair_wal_tail = false;
    let mut repair_main_ref = false;
    let mut path = None;
    for arg in args {
        match arg.as_str() {
            "--repair-wal-tail" => repair_wal_tail = true,
            "--repair-main-ref" => repair_main_ref = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown doctor argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("doctor accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    Ok(DoctorArgs {
        root: optional_path_or_current(path)?,
        repair_wal_tail,
        repair_main_ref,
    })
}

fn print_doctor_report(layout: &RepositoryLayout, report: &prikk_store::DoctorReport) {
    if let Some(verification) = &report.verification {
        print_verify_report(layout, verification);
    }
    for issue in &report.issues {
        println!("{} [{}]: {}", issue.severity.as_str(), issue.code, issue.message);
        println!("  recommendation: {}", issue.recommendation);
    }
    println!(
        "issue summary: errors={}, warnings={}, info={}",
        report.count_by_severity(DoctorSeverity::Error),
        report.count_by_severity(DoctorSeverity::Warning),
        report.count_by_severity(DoctorSeverity::Info)
    );
}

fn parse_empty_commit_message(args: Vec<String>) -> std::result::Result<String, String> {
    let mut allow_empty = false;
    let mut message = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--allow-empty" => allow_empty = true,
            "-m" | "--message" => {
                let Some(value) = iter.next() else {
                    return Err("commit message option requires a value".to_string());
                };
                message = Some(value);
            }
            other => return Err(format!("unknown commit argument: {other}")),
        }
    }
    if !allow_empty {
        return Err("PR-013 supports only `prikk commit --allow-empty -m <message>`".to_string());
    }
    let Some(message) = message else {
        return Err("empty commit requires -m <message>".to_string());
    };
    if message.trim().is_empty() {
        return Err("commit message must not be empty".to_string());
    }
    Ok(message)
}

fn empty_patch_envelope(message: &str) -> std::result::Result<ObjectEnvelope, String> {
    let message_hash = sha256(message.as_bytes());
    let payload = PatchPayload {
        operations: Vec::new(),
        parent_patch_ids: Vec::new(),
        intent: None,
        preconditions: vec![OperationConditionEntry {
            key: "prikk.dev.empty-commit-message-sha256".to_string(),
            value: OperationCondition::OldContentHash(message_hash.to_vec()),
        }],
    };
    let payload_bytes = payload.to_canonical_bytes().map_err(|err| err.to_string())?;
    let mut envelope = ObjectEnvelope::unsigned(ObjectType::Patch, 1, payload_bytes);
    envelope.add_signature(dev_author_signature(message)).map_err(|err| err.to_string())?;
    Ok(envelope)
}

fn dev_author_signature(message: &str) -> Signature {
    let mut signature_preimage = Vec::new();
    signature_preimage.extend_from_slice(b"prikk.dev.placeholder-signature.v1");
    signature_preimage.extend_from_slice(message.as_bytes());
    Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        key_id: "dev-placeholder-author".to_string(),
        signature_bytes: sha256(&signature_preimage).to_vec(),
        created_at: 0,
        signer_role: SignerRole::Author,
    }
}

fn optional_path_or_current(path: Option<String>) -> std::result::Result<PathBuf, String> {
    match path {
        Some(path) => Ok(PathBuf::from(path)),
        None => current_dir(),
    }
}

fn print_verify_report(layout: &RepositoryLayout, report: &prikk_store::RepositoryVerification) {
    println!("verified repository: {}", layout.prikk_dir().display());
    println!("checked objects: {}", report.checked_objects);
    println!("checked blocks: {}", report.checked_blocks);
    println!("checked WAL records: {}", report.checked_wal_records);
    println!("persisted WAL patches: {}", report.persisted_wal_patches);
    println!("checked refs: {}", report.checked_refs);
    println!("checked ref-log records: {}", report.checked_ref_log_records);
    println!("trailing partial WAL bytes: {}", report.trailing_partial_wal_bytes);
    if report.has_trailing_partial_wal() {
        println!("warning: active WAL contains an incomplete trailing record");
    }
}

fn current_dir() -> std::result::Result<PathBuf, String> {
    std::env::current_dir().map_err(|err| err.to_string())
}

fn print_help() {
    println!("prikk {VERSION}");
    println!();
    println!("Usage:");
    println!("  prikk init [path]                         Create a .prikk repository layout");
    println!("  prikk commit --allow-empty -m <message>   Append an empty patch to the active WAL");
    println!("  prikk status                              Check repository and active WAL status");
    println!("  prikk seal --allow-no-audit              Seal active WAL into heads/main");
    println!("  prikk verify [path]                       Verify objects and WAL records");
    println!("  prikk doctor [path]                       Run health diagnostics");
    println!("  prikk doctor [path] --repair-wal-tail     Truncate incomplete trailing WAL bytes");
    println!("  prikk doctor [path] --repair-main-ref     Reconstruct a missing heads/main pointer");
    println!("  prikk --version                           Print version");
}
