//! `prikk compact` — reclaim stale records from the three genuine compaction targets (RFC 102 Stage
//! 6 Step 2). No confirmation prompt, unlike `prikk unlock`: see `prikk_store::compact`'s own module
//! doc for why compaction has no operator-only fact the tool cannot check itself.
//!
//! A bare `prikk compact` names no target and refuses rather than defaulting to `--all` -- the one
//! place in this command where "the tool decides" has a cheap, obvious alternative.

use std::path::PathBuf;

use prikk_store::{
    CompactionReport, compact_received_index, compact_ref_pointer_index, compact_trust_policy,
    plan_compact_received_index, plan_compact_ref_pointer_index, plan_compact_trust_policy,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    PointerIndex,
    ReceivedIndex,
    TrustPolicy,
}

const ALL_TARGETS: [Target; 3] = [
    Target::PointerIndex,
    Target::ReceivedIndex,
    Target::TrustPolicy,
];

pub(crate) fn run_compact(root: PathBuf, args: Vec<String>) -> std::result::Result<(), String> {
    let mut targets: Vec<Target> = Vec::new();
    let mut plan_only = false;
    for arg in args {
        match arg.as_str() {
            "--pointer-index" => targets.push(Target::PointerIndex),
            "--received-index" => targets.push(Target::ReceivedIndex),
            "--trust-policy" => targets.push(Target::TrustPolicy),
            "--all" => targets.extend(ALL_TARGETS),
            "--plan-only" => plan_only = true,
            other => return Err(format!("unknown compact argument: {other}")),
        }
    }
    if targets.is_empty() {
        return Err(
            "compact requires a target: --pointer-index, --received-index, --trust-policy, or \
             --all (add --plan-only to preview without writing)"
                .to_string(),
        );
    }
    targets.dedup();

    let layout = crate::open_repository(root)?;
    for target in targets {
        let report = run_one(&layout, target, plan_only).map_err(|err| err.to_string())?;
        print_report(&report, plan_only);
    }
    Ok(())
}

fn run_one(
    layout: &prikk_store::RepositoryLayout,
    target: Target,
    plan_only: bool,
) -> prikk_error::Result<CompactionReport> {
    match (target, plan_only) {
        (Target::PointerIndex, false) => compact_ref_pointer_index(layout),
        (Target::PointerIndex, true) => plan_compact_ref_pointer_index(layout),
        (Target::ReceivedIndex, false) => compact_received_index(layout),
        (Target::ReceivedIndex, true) => plan_compact_received_index(layout),
        (Target::TrustPolicy, false) => compact_trust_policy(layout),
        (Target::TrustPolicy, true) => plan_compact_trust_policy(layout),
    }
}

fn print_report(report: &CompactionReport, plan_only: bool) {
    let name = match report.container {
        prikk_store::LockableContainer::RefPointerIndex => "pointer-index",
        prikk_store::LockableContainer::ReceivedIndex => "received-index",
        prikk_store::LockableContainer::TrustPolicy => "trust-policy",
        prikk_store::LockableContainer::RefLog => "ref-log",
    };
    let verb = if plan_only {
        "would reclaim"
    } else {
        "reclaimed"
    };
    let reclaimed = report.entries_before.saturating_sub(report.entries_after);
    println!(
        "{name}: {reclaimed} {verb} ({} -> {} live records)",
        report.entries_before, report.entries_after
    );
}
