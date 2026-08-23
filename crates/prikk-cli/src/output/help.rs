//! Top-level help output.

/// Print top-level help.
pub(crate) fn print_help(version: &str) {
    println!("prikk {version}");
    println!();
    println!("Usage:");
    println!("  prikk init [path]                         Create a .prikk repository layout");
    println!("  prikk trust maintainer add --key-id ID --public-key HEX  Trust one MAINTAINER key");
    println!("  prikk trust maintainer remove --key-id ID Revoke one MAINTAINER key");
    println!(
        "  prikk commit --from-worktree [--text-edits] [--ref REF] -m <message> Append worktree changes"
    );
    println!("  prikk status                              Check repository and active WAL status");
    println!("  prikk seal --allow-no-audit [--ref REF] Seal active WAL into a branch ref");
    println!(
        "  prikk branch [list] [--all]                List branches deterministically (name, RefState id); --all also shows closed branches, marked"
    );
    println!("  prikk branch create <name> [--from REF]   Publish a branch at an existing target");
    println!(
        "  prikk branch close <name>                 Close a branch (not delete — pointer, history, and objects stay; reclaims nothing)"
    );
    println!(
        "  note: there is no `branch switch` yet, and no current-branch pointer; switching needs \
         a separate, not-yet-designed increment; every command resolves --ref explicitly in the \
         meantime"
    );
    println!(
        "  prikk tag [list]                          List tags deterministically (name, target block)"
    );
    println!(
        "  prikk tag create <name> --target <ref|block> [-m <message>]  Publish a tag at a block"
    );
    println!(
        "  prikk bundle export --ref REF --output <file>  Write a self-contained history bundle"
    );
    println!(
        "  prikk bundle import --input <file>        Import a bundle as an untrusted received pointer"
    );
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
    println!(
        "  prikk merge-evidence --baseline-block ID (--left-block ID|--left-ref REF) \
         (--right-block ID|--right-ref REF) [path]  Show read-only merge evidence"
    );
    println!(
        "  prikk merge-plan --baseline-block ID (--left-block ID|--left-ref REF) \
         (--right-block ID|--right-ref REF) [path]  Show a read-only merge plan"
    );
    println!(
        "  prikk merge --allow-no-audit --baseline-block ID --into REF --from REF [path]  \
         Seal a proven-confluent merge"
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
        "  prikk verify [path]                       Verify objects, WAL, refs, and publication trust"
    );
    println!("  prikk doctor [path]                       Run health diagnostics");
    println!("  prikk doctor [path] --repair-wal-tail     Truncate incomplete trailing WAL bytes");
    println!("  prikk unlock                              List every currently held lock");
    println!(
        "  prikk unlock --lock <path> [--yes]        Clear one stale lock (asks to confirm \
         unless --yes)"
    );
    println!("  prikk compact --pointer-index|--received-index|--trust-policy|--all [--plan-only]");
    println!(
        "                                             Reclaim stale index/policy records \
         (--plan-only previews only)"
    );
    println!(
        "  prikk sync summary --output <file>        Write this repository's PSYNCSU1 sync summary"
    );
    println!(
        "  prikk sync compare --summary <file>       Compare local refs against a remote summary"
    );
    println!("  prikk sync have <ref> --output <file>     Write a PSYNCHV1 have-list for one ref");
    println!(
        "  prikk sync build <ref> --have <file> --output <file>  Build a PEXCH002 artifact closing \
         the gap"
    );
    println!(
        "  note: a built sync artifact contains repository content in the clear -- prikk does \
         not encrypt it; move it only over a channel you trust"
    );
    println!(
        "  prikk sync accept <file> [--claims-out <file>]  Accept a PEXCH002 artifact (prints \
         claim ids; optionally writes them)"
    );
    println!("  prikk sync pending                        List accepted-but-unsealed patches");
    println!(
        "  prikk sync seal <ref> --claim <id>        Seal one accepted claim's patches into a block"
    );
    println!(
        "  prikk sync seal <ref> --claims <file>     Seal a batch of claims, ordered by parent \
         block first"
    );
    println!("  prikk sync tags                           List received tags and their resolution");
    println!(
        "  prikk sync adopt-tag <name>               Create a local, receiver-signed tag from a \
         received tag"
    );
    println!("  prikk --version                           Print version");
}
