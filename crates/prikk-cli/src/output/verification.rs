use prikk_store::{ActiveWalMetadataStatus, DoctorSeverity, RepositoryLayout};

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
        report.checked_rollback_draft_records
    );
    println!(
        "checked publication trust records: {}",
        report.checked_publication_trust_records
    );
    println!(
        "publication trust issues: {}",
        report.publication_trust_issues.len()
    );
    for issue in &report.publication_trust_issues {
        println!("publication-trust [{}]: {}", issue.code, issue.message);
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
        report.trailing_partial_wal_bytes
    );
    if report.has_trailing_partial_wal() {
        println!("warning: active WAL contains an incomplete trailing record");
    }
    print_active_wal_metadata_status(&report.active_wal_metadata_status);
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
