use prikk_store::{RefHistory, RepositoryLayout, WorktreeChangeKind, WorktreeStatusReport};

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
        println!("worktree: clean against baseline");
    } else {
        println!("worktree: changed against baseline");
        for change in &report.changes {
            println!(
                "  {} {} — {}",
                change.kind.as_str(),
                change.path,
                change.detail
            );
        }
    }
    if let Some(other_ref) = &report.queued_elsewhere {
        println!(
            "note: the active WAL has queued (unsealed) patches for {other_ref}, not {} -- that \
             is real, committed work, not shown above; any \"untracked\" file here may be exactly \
             that work seen from this ref's own baseline, so do not delete based on this report \
             alone (see `prikk status`)",
            report.ref_name
        );
    }
    println!(
        "note: use `prikk commit -m <message>` to author node-addressed worktree changes; \
         text nodes use deterministic arbitrary-span EditText"
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
