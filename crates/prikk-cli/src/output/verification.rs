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

/// Escape a string for embedding in a JSON string literal, per RFC 8259 §7 (RFC 118 stage 5).
/// `prikk-cli` has no third-party dependencies (RFC 118 §10 prerequisite 4), so there is no
/// `serde_json` to lean on here -- this is the repository's first hand-rolled JSON emitter, and
/// this function is the one place a mistake would actually corrupt output. Handles the two
/// structural escapes (`"`, `\`), the three conventional short escapes (`\n`, `\r`, `\t`), and
/// every other C0 control character (U+0000-U+001F) as `\u00XX` -- the minimum RFC 118 stage 5
/// requires. `StageStatus::Failed`'s message is arbitrary text reaching this from `PrikkError` and
/// from filesystem paths, so this is proven against hostile input, not a happy path (this module's
/// own tests).
fn escape_json_string(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len() + 2);
    escaped.push('"');
    for character in input.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other if (other as u32) < 0x20 => {
                escaped.push_str(&format!("\\u{:04x}", other as u32));
            }
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

/// `prikk verify --format json` (RFC 118 stage 5): `verify-report-v1` -- named for the tool
/// (`verify`), the shape (`report`), and versioned like this repository's other machine-readable
/// schemas (`release-policy-boundary-v1` and its siblings), so a future breaking change to this
/// shape has somewhere to go without silently reinterpreting what an old consumer already parsed.
/// Not `verify-full-v1` or similar: this is deliberately a subset of `RepositoryVerification` (the
/// schema version name must not imply completeness the document does not have).
///
/// Emits exactly the schema version, the verdict (every currently-true blocking condition from
/// `verify_verdict::VERDICT_CONDITIONS` -- the same declaration the exit code reads, so they cannot
/// disagree), and one entry per [`prikk_store::VerificationStage::ALL`], **in `ALL` order**, keyed
/// by `label()` (an external interface as of RFC 118 stage 4/5 -- do not rename any). Counts and
/// item-level findings are deliberately out of v1 scope (RFC 118 stage 5 handoff §1).
///
/// Review fix (stage 5 review v1, condition 1): this walks [`prikk_store::VerificationStage::ALL`]
/// and looks up each stage's outcome, rather than walking `report.stage_outcomes` directly.
/// `stage_outcomes` is documented to always carry exactly one entry per `ALL` member, but that
/// guarantee lives in `verify_repository`'s own test suite (stage 4), not in this function's type
/// signature -- walking it directly would let the document silently carry fewer than fourteen
/// entries, in whatever order the pipeline happened to run, if that guarantee were ever violated
/// upstream. Walking `ALL` instead makes the document **structurally incapable** of that: a missing
/// outcome is a hard failure (`expect`) before anything is printed -- since `json` is fully built
/// before the single `println!` below, a violated invariant here means no document is emitted at
/// all, never a malformed or incomplete one. `emit valid JSON or do not emit` (handoff §3) extends
/// to `emit a complete document or do not emit`.
pub(crate) fn print_verify_report_json(report: &prikk_store::RepositoryVerification) {
    let conditions = crate::verify_verdict::all_true_conditions(report);
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str("  \"schema_version\": \"verify-report-v1\",\n");
    json.push_str("  \"verdict\": {\n");
    json.push_str(&format!("    \"ok\": {},\n", conditions.is_empty()));
    json.push_str("    \"failed_conditions\": [");
    for (index, condition) in conditions.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str("\n      {\"id\": ");
        json.push_str(&escape_json_string(condition.id));
        json.push_str(", \"message\": ");
        json.push_str(&escape_json_string(condition.message));
        json.push('}');
    }
    if !conditions.is_empty() {
        json.push_str("\n    ");
    }
    json.push_str("]\n  },\n");
    json.push_str("  \"stages\": [");
    for (index, stage) in prikk_store::VerificationStage::ALL.iter().enumerate() {
        let outcome = report
            .stage_outcomes
            .iter()
            .find(|outcome| outcome.stage == *stage)
            .unwrap_or_else(|| {
                panic!(
                    "RepositoryVerification is missing an outcome for stage {:?} \
                     (VerificationStage::ALL); this is a bug in verify_repository, not in the \
                     JSON emitter -- refusing to emit an incomplete verify-report-v1 document",
                    stage.label()
                )
            });
        if index > 0 {
            json.push(',');
        }
        json.push_str("\n    {\"stage\": ");
        json.push_str(&escape_json_string(outcome.stage.label()));
        match &outcome.status {
            StageStatus::Evaluated => json.push_str(", \"status\": \"evaluated\"}"),
            StageStatus::Failed { message } => {
                json.push_str(", \"status\": \"failed\", \"message\": ");
                json.push_str(&escape_json_string(message));
                json.push('}');
            }
            StageStatus::NotEvaluated { blocked_by } => {
                json.push_str(", \"status\": \"not_evaluated\", \"blocked_by\": ");
                json.push_str(&escape_json_string(blocked_by.label()));
                json.push('}');
            }
            StageStatus::Halted { after } => {
                json.push_str(", \"status\": \"halted\", \"after\": ");
                json.push_str(&escape_json_string(after.label()));
                json.push('}');
            }
        }
    }
    if !prikk_store::VerificationStage::ALL.is_empty() {
        json.push_str("\n  ");
    }
    json.push_str("]\n}");
    println!("{json}");
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
    // still passes -- but must be visible, not silent, so it is counted here. Count only, no
    // per-object enumeration: unlike a failed object (rare, the reason the precedent above prints
    // one line each), an unrecorded key is the default state of every patch authored before this
    // increment -- on an existing repository that is the count of patches, not a short list of
    // outliers, and printing one identical line per patch would bury the informative count line
    // rather than surface it (DC-53 Stage 1 implementation review v2).
    let unverifiable_author_patch_count = report
        .object_outcomes
        .iter()
        .filter(|outcome| match &outcome.status {
            ObjectItemStatus::Evaluated(verification)
            | ObjectItemStatus::Unindexed(verification) => {
                matches!(
                    verification.author_verification,
                    Some(AuthorSignatureVerification::Unverifiable { .. })
                )
            }
            ObjectItemStatus::Failed { .. } => false,
        })
        .count();
    println!("unverifiable author signatures: {unverifiable_author_patch_count}");
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
