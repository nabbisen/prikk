#![forbid(unsafe_code)]

//! PRIKK command-line entry point.
//!
//! PR-016 exposes minimal repository layout commands, active WAL status, an empty-commit scaffold,
//! a local no-audit seal scaffold, read-only history inspection, checkout and snapshot-manifest
//! planning, deeper repository verification, and doctor diagnostics with opt-in safe WAL tail and
//! missing-ref-pointer repair. Real diff capture, patch application, audit plugins, and sync remain
//! later increments.

use std::path::PathBuf;
use std::process::ExitCode;

mod seal;

use prikk_hash::sha256;
use prikk_object::{
    CanonicalEncode, ObjectEnvelope, ObjectType, OperationCondition, OperationConditionEntry,
    PatchPayload, Signature, SignatureAlgorithm, SignerRole,
};
use prikk_store::{
    doctor_repository, load_ref_history, prepare_checkout_plan, prepare_snapshot_checkout_plan,
    repair_repository, verify_repository, ActiveSession, CheckoutMaterialization, CheckoutPlan,
    DoctorRepairOptions, DoctorSeverity, RefHistory, RefStore, RepositoryLayout, Wal,
    DEFAULT_CHECKOUT_REF, DEFAULT_HISTORY_LIMIT,
};

const VERSION: &str = "0.1.0-pr016";

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
            println!(
                "status: patch algebra, worktree materialization, plugins, and sync not implemented in PR-016"
            );
            Ok(())
        }
        Some("log") => {
            let args = parse_log_args(args.collect())?;
            let layout = RepositoryLayout::open(args.root).map_err(|err| err.to_string())?;
            let history = load_ref_history(&layout, &args.ref_name, args.limit)
                .map_err(|err| err.to_string())?;
            print_history(&layout, &history);
            Ok(())
        }
        Some("checkout") => {
            let args = parse_checkout_args(args.collect())?;
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
            }
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


struct LogArgs {
    root: PathBuf,
    ref_name: String,
    limit: usize,
}

fn parse_log_args(args: Vec<String>) -> std::result::Result<LogArgs, String> {
    let mut path = None;
    let mut ref_name = "heads/main".to_string();
    let mut limit = DEFAULT_HISTORY_LIMIT;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("log --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("log --ref must not be empty".to_string());
                }
                ref_name = value;
            }
            "--limit" => {
                let Some(value) = iter.next() else {
                    return Err("log --limit requires a value".to_string());
                };
                limit = value
                    .parse::<usize>()
                    .map_err(|_| "log --limit must be a non-negative integer".to_string())?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown log argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("log accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    Ok(LogArgs { root: optional_path_or_current(path)?, ref_name, limit })
}


struct CheckoutArgs {
    root: PathBuf,
    ref_name: String,
    mode: CheckoutMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckoutMode {
    PlanOnly,
    SnapshotPlan,
}

fn parse_checkout_args(args: Vec<String>) -> std::result::Result<CheckoutArgs, String> {
    let mut mode = None;
    let mut path = None;
    let mut ref_name = DEFAULT_CHECKOUT_REF.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--plan-only" => set_checkout_mode(&mut mode, CheckoutMode::PlanOnly)?,
            "--snapshot-plan" => set_checkout_mode(&mut mode, CheckoutMode::SnapshotPlan)?,
            "--ref" => {
                let Some(value) = iter.next() else {
                    return Err("checkout --ref requires a value".to_string());
                };
                if value.trim().is_empty() {
                    return Err("checkout --ref must not be empty".to_string());
                }
                ref_name = value;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown checkout argument: {other}"));
            }
            _ => {
                if path.is_some() {
                    return Err("checkout accepts at most one path".to_string());
                }
                path = Some(arg);
            }
        }
    }
    let Some(mode) = mode else {
        return Err(
            "PR-016 supports `prikk checkout --plan-only` or `prikk checkout --snapshot-plan`"
                .to_string(),
        );
    };
    Ok(CheckoutArgs { root: optional_path_or_current(path)?, ref_name, mode })
}

