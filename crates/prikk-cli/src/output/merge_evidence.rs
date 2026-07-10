//! Output for read-only merge evidence reports.

use prikk_store::{MergeEvidenceDisplay, MergeEvidenceDisplayOperation};

/// Print a read-only merge evidence report.
pub(crate) fn print_merge_evidence(report: &MergeEvidenceDisplay) {
    println!("merge evidence");
    println!("baseline block: {}", report.baseline_block_id);
    println!("left selector: {}", report.left_selector.selector);
    println!(
        "left target block: {}",
        report.left_selector.target_block_id
    );
    println!("right selector: {}", report.right_selector.selector);
    println!(
        "right target block: {}",
        report.right_selector.target_block_id
    );
    println!("outcome: {}", report.outcome);
    match report.reason {
        Some(reason) => println!("reason: {reason}"),
        None => println!("reason: <none>"),
    }
    println!("left operations: {}", report.left_operation_count);
    println!("right operations: {}", report.right_operation_count);
    println!("items: {}", report.items.len());
    for item in &report.items {
        print!("{}", item.side);
        if item.operation.is_some() {
            print!(" ");
            print_operation(item.operation.as_ref());
        }
        if item.peer_operation.is_some() {
            print!(" <-> ");
            print_operation(item.peer_operation.as_ref());
        }
        println!();
        println!("  outcome: {}", item.outcome);
        println!("  reason: {}", item.reason_code);
        println!("  phase: {}", item.proof_phase);
        if let Some(scope) = item.evidence_scope {
            println!("  evidence-scope: {scope}");
        }
    }
    println!(
        "note: read-only evidence; no merge commit, ref update, WAL write, or worktree change was performed"
    );
}

fn print_operation(operation: Option<&MergeEvidenceDisplayOperation>) {
    let Some(operation) = operation else {
        print!("report");
        return;
    };
    print!("[{}]", operation.index);
    if let Some(op_seq) = operation.op_seq {
        print!(" op_seq={op_seq}");
    }
    if let Some(kind) = operation.kind {
        print!(" {kind}");
    }
    if let Some(path) = &operation.path {
        print!(" {path}");
    }
}
