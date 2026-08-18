use prikk_store::{
    ActiveWalMetadataStatus, AuthorSignatureVerification, BlockStateStatus, DoctorSeverity,
    ObjectItemStatus, RefFileStatus, RefItemStatus, RepositoryLayout, StageStatus,
};

/// Render a count sourced from one verification stage. `None` means that stage did not evaluate to
/// completion -- printed as `unknown`, never as `0`, since zero is itself a claim ("checked, found
/// none") this repository's verification does not get to make about a stage that did not finish
/// (DC-95 Stage 2 ruling: a partial count is not "this many verified").
fn format_count(count: Option<usize>) -> String {
    match count {
        Some(value) => value.to_string(),
        None => "unknown (stage did not evaluate)".to_string(),
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
    // DC-95 Stage 2 Level 1: the reader consults stage outcomes first, counts and findings second --
    // a `Failed`/`NotEvaluated` stage's own counts below read `unknown`, not `0`, for the same reason.
    println!("verification stages: {}", report.stage_outcomes.len());
    for outcome in &report.stage_outcomes {
        match &outcome.status {
            StageStatus::Evaluated => {
                println!("stage {}: evaluated", outcome.stage);
            }
            StageStatus::Failed { message } => {
                println!("stage {}: failed: {message}", outcome.stage);
            }
            StageStatus::NotEvaluated { blocked_by } => {
                println!(
                    "stage {}: not evaluated (blocked by stage {blocked_by})",
                    outcome.stage
                );
            }
            StageStatus::Halted { after } => {
                println!(
                    "stage {}: not evaluated (walk halted after stage {after} failed, --stop-on-first-error)",
                    outcome.stage
                );
            }
        }
    }
    // DC-95 Stage 2 Level 2: item outcomes are printed as a count plus only the non-clean entries --
    // unlike the twelve stages above, there can be thousands of objects, so every `Evaluated` entry
    // is not printed individually.
    let failed_objects: Vec<_> = report
        .object_outcomes
        .iter()
        .filter(|outcome| matches!(outcome.status, ObjectItemStatus::Failed { .. }))
        .collect();
    println!(
        "object items: {} scanned, {} failed",
        report.object_outcomes.len(),
        failed_objects.len()
    );
    for outcome in failed_objects {
        if let ObjectItemStatus::Failed { message } = &outcome.status {
            println!(
                "object {} ({}): failed: {message}",
                outcome.path.display(),
                outcome.object_type
            );
        }
    }
    // DC-53 Stage 1: an unverifiable AUTHOR signature is not a failure (D3's second row) -- verify
    // still passes -- but must be visible, not silent, so it is surfaced here the same way failed
    // objects are above.
    let unverifiable_author_patches: Vec<_> = report
        .object_outcomes
        .iter()
        .filter_map(|outcome| match &outcome.status {
            ObjectItemStatus::Evaluated(verification)
            | ObjectItemStatus::Unindexed(verification) => {
                match &verification.author_verification {
                    Some(AuthorSignatureVerification::Unverifiable { key_id }) => {
                        Some((outcome, key_id))
                    }
                    _ => None,
                }
            }
            ObjectItemStatus::Failed { .. } => None,
        })
        .collect();
    println!(
        "unverifiable author signatures: {}",
        unverifiable_author_patches.len()
    );
    for (outcome, key_id) in unverifiable_author_patches {
        println!(
            "object {} ({}): unverifiable author signature: no key material recorded for {key_id}",
            outcome.path.display(),
            outcome.object_type
        );
    }
    let incomplete_blocks: Vec<_> = report
        .block_state_outcomes
        .iter()
        .filter(|outcome| !matches!(outcome.status, BlockStateStatus::Verified))
        .collect();
    println!(
        "block state items: {} checked, {} incomplete",
        report.block_state_outcomes.len(),
        incomplete_blocks.len()
    );
    for outcome in incomplete_blocks {
        match &outcome.status {
            BlockStateStatus::Verified => {}
            BlockStateStatus::Failed { message } => {
                println!("block {}: state-root failed: {message}", outcome.block_id);
            }
            BlockStateStatus::NotEvaluated { blocked_by } => {
                println!(
                    "block {}: state root not evaluated (blocked by block {blocked_by})",
                    outcome.block_id
                );
            }
        }
    }
    let failed_ref_files: Vec<_> = report
        .pointer_outcomes
        .iter()
        .chain(&report.log_outcomes)
        .filter(|outcome| matches!(outcome.status, RefFileStatus::Failed { .. }))
        .collect();
    println!(
        "ref files: {} scanned, {} failed",
        report.pointer_outcomes.len() + report.log_outcomes.len(),
        failed_ref_files.len()
    );
    for outcome in failed_ref_files {
        if let RefFileStatus::Failed { message } = &outcome.status {
            println!("ref file {}: failed: {message}", outcome.path.display());
        }
    }
    let failed_refs: Vec<_> = report
        .ref_item_outcomes
        .iter()
        .filter(|outcome| matches!(outcome.status, RefItemStatus::Failed { .. }))
        .collect();
    println!(
        "ref items: {} scanned, {} failed",
        report.ref_item_outcomes.len(),
        failed_refs.len()
    );
    for outcome in failed_refs {
        if let RefItemStatus::Failed { message } = &outcome.status {
            println!("ref {}: failed: {message}", outcome.ref_name);
        }
    }
    println!("checked objects: {}", format_count(report.checked_objects));
    println!("checked blocks: {}", format_count(report.checked_blocks));
    println!(
        "checked rollback blocks: {}",
        format_count(report.checked_rollback_blocks)
    );
    println!(
        "checked sealed rollback patches: {}",
        format_count(report.checked_sealed_rollback_patches)
    );
    println!(
        "checked WAL records: {}",
        format_count(report.checked_wal_records)
    );
    println!(
        "persisted WAL patches: {}",
        format_count(report.persisted_wal_patches)
    );
    println!("checked refs: {}", format_count(report.checked_refs));
    println!(
        "checked ref-log records: {}",
        format_count(report.checked_ref_log_records)
    );
    println!(
        "ref publication issues: {}",
        report.ref_publication_issues.len()
    );
    for issue in &report.ref_publication_issues {
        println!("ref-publication [{}]: {}", issue.code, issue.message);
    }
    println!(
        "signature envelope warnings: {}",
        report.signature_envelope_issues.len()
    );
    for issue in &report.signature_envelope_issues {
        println!(
            "signature-envelope [{}] {}: {}",
            issue.code, issue.source, issue.message
        );
    }
    println!(
        "checked rollback draft WAL records: {}",
        format_count(report.checked_rollback_draft_records)
    );
    println!(
        "checked publication trust records: {}",
        format_count(report.checked_publication_trust_records)
    );
    println!(
        "publication trust issues: {}",
        report.publication_trust_issues.len()
    );
    for issue in &report.publication_trust_issues {
        println!("publication-trust [{}]: {}", issue.code, issue.message);
    }
    println!("sealed blocks: {}", report.block_seals.len());
    for seal in &report.block_seals {
        println!("sealed-block {}: {}", seal.block_id, seal.sealed_by_key_id);
    }
    println!("object temp warnings: {}", report.object_temp_paths.len());
    for path in &report.object_temp_paths {
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("<non-UTF-8 object temp>");
        println!("warning: non-authoritative object publication temp: {name}");
    }
    println!(
        "trailing partial WAL bytes: {}",
        format_count(report.trailing_partial_wal_bytes)
    );
    if report.has_trailing_partial_wal() {
        println!("warning: active WAL contains an incomplete trailing record");
    }
    match &report.active_wal_metadata_status {
        Some(status) => print_active_wal_metadata_status(status),
        None => println!("active WAL metadata: unknown (stage did not evaluate)"),
    }
    println!(
        "commit-index divergences: {}",
        report.commit_index_divergences.len()
    );
    for divergence in &report.commit_index_divergences {
        println!(
            "commit-index [divergence] {}: recorded {} but worktree content hashes to {}",
            divergence.path, divergence.recorded_hash, divergence.actual_hash
        );
    }
    println!(
        "lifecycle-cache divergences: {}",
        report.lifecycle_cache_divergences.len()
    );
    for divergence in &report.lifecycle_cache_divergences {
        println!(
            "lifecycle-cache [divergence] block {}: {}",
            divergence.baseline_block_id, divergence.detail
        );
    }
    println!(
        "merge-baseline divergences: {}",
        report.merge_baseline_divergences.len()
    );
    for divergence in &report.merge_baseline_divergences {
        println!(
            "merge-baseline [divergence] block {}: recorded baseline {} is not a common ancestor \
             of mainline parent {} and secondary parent {}",
            divergence.block_id,
            divergence.recorded_baseline,
            divergence.mainline_parent_id,
            divergence.secondary_parent_id
        );
    }
    println!(
        "active WAL ordering issues: {}",
        report.active_wal_ordering_issues.len()
    );
    for issue in &report.active_wal_ordering_issues {
        println!(
            "active-wal-ordering [violation] record {} has sequence {} not greater than \
             preceding sequence {}",
            issue.index, issue.seq, issue.previous_seq
        );
    }
}

fn print_active_wal_metadata_status(status: &ActiveWalMetadataStatus) {
    match status {
        ActiveWalMetadataStatus::MissingForEmptyWal => {
            println!("active WAL metadata: absent for empty WAL");
        }
        ActiveWalMetadataStatus::ValidForEmptyWal { ref_name } => {
            println!("active WAL metadata: stale local metadata for empty WAL ({ref_name})");
            println!("warning: active WAL ref metadata exists but the active WAL is empty");
        }
        ActiveWalMetadataStatus::InvalidForEmptyWal { reason } => {
            println!("active WAL metadata: malformed local metadata for empty WAL ({reason})");
            println!("warning: active WAL ref metadata exists but the active WAL is empty");
        }
        ActiveWalMetadataStatus::ValidForNonEmptyWal { ref_name } => {
            println!("active WAL metadata: valid for {ref_name}");
        }
        ActiveWalMetadataStatus::MissingForNonEmptyWal => {
            println!("active WAL metadata: missing for non-empty WAL");
            println!("error: active WAL contains records but has no ref metadata");
        }
        ActiveWalMetadataStatus::InvalidForNonEmptyWal { reason } => {
            println!("active WAL metadata: malformed for non-empty WAL ({reason})");
            println!("error: active WAL contains records but has malformed ref metadata");
        }
    }
}
