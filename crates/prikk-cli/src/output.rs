//! CLI output helpers.

use prikk_store::{
    CheckoutMaterialization, CheckoutPlan, DoctorSeverity, PatchDeletionPlan, PatchInversePlan,
    PatchMaterializationReport, PatchReplayPlan, RefHistory, RepositoryLayout, RollbackDraftReport,
    RollbackDraftVerification, RollbackPreviewPlan, SnapshotCheckoutPlan,
    SnapshotMaterializationReport, WorktreeChangeKind, WorktreeStatusReport,
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
            println!("note: use `prikk checkout --patch-plan` for supported replay planning");
        }
    }
}

/// Print a snapshot checkout plan.
pub(crate) fn print_snapshot_checkout_plan(layout: &RepositoryLayout, plan: &SnapshotCheckoutPlan) {
    println!(
        "snapshot checkout plan repository: {}",
        layout.prikk_dir().display()
    );
    print_checkout_plan(layout, &plan.checkout);
    println!("snapshot blob: {}", plan.snapshot_blob_id);
    println!("snapshot files: {}", plan.file_count);
    println!("snapshot content bytes: {}", plan.total_content_bytes);
    for path in &plan.paths {
        println!("  file: {path}");
    }
    println!("note: use `prikk checkout --snapshot-materialize` to write validated snapshot files");
}

/// Print a supported patch replay plan.
pub(crate) fn print_patch_replay_plan(layout: &RepositoryLayout, plan: &PatchReplayPlan) {
    println!(
        "patch replay plan repository: {}",
        layout.prikk_dir().display()
    );
    println!("ref: {}", plan.ref_name);
    println!("target block: {}", plan.target_block_id);
    println!("blocks replayed: {}", plan.block_count);
    println!("patches replayed: {}", plan.patch_count);
    println!("operations applied: {}", plan.applied_operation_count);
    println!("result files: {}", plan.file_count);
    println!("result content bytes: {}", plan.total_content_bytes);
    for path in &plan.paths {
        println!("  file: {path}");
    }
    println!(
        "note: this replays CreateFile/DeleteNode/ReplaceBinary only; EditText \
         span-anchored apply, renames, conflicts, and full patch algebra remain \
         later PRs"
    );
}

/// Print a patch checkout deletion plan.
pub(crate) fn print_patch_deletion_plan(layout: &RepositoryLayout, plan: &PatchDeletionPlan) {
    println!(
        "patch deletion plan repository: {}",
        layout.prikk_dir().display()
    );
    println!("ref: {}", plan.ref_name);
    println!("planned deletions: {}", plan.planned_deletions);
    println!("deletable files: {}", plan.deletable_files);
    println!("already absent files: {}", plan.already_absent_files);
    println!("deletion conflicts: {}", plan.conflicts.len());
    for path in &plan.deletable_paths {
        println!("  delete: {path}");
    }
    for conflict in &plan.conflicts {
        println!("  refused: {} — {}", conflict.path, conflict.reason);
    }
    println!("note: only explicit patch-deleted files are eligible; extra files are never deleted");
}

