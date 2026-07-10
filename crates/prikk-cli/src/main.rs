#![forbid(unsafe_code)]

//! Prikk command-line entry point.
//!
//! The CLI exposes minimal repository layout commands, active WAL status, node-addressed worktree
//! commit authoring, explicit non-default branch genesis, deterministic arbitrary-span text edit generation,
//! read-only inverse planning, rollback preview, rollback draft append/verification, sealed rollback classification,
//! supported patch replay planning/materialization, explicit patch deletion planning, a local
//! no-audit seal scaffold, read-only history inspection, checkout planning, conservative snapshot
//! materialization, read-only worktree status, minimal publication trust setup, repository
//! verification, and doctor diagnostics.
//! Multi-operation text diff minimization, full patch algebra, audit plugins, and sync remain later increments.

use std::path::PathBuf;
use std::process::ExitCode;

mod args;
mod output;
mod seal;

use args::{
    CheckoutMode, MergeEvidenceTargetArg, current_dir, optional_path_or_current,
    parse_checkout_args, parse_commit_args, parse_doctor_args, parse_inverse_plan_args,
    parse_log_args, parse_merge_evidence_args, parse_rollback_draft_args,
    parse_rollback_draft_verify_args, parse_rollback_preview_args, parse_worktree_status_args,
};
use output::{
    print_checkout_plan, print_doctor_report, print_help, print_history, print_merge_evidence,
    print_patch_deletion_plan, print_patch_inverse_plan, print_patch_materialization_report,
    print_patch_replay_plan, print_rollback_draft_report, print_rollback_draft_verification,
    print_rollback_preview_plan, print_snapshot_checkout_plan,
    print_snapshot_materialization_report, print_verify_report, print_worktree_status,
};
use prikk_store::{
    DoctorRepairOptions, Ed25519AuthorSigner, Ed25519MaintainerSigner, MergeEvidenceTarget,
    RefStore, RepositoryLayout, Wal, WorktreePatchCommitOptions, add_trusted_maintainer,
    append_rollback_draft, commit_worktree_changes_signed, doctor_repository, load_ref_history,
    materialize_patch_checkout, materialize_patch_checkout_with_deletions,
    materialize_snapshot_checkout, plan_patch_checkout_deletions, prepare_checkout_plan,
    prepare_merge_evidence, prepare_patch_inverse_plan, prepare_patch_replay_plan,
    prepare_rollback_preview, prepare_snapshot_checkout_plan, repair_repository,
    verify_active_rollback_draft, verify_repository, worktree_status,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
        Some("trust") => run_trust(args.collect()),
        Some("status") => run_status(),
        Some("log") => run_log(args.collect()),
        Some("checkout") => run_checkout(args.collect()),
        Some("merge-evidence") => run_merge_evidence(args.collect()),
        Some("inverse-plan") => run_inverse_plan(args.collect()),
        Some("rollback-preview") => run_rollback_preview(args.collect()),
        Some("rollback-draft") => run_rollback_draft(args.collect()),
        Some("rollback-draft-verify") => run_rollback_draft_verify(args.collect()),
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
    println!(
        "initialized Prikk repository at {}",
        root.join(".prikk").display()
    );
    Ok(())
}

fn run_commit(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_commit_args(args)?;
    let root = current_dir()?;
    let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
    let options = if args.text_edits {
        WorktreePatchCommitOptions::prefer_text_edits()
    } else {
        WorktreePatchCommitOptions::file_level()
    };
    let signer = author_signer_from_env()?;
    let report =
        commit_worktree_changes_signed(&layout, &args.ref_name, &args.message, options, &signer)
            .map_err(|err| err.to_string())?;
    println!("recorded worktree patch in active WAL");
    println!("baseline ref: {}", report.ref_name);
    println!("patch id: {}", report.patch_id);
    println!("WAL sequence: {}", report.wal_sequence);
    println!("operations: {}", report.operation_count);
    println!("referenced blobs: {}", report.referenced_blob_count);
    println!("text edits: {}", report.text_edit_count);
    for change in &report.changes {
        println!("  {} {}", change.operation.as_str(), change.path);
    }
    println!(
        "note: multi-operation text diff minimization, patch algebra, rename detection, audit \
         plugins, and sync remain later increments"
    );
    Ok(())
}

fn run_seal(args: Vec<String>) -> std::result::Result<(), String> {
    let root = current_dir()?;
    let signer = maintainer_signer_from_env()?;
    let result = seal::run_seal(root, args, &signer)?;
    println!("sealed active WAL into block");
    println!("patches: {}", result.patch_count);
    println!("block id: {}", result.block_id);
    println!("{} RefState: {}", result.ref_name, result.ref_state_id);
    println!("note: audit plugins and patch-based worktree materialization remain later PRs");
    Ok(())
}

