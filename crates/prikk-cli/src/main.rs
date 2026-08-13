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
mod branch;
mod bundle;
mod merge;
mod output;
mod seal;
mod tag;

use args::{
    CheckoutMode, MergeEvidenceTargetArg, current_dir, parse_checkout_args, parse_commit_args,
    parse_doctor_args, parse_inverse_plan_args, parse_log_args, parse_merge_evidence_args,
    parse_merge_plan_args, parse_rollback_draft_args, parse_rollback_draft_verify_args,
    parse_rollback_preview_args, parse_verify_args, parse_worktree_status_args,
};
use output::{
    print_checkout_plan, print_doctor_report, print_help, print_history, print_merge_evidence,
    print_merge_plan, print_patch_deletion_plan, print_patch_inverse_plan,
    print_patch_materialization_report, print_patch_replay_plan, print_rollback_draft_report,
    print_rollback_draft_verification, print_rollback_preview_plan, print_snapshot_checkout_plan,
    print_snapshot_materialization_report, print_verify_report, print_worktree_status,
};
use prikk_store::{
    ActiveRefMetadata, DEFAULT_ACTIVE_PATCH_LIMIT, DoctorRepairOptions, Ed25519AuthorSigner,
    Ed25519MaintainerSigner, MergeEvidenceTarget, RefStore, RepositoryFormat, RepositoryLayout,
    VerifyOptions, Wal, WorktreePatchCommitOptions, add_trusted_maintainer, append_rollback_draft,
    commit_worktree_changes_signed, doctor_repository, list_received_pointers,
    load_received_ref_history, load_ref_history, materialize_patch_checkout,
    materialize_patch_checkout_with_deletions, materialize_snapshot_checkout,
    plan_patch_checkout_deletions, prepare_checkout_plan, prepare_merge_evidence,
    prepare_merge_plan, prepare_patch_inverse_plan, prepare_patch_replay_plan,
    prepare_rollback_preview, prepare_snapshot_checkout_plan, read_active_ref_metadata,
    repair_repository, verify_active_rollback_draft, verify_repository_with_options,
    worktree_status,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn open_repository(
    root: impl Into<PathBuf>,
) -> std::result::Result<RepositoryLayout, String> {
    let layout = RepositoryLayout::open(root).map_err(|err| err.to_string())?;
    if layout.format() == RepositoryFormat::LegacyV1 {
        eprintln!(
            "warning: format-1 repository opened in legacy read-only mode; scaffold roots are not verifiable state commitments"
        );
    }
    Ok(layout)
}

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
        Some("branch") => run_branch(args.collect()),
        Some("bundle") => run_bundle(args.collect()),
        Some("tag") => run_tag(args.collect()),
        Some("trust") => run_trust(args.collect()),
        Some("status") => run_status(),
        Some("log") => run_log(args.collect()),
        Some("checkout") => run_checkout(args.collect()),
        Some("merge-evidence") => run_merge_evidence(args.collect()),
        Some("merge-plan") => run_merge_plan(args.collect()),
        Some("merge") => run_merge(args.collect()),
        Some("inverse-plan") => run_inverse_plan(args.collect()),
        Some("rollback-preview") => run_rollback_preview(args.collect()),
        Some("rollback-draft") => run_rollback_draft(args.collect()),
        Some("rollback-draft-verify") => run_rollback_draft_verify(args.collect()),
        Some("worktree-status") => run_worktree_status(args.collect()),
        Some("verify") => run_verify(args.collect()),
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
    let layout = open_repository(root)?;
    layout
        .require_current_format()
        .map_err(|err| err.to_string())?;
    let thresholds = ActivePatchThresholds::from_env()?;
    let options = if args.text_edits {
        WorktreePatchCommitOptions::prefer_text_edits()
    } else {
        WorktreePatchCommitOptions::file_level()
    }
    .with_active_patch_limit(thresholds.limit);
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

fn run_merge(args: Vec<String>) -> std::result::Result<(), String> {
    let signer = maintainer_signer_from_env()?;
    let report = merge::run_merge(args, &signer)?;
    println!("merged {} into {}", report.from_ref, report.into_ref);
    println!("baseline block: {}", report.baseline_block_id);
    println!("parent block: {}", report.parent_block_id);
    println!("adopted target block: {}", report.adopted_target_block_id);
    println!("adopted patches: {}", report.adopted_patch_ids.len());
    for patch_id in &report.adopted_patch_ids {
        println!("  {patch_id}");
    }
    println!("block id: {}", report.block_id);
    println!("{} RefState: {}", report.into_ref, report.ref_state_id);
    Ok(())
}

fn run_branch(args: Vec<String>) -> std::result::Result<(), String> {
    let root = current_dir()?;
    branch::run_branch(root, args)
}

fn run_tag(args: Vec<String>) -> std::result::Result<(), String> {
    let root = current_dir()?;
    tag::run_tag(root, args)
}

fn run_bundle(args: Vec<String>) -> std::result::Result<(), String> {
    let root = current_dir()?;
    bundle::run_bundle(root, args)
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
            let layout = open_repository(root)?;
            let (adopted, newly_added) = add_trusted_maintainer(&layout, &key_id, &public_key)
                .map_err(|err| err.to_string())?;
            if newly_added {
                println!("trusted maintainer key: {}", adopted.key_id);
            } else {
                println!("maintainer key already trusted: {}", adopted.key_id);
            }
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
    let layout = open_repository(root)?;
    let wal = Wal::for_layout(&layout);
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
    // DC-66 criterion 7: report the queued patch count and the ref the queue targets, distinct from
    // `replay.records.len()` (a raw count with no ownership) and `heads/main RefState` (the last
    // *sealed* state, not what an active queue is targeting).
    if replay.records.is_empty() {
        println!("queued patches: 0");
    } else {
        let target = match read_active_ref_metadata(&layout).map_err(|err| err.to_string())? {
            ActiveRefMetadata::Valid(ref_name) => ref_name,
            ActiveRefMetadata::Missing => "<missing metadata>".to_string(),
            ActiveRefMetadata::Invalid(_) => "<malformed metadata>".to_string(),
        };
        println!(
            "queued patches: {} targeting {target}",
            replay.records.len()
        );
        // DC-57 (NFR-PERF-02): extends DC-66's existing queue report rather than inventing a second
        // reporting path, per app-requirements §6.3 ("status must recommend sealing").
        let thresholds = ActivePatchThresholds::from_env()?;
        if replay.records.len() >= thresholds.limit {
            println!(
                "warning: active patches ({}) at or above the configured hard limit ({}); \
                 commit is blocked until you run `prikk seal`",
                replay.records.len(),
                thresholds.limit
            );
        } else if replay.records.len() >= thresholds.warn {
            println!(
                "warning: active patches ({}) at or above the recommended threshold ({}); \
                 consider running `prikk seal`",
                replay.records.len(),
                thresholds.warn
            );
        }
    }
    println!(
        "status: multi-operation text diff minimization, plugins, and sync not \
         yet implemented"
    );
    Ok(())
}

/// DC-57 (NFR-PERF-02): active-patch warn/hard-block thresholds, read once from the environment and
/// validated together — a warn threshold above the hard limit, a non-numeric value, or zero for
/// either is rejected rather than silently kept at the default (the same fail-closed precedent as
/// `PRIKK_AUTHOR_KEY_ID`/`PRIKK_AUTHOR_SEED`). Per-invocation only; never persisted in the
/// repository — a durable policy belongs to a future general configuration increment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ActivePatchThresholds {
    warn: usize,
    limit: usize,
}