/// Print a patch replay materialization report.
pub(crate) fn print_patch_materialization_report(
    layout: &RepositoryLayout,
    report: &PatchMaterializationReport,
) {
    println!(
        "patch materialization repository: {}",
        layout.prikk_dir().display()
    );
    println!("ref: {}", report.ref_name);
    println!("blocks replayed: {}", report.block_count);
    println!("patches replayed: {}", report.patch_count);
    println!("operations applied: {}", report.applied_operation_count);
    println!("planned files: {}", report.planned_files);
    println!("written files: {}", report.written_files);
    println!("unchanged files: {}", report.unchanged_files);
    println!("deleted files: {}", report.deleted_files);
    println!(
        "already absent deleted files: {}",
        report.already_absent_deleted_files
    );
    println!("deletion conflicts: {}", report.deletion_conflicts);
    println!("result content bytes: {}", report.total_content_bytes);
    for path in &report.paths {
        println!("  file: {path}");
    }
    println!(
        "note: this materializes the supported patch replay result; opt-in deletion removes only \
         explicit patch-deleted files whose current bytes still match the old blob"
    );
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

/// Print a supported patch inverse plan.
pub(crate) fn print_patch_inverse_plan(layout: &RepositoryLayout, plan: &PatchInversePlan) {
    println!(
        "patch inverse plan repository: {}",
        layout.prikk_dir().display()
    );
    println!("ref: {}", plan.ref_name);
    println!("target block: {}", plan.target_block_id);
    println!("blocks inspected: {}", plan.block_count);
    println!("patches inspected: {}", plan.patch_count);
    println!("original operations: {}", plan.original_operation_count);
    println!("inverse operations: {}", plan.inverse_operation_count);
    println!(
        "unsigned inverse patch id hint: {}",
        plan.inverse_patch_id_hint
    );
    for operation in &plan.operations {
        println!(
            "  {:04} {} {}",
            operation.op_seq,
            operation.kind.as_str(),
            operation.path
        );
    }
    println!(
        "note: this is a non-mutating unsigned inverse plan for the supported operation subset; \
         rollback refs, authorization, conflicts, and full patch algebra remain later PRs"
    );
}

/// Print a rollback preview plan.
pub(crate) fn print_rollback_preview_plan(layout: &RepositoryLayout, plan: &RollbackPreviewPlan) {
    println!(
        "rollback preview repository: {}",
        layout.prikk_dir().display()
    );
    println!("ref: {}", plan.ref_name);
    println!("target block: {}", plan.target_block_id);
    println!("blocks validated: {}", plan.block_count);
    println!("patches validated: {}", plan.patch_count);
    println!("inverse operations: {}", plan.inverse_operation_count);
    println!(
        "unsigned inverse patch id hint: {}",
        plan.inverse_patch_id_hint
    );
    println!("current files: {}", plan.current_file_count);
    println!("current content bytes: {}", plan.current_content_bytes);
    println!("preview files: {}", plan.preview_file_count);
    println!("preview content bytes: {}", plan.preview_content_bytes);
    println!("changes: {}", plan.change_count);
    println!("would create: {}", plan.would_create_files);
    println!("would delete: {}", plan.would_delete_files);
    println!("would replace: {}", plan.would_replace_files);
    for change in &plan.changes {
        match (change.current_bytes, change.preview_bytes) {
            (Some(current), Some(preview)) => println!(
                "  {} {} current-bytes={} preview-bytes={}",
                change.kind.as_str(),
                change.path,
                current,
                preview
            ),
            (Some(current), None) => println!(
                "  {} {} current-bytes={} preview-bytes=<absent>",
                change.kind.as_str(),
                change.path,
                current
            ),
            (None, Some(preview)) => println!(
                "  {} {} current-bytes=<absent> preview-bytes={}",
                change.kind.as_str(),
                change.path,
                preview
            ),
            (None, None) => println!(
                "  {} {} current-bytes=<absent> preview-bytes=<absent>",
                change.kind.as_str(),
                change.path
            ),
        }
    }
    println!(
        "note: this is a non-mutating rollback preview to the latest snapshot baseline; \
         rollback refs, authorization, worktree writes, and full patch algebra remain later PRs"
    );
}

/// Print a rollback draft append report.
pub(crate) fn print_rollback_draft_report(layout: &RepositoryLayout, report: &RollbackDraftReport) {
    println!(
        "rollback draft repository: {}",
        layout.prikk_dir().display()
    );
    println!("ref: {}", report.ref_name);
    println!("target block: {}", report.target_block_id);
    println!("inverse patch: {}", report.inverse_patch_id);
    println!("WAL sequence: {}", report.wal_sequence);
    println!("blocks inspected: {}", report.block_count);
    println!("patches inspected: {}", report.patch_count);
    println!("inverse operations: {}", report.inverse_operation_count);
    println!("preview changes: {}", report.preview_change_count);
    println!("would create: {}", report.would_create_files);
    println!("would delete: {}", report.would_delete_files);
    println!("would replace: {}", report.would_replace_files);
    for operation in &report.operations {
        println!(
            "  {:04} {} {}",
            operation.op_seq,
            operation.kind.as_str(),
            operation.path
        );
    }
    println!(
        "note: this appended a signed inverse Patch draft to the active WAL only; run seal later \
         to publish it, and rollback refs, audit policy, and worktree writes remain later PRs"
    );
}

/// Print a rollback draft verification report.
pub(crate) fn print_rollback_draft_verification(
    layout: &RepositoryLayout,
    report: &RollbackDraftVerification,
) {
    println!(
        "rollback draft verification repository: {}",
        layout.prikk_dir().display()
    );
    println!("ref: {}", report.ref_name);
    println!("target block: {}", report.target_block_id);
    println!("draft patch: {}", report.draft_patch_id);
    println!("WAL sequence: {}", report.wal_sequence);
    println!("blocks inspected: {}", report.block_count);
    println!("patches inspected: {}", report.patch_count);
    println!("inverse operations: {}", report.inverse_operation_count);
    println!("decoded operations: {}", report.decoded_operation_count);
    println!(
        "note: this validates the active rollback draft against the current inverse plan only; \
         seal, rollback refs, authorization, audit policy, and worktree writes remain separate"
    );
}

/// Print a worktree status report.
pub(crate) fn print_worktree_status(layout: &RepositoryLayout, report: &WorktreeStatusReport) {
    println!(
        "worktree-status repository: {}",
        layout.prikk_dir().display()
    );
    println!("ref: {}", report.ref_name);
    println!("tracked files: {}", report.tracked_files);
    println!("unchanged files: {}", report.unchanged_files);
    println!(
        "missing files: {}",
        report.count_kind(WorktreeChangeKind::Missing)
    );
    println!(
        "modified files: {}",
        report.count_kind(WorktreeChangeKind::Modified)
    );
    println!(
        "untracked files: {}",
        report.count_kind(WorktreeChangeKind::Untracked)
    );
    println!(
        "unsupported paths: {}",
        report.count_kind(WorktreeChangeKind::UnsupportedPath)
    );
    if report.is_clean() {
        println!("worktree: clean against snapshot baseline");
    } else {
        println!("worktree: changed against snapshot baseline");
        for change in &report.changes {
            println!(
                "  {} {} — {}",
                change.kind.as_str(),
                change.path,
                change.detail
            );
        }
    }
    println!(
        "note: use `prikk commit --from-worktree -m <message>` for coarse file-level \
         operations, or add `--text-edits` for conservative full-file UTF-8 edits"
    );
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
        println!("  rollback-block: {}", entry.is_rollback_block);
        println!("  parents: {}", entry.parent_count);
        println!("  patches: {}", entry.patch_count);
        println!("  rollback-patches: {}", entry.rollback_patch_count);
        println!(
            "  required-attestations: {}",
            entry.required_attestation_count
        );
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
        println!(
            "{} [{}]: {}",
            issue.severity.as_str(),
            issue.code,
            issue.message
        );
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
    println!(
        "checked rollback blocks: {}",
        report.checked_rollback_blocks
    );
    println!(
        "checked sealed rollback patches: {}",
        report.checked_sealed_rollback_patches
    );
    println!("checked WAL records: {}", report.checked_wal_records);
    println!("persisted WAL patches: {}", report.persisted_wal_patches);
    println!("checked refs: {}", report.checked_refs);
    println!(
        "checked ref-log records: {}",
        report.checked_ref_log_records
    );
    println!(
        "checked rollback draft WAL records: {}",
        report.checked_rollback_draft_records
    );
    println!(
        "trailing partial WAL bytes: {}",
        report.trailing_partial_wal_bytes
    );
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
    println!("  prikk commit --from-worktree [--text-edits] -m <message> Append worktree changes");
    println!("  prikk status                              Check repository and active WAL status");
    println!("  prikk seal --allow-no-audit              Seal active WAL into heads/main");
    println!(
        "  prikk log [path] [--limit N] [--ref REF]  Show sealed ref history including rollback blocks"
    );
    println!("  prikk checkout --plan-only [path] [--ref REF]      Show a safe checkout plan");
    println!(
        "  prikk checkout --snapshot-plan [path] [--ref REF]  Validate snapshot manifest paths"
    );
    println!(
        "  prikk checkout --snapshot-materialize [path] [--ref REF]  Safely write snapshot files"
    );
    println!(
        "  prikk checkout --patch-plan [path] [--ref REF]  Replay supported file-level patches"
    );
    println!(
        "  prikk checkout --patch-materialize [path] [--ref REF]  Safely write patch replay files"
    );
    println!(
        "  prikk checkout --patch-delete-plan [path] [--ref REF]  Plan explicit patch deletions"
    );
    println!(
        "  prikk checkout --patch-materialize-delete [path] [--ref REF]  Write/delete patch files"
    );
    println!("  prikk inverse-plan [path] [--ref REF]     Plan an unsigned inverse patch");
    println!("  prikk rollback-preview [path] [--ref REF] Preview non-mutating rollback");
    println!(
        "  prikk rollback-draft --append-inverse [path] [--ref REF] \
         -m <message> Append inverse Patch"
    );
    println!("  prikk rollback-draft-verify [path] [--ref REF] Verify active rollback Patch");
    println!(
        "  prikk worktree-status [path] [--ref REF]  Report changes against snapshot baseline"
    );
    println!(
        "  prikk verify [path]                       Verify objects, rollback blocks, and WAL records"
    );
    println!("  prikk doctor [path]                       Run health diagnostics");
    println!("  prikk doctor [path] --repair-wal-tail     Truncate incomplete trailing WAL bytes");
    println!(
        "  prikk doctor [path] --repair-main-ref     Reconstruct a missing heads/main pointer"
    );
    println!("  prikk --version                           Print version");
}