fn set_checkout_mode(
    mode: &mut Option<CheckoutMode>,
    next: CheckoutMode,
) -> std::result::Result<(), String> {
    if mode.is_some() {
        return Err("checkout accepts only one of --plan-only or --snapshot-plan".to_string());
    }
    *mode = Some(next);
    Ok(())
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


fn print_checkout_plan(layout: &RepositoryLayout, plan: &CheckoutPlan) {
    println!("checkout plan repository: {}", layout.prikk_dir().display());
    println!("ref: {}", plan.ref_name);
    match plan.ref_state_id {
        Some(id) => println!("ref-state: {id}"),
        None => println!("ref-state: <not published>"),
    }
    match plan.block_id {
        Some(id) => println!("target block: {id}"),
        None => println!("target block: <none>"),
    }
    match plan.block_kind {
        Some(kind) => println!("block kind: {kind:?}"),
        None => println!("block kind: <none>"),
    }
    println!("parents: {}", plan.parent_count);
    println!("patches: {}", plan.patch_count);
    match plan.snapshot_blob_ref {
        Some(snapshot) => println!("snapshot blob: {snapshot}"),
        None => println!("snapshot blob: <none>"),
    }
    println!("materialization: {}", plan.materialization.as_str());
    match plan.materialization {
        CheckoutMaterialization::UnpublishedRef => {
            println!("note: publish a ref before checkout can target a block");
        }
        CheckoutMaterialization::NoWorktreeChanges => {
            println!("note: no worktree changes would be needed for this block");
        }
        CheckoutMaterialization::RequiresSnapshotMaterialization => {
            println!(
                "note: use `prikk checkout --snapshot-plan` to validate the snapshot manifest"
            );
        }
        CheckoutMaterialization::RequiresPatchEngine => {
            println!("note: patch application/algebra is deferred after PR-016");
        }
    }
}

fn print_snapshot_checkout_plan(
    layout: &RepositoryLayout,
    plan: &prikk_store::SnapshotCheckoutPlan,
) {
    println!("snapshot checkout plan repository: {}", layout.prikk_dir().display());
    print_checkout_plan(layout, &plan.checkout);
    println!("snapshot blob: {}", plan.snapshot_blob_id);
    println!("snapshot files: {}", plan.file_count);
    println!("snapshot content bytes: {}", plan.total_content_bytes);
    for path in &plan.paths {
        println!("  file: {path}");
    }
    println!("note: PR-016 validates the snapshot manifest but does not write the worktree");
}

fn print_history(layout: &RepositoryLayout, history: &RefHistory) {
    println!("history repository: {}", layout.prikk_dir().display());
    println!("ref: {}", history.ref_name);
    if history.is_empty() {
        println!("history: <empty>");
        return;
    }
    for entry in &history.entries {
        println!("block {}", entry.block_id);
        println!("  ref-state: {}", entry.ref_state_id);
        println!("  update-seq: {}", entry.update_seq);
        println!("  kind: {:?}", entry.block_kind);
        println!("  parents: {}", entry.parent_count);
        println!("  patches: {}", entry.patch_count);
        println!("  required-attestations: {}", entry.required_attestation_count);
        match entry.previous_ref_state_id {
            Some(previous) => println!("  previous-ref-state: {previous}"),
            None => println!("  previous-ref-state: <none>"),
        }
    }
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
        return Err("PR-016 supports only `prikk commit --allow-empty -m <message>`".to_string());
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
    println!("  prikk log [path] [--limit N] [--ref REF]  Show sealed ref history");
    println!("  prikk checkout --plan-only [path] [--ref REF]      Show a safe checkout plan");
    println!(
        "  prikk checkout --snapshot-plan [path] [--ref REF]  Validate snapshot manifest paths"
    );
    println!("  prikk verify [path]                       Verify objects and WAL records");
    println!("  prikk doctor [path]                       Run health diagnostics");
    println!("  prikk doctor [path] --repair-wal-tail     Truncate incomplete trailing WAL bytes");
    println!(
        "  prikk doctor [path] --repair-main-ref     Reconstruct a missing heads/main pointer"
    );
    println!("  prikk --version                           Print version");
}
