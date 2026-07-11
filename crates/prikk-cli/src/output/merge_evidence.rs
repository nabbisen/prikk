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
    println!("left operations: {}", report.left_operation_count);
    println!("right selector: {}", report.right_selector.selector);
    println!(
        "right target block: {}",
        report.right_selector.target_block_id
    );
    println!("right operations: {}", report.right_operation_count);
    println!("outcome: {}", report.outcome);
    match report.reason {
        Some(reason) => println!("reason: {reason}"),
        None => println!("reason: <none>"),
    }
    println!(
        "items: {} displayed of {}",
        report.displayed_item_count(),
        report.total_item_count()
    );
    for item in &report.items {
        println!();
        println!("{}:", item.side);
        match item.side {
            "cross" => {
                print_labeled_operation("left", item.operation.as_ref());
                print_labeled_operation("right", item.peer_operation.as_ref());
            }
            "left" | "right" => print_labeled_operation(item.side, item.operation.as_ref()),
            "report" => {}
            side => {
                println!("  {side}");
            }
        }
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

fn print_labeled_operation(label: &str, operation: Option<&MergeEvidenceDisplayOperation>) {
    let Some(operation) = operation else {
        println!("  {label}");
        return;
    };
    print!("  {label}[{}]", operation.index);
    if let Some(op_seq) = operation.op_seq {
        print!(" op_seq={op_seq}");
    }
    if let Some(kind) = operation.kind {
        print!(" {kind}");
    }
    if let Some(path) = &operation.path {
        print!(" {path}");
    }
    println!();
}