fn run_trust(args: Vec<String>) -> std::result::Result<(), String> {
    let mut args = args.into_iter();
    match (args.next().as_deref(), args.next().as_deref()) {
        (Some("maintainer"), Some("add")) => {
            let mut key_id = None;
            let mut public_key = None;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--key-id" => {
                        key_id = Some(
                            args.next()
                                .ok_or_else(|| "--key-id requires a value".to_string())?,
                        );
                    }
                    "--public-key" => {
                        public_key = Some(
                            args.next()
                                .ok_or_else(|| "--public-key requires a value".to_string())?,
                        );
                    }
                    other => return Err(format!("unknown trust maintainer add argument: {other}")),
                }
            }
            let key_id =
                key_id.ok_or_else(|| "trust maintainer add requires --key-id".to_string())?;
            let public_key = public_key
                .ok_or_else(|| "trust maintainer add requires --public-key".to_string())?;
            let root = current_dir()?;
            let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
            let policy = add_trusted_maintainer(&layout, &key_id, &public_key)
                .map_err(|err| err.to_string())?;
            println!("trusted maintainer key: {}", policy.key_id);
            println!("policy: required=1");
            Ok(())
        }
        _ => Err(
            "usage: prikk trust maintainer add --key-id <key-id> --public-key <64-hex>".to_string(),
        ),
    }
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
    println!(
        "trailing partial WAL bytes: {}",
        replay.trailing_partial_bytes
    );
    match main_ref {
        Some(id) => println!("heads/main RefState: {id}"),
        None => println!("heads/main RefState: <not published>"),
    }
    println!(
        "status: multi-operation text diff minimization, plugins, and sync not \
         yet implemented"
    );
    Ok(())
}

fn run_log(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_log_args(args)?;
    let layout = RepositoryLayout::open(args.root).map_err(|err| err.to_string())?;
    let history =
        load_ref_history(&layout, &args.ref_name, args.limit).map_err(|err| err.to_string())?;
    print_history(&layout, &history);
    Ok(())
}

