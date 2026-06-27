//! CLI output helpers.

use prikk_store::{
    CheckoutMaterialization, CheckoutPlan, DoctorSeverity, RefHistory, RepositoryLayout,
    SnapshotCheckoutPlan, SnapshotMaterializationReport, WorktreeChangeKind, WorktreeStatusReport,
};

/// Print a checkout plan.
pub(crate) fn print_checkout_plan(layout: &RepositoryLayout, plan: &CheckoutPlan) {
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
                "note: use `prikk checkout --snapshot-plan` to validate, or \
                 `--snapshot-materialize` to write safely"
            );
        }
        CheckoutMaterialization::RequiresPatchEngine => {
            println!("note: patch application/algebra is deferred after PR-018");
        }
    }
}

/// Print a snapshot checkout plan.
pub(crate) fn print_snapshot_checkout_plan(layout: &RepositoryLayout, plan: &SnapshotCheckoutPlan) {
    println!("snapshot checkout plan repository: {}", layout.prikk_dir().display());
    print_checkout_plan(layout, &plan.checkout);
    println!("snapshot blob: {}", plan.snapshot_blob_id);
    println!("snapshot files: {}", plan.file_count);
    println!("snapshot content bytes: {}", plan.total_content_bytes);
    for path in &plan.paths {
        println!("  file: {path}");
    }
    println!("note: use `prikk checkout --snapshot-materialize` to write validated snapshot files");
}

/// Print a snapshot materialization report.
pub(crate) fn print_snapshot_materialization_report(
    layout: &RepositoryLayout,
    report: &SnapshotMaterializationReport,
) {
    println!(
        "snapshot materialization repository: {}",
        layout.prikk_dir().display()
    );
    println!("ref: {}", report.ref_name);
    println!("planned files: {}", report.planned_files);
    println!("written files: {}", report.written_files);
    println!("unchanged files: {}", report.unchanged_files);
    println!("snapshot content bytes: {}", report.total_content_bytes);
    for path in &report.paths {
        println!("  file: {path}");
    }
    println!("note: this path writes only snapshot-backed files and never applies patches");
}

/// Print a worktree status report.
pub(crate) fn print_worktree_status(layout: &RepositoryLayout, report: &WorktreeStatusReport) {
    println!("worktree-status repository: {}", layout.prikk_dir().display());
    println!("ref: {}", report.ref_name);
    println!("tracked files: {}", report.tracked_files);
    println!("unchanged files: {}", report.unchanged_files);
    println!("missing files: {}", report.count_kind(WorktreeChangeKind::Missing));
    println!("modified files: {}", report.count_kind(WorktreeChangeKind::Modified));
    println!("untracked files: {}", report.count_kind(WorktreeChangeKind::Untracked));
    println!(
        "unsupported paths: {}",
        report.count_kind(WorktreeChangeKind::UnsupportedPath)
    );
    if report.is_clean() {
        println!("worktree: clean against snapshot baseline");
    } else {
        println!("worktree: changed against snapshot baseline");
        for change in &report.changes {
            println!("  {} {} — {}", change.kind.as_str(), change.path, change.detail);
        }
    }
    println!("note: PR-018 reports changes only; it does not create patch operations yet");
}

/// Print ref history.
pub(crate) fn print_history(layout: &RepositoryLayout, history: &RefHistory) {
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

/// Print doctor results.
pub(crate) fn print_doctor_report(layout: &RepositoryLayout, report: &prikk_store::DoctorReport) {
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

/// Print verification results.
pub(crate) fn print_verify_report(
    layout: &RepositoryLayout,
    report: &prikk_store::RepositoryVerification,
) {
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

/// Print top-level help.
pub(crate) fn print_help(version: &str) {
    println!("prikk {version}");
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
    println!(
        "  prikk checkout --snapshot-materialize [path] [--ref REF]  Safely write snapshot files"
    );
    println!(
        "  prikk worktree-status [path] [--ref REF]  Report changes against snapshot baseline"
    );
    println!("  prikk verify [path]                       Verify objects and WAL records");
    println!("  prikk doctor [path]                       Run health diagnostics");
    println!("  prikk doctor [path] --repair-wal-tail     Truncate incomplete trailing WAL bytes");
    println!(
        "  prikk doctor [path] --repair-main-ref     Reconstruct a missing heads/main pointer"
    );
    println!("  prikk --version                           Print version");
}