/// NFR-PERF-02's default warn threshold. The hard-block default,
/// [`prikk_store::DEFAULT_ACTIVE_PATCH_LIMIT`], is owned by `prikk-store` since it is also the
/// fallback baked into `WorktreePatchCommitOptions::file_level`/`prefer_text_edits`; this constant
/// has no store-side counterpart to share, since only the CLI's `status` output ever consults it.
const DEFAULT_ACTIVE_PATCH_WARN: usize = 800;

impl ActivePatchThresholds {
    fn from_env() -> std::result::Result<Self, String> {
        let warn =
            parse_active_patch_threshold_env("PRIKK_ACTIVE_PATCH_WARN", DEFAULT_ACTIVE_PATCH_WARN)?;
        let limit = parse_active_patch_threshold_env(
            "PRIKK_ACTIVE_PATCH_LIMIT",
            DEFAULT_ACTIVE_PATCH_LIMIT,
        )?;
        if warn > limit {
            return Err(format!(
                "PRIKK_ACTIVE_PATCH_WARN ({warn}) must not exceed PRIKK_ACTIVE_PATCH_LIMIT ({limit})"
            ));
        }
        Ok(Self { warn, limit })
    }
}

/// Parse one active-patch threshold environment variable, failing closed (never silently defaulting)
/// on anything present but malformed: non-numeric, or zero.
fn parse_active_patch_threshold_env(
    name: &str,
    default: usize,
) -> std::result::Result<usize, String> {
    let Ok(raw) = std::env::var(name) else {
        return Ok(default);
    };
    let trimmed = raw.trim();
    let value: usize = trimmed
        .parse()
        .map_err(|_| format!("{name} must be a positive integer, got {raw:?}"))?;
    if value == 0 {
        return Err(format!("{name} must be greater than zero, got 0"));
    }
    Ok(value)
}