fn run_checkout(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_checkout_args(args)?;
    let layout = RepositoryLayout::open(args.root).map_err(|err| err.to_string())?;
    match args.mode {
        CheckoutMode::PlanOnly => {
            let plan =
                prepare_checkout_plan(&layout, &args.ref_name).map_err(|err| err.to_string())?;
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
        CheckoutMode::PatchDeletePlan => {
            let plan = plan_patch_checkout_deletions(&layout, &args.ref_name)
                .map_err(|err| err.to_string())?;
            print_patch_deletion_plan(&layout, &plan);
            if !plan.is_safe_to_apply() {
                return Err("patch deletion plan has unsafe candidates".to_string());
            }
        }
        CheckoutMode::PatchMaterializeDelete => {
            let report = materialize_patch_checkout_with_deletions(&layout, &args.ref_name)
                .map_err(|err| err.to_string())?;
            print_patch_materialization_report(&layout, &report);
        }
    }
    Ok(())
}

fn run_merge_evidence(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_merge_evidence_args(args)?;
    let layout = RepositoryLayout::open(args.root).map_err(|err| err.to_string())?;
    let report = prepare_merge_evidence(
        &layout,
        args.baseline_block_id,
        merge_target_from_arg(args.left_target),
        merge_target_from_arg(args.right_target),
    )
    .map_err(|err| err.to_string())?;
    print_merge_evidence(&report);
    Ok(())
}

fn merge_target_from_arg(target: MergeEvidenceTargetArg) -> MergeEvidenceTarget {
    match target {
        MergeEvidenceTargetArg::Block(block_id) => MergeEvidenceTarget::Block(block_id),
        MergeEvidenceTargetArg::Ref(ref_name) => MergeEvidenceTarget::Ref(ref_name),
    }
}

fn run_inverse_plan(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_inverse_plan_args(args)?;
    let layout = RepositoryLayout::open(args.root).map_err(|err| err.to_string())?;
    let plan =
        prepare_patch_inverse_plan(&layout, &args.ref_name).map_err(|err| err.to_string())?;
    print_patch_inverse_plan(&layout, &plan);
    Ok(())
}

fn run_rollback_preview(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_rollback_preview_args(args)?;
    let layout = RepositoryLayout::open(args.root).map_err(|err| err.to_string())?;
    let plan = prepare_rollback_preview(&layout, &args.ref_name).map_err(|err| err.to_string())?;
    print_rollback_preview_plan(&layout, &plan);
    Ok(())
}

fn run_rollback_draft(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_rollback_draft_args(args)?;
    let layout = RepositoryLayout::open(args.root).map_err(|err| err.to_string())?;
    let signer = author_signer_from_env()?;
    let report = append_rollback_draft(&layout, &args.ref_name, &args.message, &signer)
        .map_err(|err| err.to_string())?;
    print_rollback_draft_report(&layout, &report);
    Ok(())
}

fn run_rollback_draft_verify(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_rollback_draft_verify_args(args)?;
    let layout = RepositoryLayout::open(args.root).map_err(|err| err.to_string())?;
    let report =
        verify_active_rollback_draft(&layout, &args.ref_name).map_err(|err| err.to_string())?;
    print_rollback_draft_verification(&layout, &report);
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
    if report.has_active_wal_metadata_integrity_issue() {
        Err("repository has active-WAL metadata integrity issues".to_string())
    } else if report.has_publication_trust_issues() {
        Err("repository has publication-trust issues".to_string())
    } else {
        Ok(())
    }
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
                if ref_repair.wrote_pointer {
                    "reconstructed"
                } else {
                    "kept existing"
                },
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

/// Build the AUTHOR signer from caller-supplied key material in the environment, failing closed if
/// none is configured. This is deliberately minimal key *input* — no trust store, key file, rotation,
/// or persistence (those are later phases). Real authoring requires:
///   - `PRIKK_AUTHOR_KEY_ID`: non-empty key identifier recorded in the signature;
///   - `PRIKK_AUTHOR_SEED`: 64 hex characters (a 32-byte Ed25519 secret seed).
fn author_signer_from_env() -> Result<Ed25519AuthorSigner, String> {
    let key_id = std::env::var("PRIKK_AUTHOR_KEY_ID").map_err(|_| {
        "author signing is required: set PRIKK_AUTHOR_KEY_ID (no signing key configured)"
            .to_string()
    })?;
    if key_id.trim().is_empty() {
        return Err("PRIKK_AUTHOR_KEY_ID must not be empty".to_string());
    }
    let seed_hex = std::env::var("PRIKK_AUTHOR_SEED").map_err(|_| {
        "author signing is required: set PRIKK_AUTHOR_SEED (64 hex chars; no signing key configured)"
            .to_string()
    })?;
    let seed = decode_seed_hex(&seed_hex, "PRIKK_AUTHOR_SEED")?;
    Ed25519AuthorSigner::from_seed(key_id, &seed).map_err(|err| err.to_string())
}

/// Build the MAINTAINER signer from caller-supplied key material in the environment, failing closed
/// if none is configured.
fn maintainer_signer_from_env() -> Result<Ed25519MaintainerSigner, String> {
    let key_id = std::env::var("PRIKK_MAINTAINER_KEY_ID").map_err(|_| {
        "maintainer signing is required: set PRIKK_MAINTAINER_KEY_ID (no signing key configured)"
            .to_string()
    })?;
    if key_id.trim().is_empty() {
        return Err("PRIKK_MAINTAINER_KEY_ID must not be empty".to_string());
    }
    let seed_hex = std::env::var("PRIKK_MAINTAINER_SEED").map_err(|_| {
        "maintainer signing is required: set PRIKK_MAINTAINER_SEED (64 hex chars; no signing key configured)"
            .to_string()
    })?;
    let seed = decode_seed_hex(&seed_hex, "PRIKK_MAINTAINER_SEED")?;
    Ed25519MaintainerSigner::from_seed(key_id, &seed).map_err(|err| err.to_string())
}

/// Decode exactly 64 hex characters into a 32-byte Ed25519 secret seed.
fn decode_seed_hex(hex: &str, env_name: &str) -> Result<[u8; 32], String> {
    let hex = hex.trim();
    if hex.len() != 64 {
        return Err(format!(
            "{env_name} must be 64 hex characters, got {}",
            hex.len()
        ));
    }
    let mut seed = [0_u8; 32];
    for (slot, pair) in seed.iter_mut().zip(hex.as_bytes().chunks_exact(2)) {
        let hi = pair
            .first()
            .copied()
            .ok_or_else(|| format!("{env_name} truncated"))?;
        let lo = pair
            .get(1)
            .copied()
            .ok_or_else(|| format!("{env_name} truncated"))?;
        *slot = (hex_nibble(hi, env_name)? << 4) | hex_nibble(lo, env_name)?;
    }
    Ok(seed)
}

/// Convert one ASCII hex character to its 4-bit value.
fn hex_nibble(c: u8, env_name: &str) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(format!("{env_name} contains a non-hex character")),
    }
}