fn run_log(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_log_args(args)?;
    let layout = open_repository(args.root)?;
    // Received refs (DC-78 ruling 4) live under refs/received/, not refs/by-id/, and their
    // RefState objects carry the origin's own name rather than the local "remotes/"-prefixed one
    // — load_ref_history's pointer/name-match check can't resolve them, so route separately.
    let history = if args.ref_name.starts_with("remotes/") {
        load_received_ref_history(&layout, &args.ref_name, args.limit)
    } else {
        load_ref_history(&layout, &args.ref_name, args.limit)
    }
    .map_err(|err| err.to_string())?;
    print_history(&layout, &history);
    Ok(())
}

fn run_checkout(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_checkout_args(args)?;
    let layout = open_repository(args.root)?;
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
    let layout = open_repository(args.root)?;
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

fn run_merge_plan(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_merge_plan_args(args)?;
    let layout = open_repository(args.root)?;
    let plan = prepare_merge_plan(
        &layout,
        args.baseline_block_id,
        merge_target_from_arg(args.left_target),
        merge_target_from_arg(args.right_target),
    )
    .map_err(|err| err.to_string())?;
    print_merge_plan(&plan);
    Ok(())
}

fn merge_target_from_arg(target: MergeEvidenceTargetArg) -> MergeEvidenceTarget {
    match target {
        MergeEvidenceTargetArg::Block(block_id) => MergeEvidenceTarget::Block(block_id),
        // DC-85: a `--left-ref`/`--right-ref` value can name a received ref too, so evidence/plan
        // previews reach the same source a `merge` invocation would — same prefix-routing shape as
        // `run_log`'s `remotes/` dispatch, not `run_verify`/`branch list`'s list-both shape, since
        // this resolves one specific input rather than showing everything.
        MergeEvidenceTargetArg::Ref(ref_name) if ref_name.starts_with("remotes/") => {
            MergeEvidenceTarget::ReceivedRef(ref_name)
        }
        MergeEvidenceTargetArg::Ref(ref_name) => MergeEvidenceTarget::Ref(ref_name),
    }
}

fn run_inverse_plan(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_inverse_plan_args(args)?;
    let layout = open_repository(args.root)?;
    let plan =
        prepare_patch_inverse_plan(&layout, &args.ref_name).map_err(|err| err.to_string())?;
    print_patch_inverse_plan(&layout, &plan);
    Ok(())
}

fn run_rollback_preview(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_rollback_preview_args(args)?;
    let layout = open_repository(args.root)?;
    let plan = prepare_rollback_preview(&layout, &args.ref_name).map_err(|err| err.to_string())?;
    print_rollback_preview_plan(&layout, &plan);
    Ok(())
}

fn run_rollback_draft(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_rollback_draft_args(args)?;
    let layout = open_repository(args.root)?;
    layout
        .require_current_format()
        .map_err(|err| err.to_string())?;
    let signer = author_signer_from_env()?;
    let report = append_rollback_draft(&layout, &args.ref_name, &args.message, &signer)
        .map_err(|err| err.to_string())?;
    print_rollback_draft_report(&layout, &report);
    Ok(())
}

fn run_rollback_draft_verify(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_rollback_draft_verify_args(args)?;
    let layout = open_repository(args.root)?;
    let report =
        verify_active_rollback_draft(&layout, &args.ref_name).map_err(|err| err.to_string())?;
    print_rollback_draft_verification(&layout, &report);
    Ok(())
}

fn run_worktree_status(args: Vec<String>) -> std::result::Result<(), String> {
    let args = parse_worktree_status_args(args)?;
    let layout = open_repository(args.root)?;
    let report = worktree_status(&layout, &args.ref_name).map_err(|err| err.to_string())?;
    print_worktree_status(&layout, &report);
    if report.is_clean() {
        Ok(())
    } else {
        Err("worktree has snapshot-baseline changes".to_string())
    }
}

fn run_verify(args: Vec<String>) -> std::result::Result<(), String> {
    let verify_args = parse_verify_args(args)?;
    let layout = open_repository(verify_args.root)?;
    let options = VerifyOptions {
        stop_on_first_error: verify_args.stop_on_first_error,
    };
    let report = verify_repository_with_options(&layout, options).map_err(|err| err.to_string())?;
    print_verify_report(&layout, &report);
    // Received refs (DC-78 ruling 4) are never read by verify_repository itself — every object
    // they point at is already checked by the ordinary type-based object scan regardless of which
    // ref (if any) points to it, so this is purely additive presentation, not a new check.
    let received = list_received_pointers(&layout).map_err(|err| err.to_string())?;
    println!("received refs: {}", received.len());
    for pointer in &received {
        println!(
            "received-ref {}: {}",
            pointer.ref_name, pointer.ref_state_id
        );
    }
    if report.has_stage_failure() {
        Err(
            "repository verification did not complete every stage; see stage outcomes above"
                .to_string(),
        )
    } else if report.has_item_failure() {
        Err(
            "repository verification found at least one failed object or block; see item outcomes above"
                .to_string(),
        )
    } else if report.has_unverifiable_state_roots() {
        Err("format-1 scaffold roots are not verifiable state commitments".to_string())
    } else if report.has_active_wal_metadata_integrity_issue() {
        Err("repository has active-WAL metadata integrity issues".to_string())
    } else if report.has_blocking_ref_publication_issues() {
        Err("repository has interrupted or divergent ref publication state".to_string())
    } else if report.has_publication_trust_issues() {
        Err("repository has publication-trust issues".to_string())
    } else if report.has_commit_index_divergence() {
        Err("commit-index cache disagrees with the worktree for at least one path".to_string())
    } else if report.has_lifecycle_cache_divergence() {
        Err("lifecycle-state cache disagrees with an independent replay".to_string())
    } else if report.has_active_wal_ordering_issue() {
        Err("active WAL contains an out-of-order or duplicate queued patch sequence".to_string())
    } else if report.has_merge_baseline_divergence() {
        Err("a merge block's recorded baseline is not a common ancestor of its parents".to_string())
    } else {
        Ok(())
    }
}

fn run_doctor(args: Vec<String>) -> std::result::Result<(), String> {
    let doctor_args = parse_doctor_args(args)?;
    let layout = open_repository(doctor_args.root)?;
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
        // DC-66 criterion 5: a repair against a queue of N must say *which* patches survived, not
        // just how many — "3 records preserved" does not identify them for N > 1.
        for patch_id in &repair.wal_repair.preserved_patch_ids {
            println!("repair: preserved queued patch {patch_id}");
        }
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
pub(crate) fn maintainer_signer_from_env() -> Result<Ed25519MaintainerSigner, String> {
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
