//! The command registry — RFC 118 stage 1: one declaration of prikk's dispatchable command
//! surface, so `main.rs::run`'s dispatch and `output::help::print_help`'s rendering both *derive*
//! from `COMMANDS` instead of each restating the command list by hand. Meta-arms (`--help`, `-h`,
//! `--version`, `-V`, and the argument-less case) are not commands and stay outside this table,
//! per the RFC 118 prerequisite ruling §1.
//!
//! `help_lines` holds each command's `--help` output pre-formatted, verbatim, exactly as it must
//! appear on the line -- not decomposed into separate form/summary fields with alignment computed
//! at render time. The existing column alignment is hand-tuned per line, not a single fixed-width
//! formula (compare `init`'s ~43-column gap against `trust maintainer add`'s 2-space gap once its
//! own form already overruns that column), and re-deriving it algorithmically risked exactly the
//! "improve the output to make rendering easier" regression the handoff forbids. Storing the
//! already-correct text still removes all command-specific text from `help.rs` itself, which
//! becomes a pure iterate-and-print renderer holding no literal command text of its own -- the
//! requirement the handoff actually states, even though the shape differs from its suggested
//! `Usage { form, summary }` split.

pub(crate) struct Command {
    pub(crate) name: &'static str,
    pub(crate) run: fn(Vec<String>) -> std::result::Result<(), String>,
    pub(crate) help_lines: &'static [&'static str],
}

/// `init` is the one dispatch arm with a non-`Vec<String>` signature ([`Option<String>`], a single
/// optional path) -- adapted here so the table's `run` field stays uniform, per the prerequisite
/// ruling's "one adapter closure each."
fn run_init_adapter(args: Vec<String>) -> std::result::Result<(), String> {
    crate::run_init(args.into_iter().next())
}

/// `status` is the other non-uniform arm: no arguments at all.
fn run_status_adapter(_args: Vec<String>) -> std::result::Result<(), String> {
    crate::run_status()
}

/// Order here is the `--help` rendering order (`output::help::print_help` iterates this table
/// directly) -- it is **not** the old `main.rs` match-arm order, which was itself never the order
/// `help.rs` printed in. Dispatch is a name lookup, so table order does not affect it.
pub(crate) const COMMANDS: &[Command] = &[
    Command {
        name: "init",
        run: run_init_adapter,
        help_lines: &[
            "  prikk init [path]                         Create a .prikk repository layout",
        ],
    },
    Command {
        name: "trust",
        run: crate::run_trust,
        help_lines: &[
            "  prikk trust maintainer add --key-id ID --public-key HEX  Trust one MAINTAINER key",
            "  prikk trust maintainer remove --key-id ID Revoke one MAINTAINER key",
        ],
    },
    Command {
        name: "commit",
        run: crate::run_commit,
        help_lines: &[
            "  prikk commit --from-worktree [--text-edits] [--ref REF] -m <message> Append worktree changes",
        ],
    },
    Command {
        name: "status",
        run: run_status_adapter,
        help_lines: &[
            "  prikk status                              Check repository and active WAL status",
        ],
    },
    Command {
        name: "seal",
        run: crate::run_seal,
        help_lines: &[
            "  prikk seal --allow-no-audit [--ref REF] Seal active WAL into a branch ref",
        ],
    },
    Command {
        name: "branch",
        run: crate::run_branch,
        help_lines: &[
            "  prikk branch [list] [--all]                List branches deterministically (name, RefState id); --all also shows closed branches, marked",
            "  prikk branch create <name> [--from REF]   Publish a branch at an existing target",
            "  prikk branch close <name>                 Close a branch (not delete — pointer, history, and objects stay; reclaims nothing)",
            "  note: there is no `branch switch` yet, and no current-branch pointer; switching needs a separate, not-yet-designed increment; every command resolves --ref explicitly in the meantime",
        ],
    },
    Command {
        name: "tag",
        run: crate::run_tag,
        help_lines: &[
            "  prikk tag [list]                          List tags deterministically (name, target block)",
            "  prikk tag create <name> --target <ref|block> [-m <message>]  Publish a tag at a block",
        ],
    },
    Command {
        name: "bundle",
        run: crate::run_bundle,
        help_lines: &[
            "  prikk bundle export --ref REF --output <file> [--force]  Write a self-contained history bundle; refuses an existing file unless --force",
            "  prikk bundle import --input <file>        Import a bundle as an untrusted received pointer",
            "  prikk bundle verify --input <file>        Check a bundle offline; writes nothing, needs no repository",
        ],
    },
    Command {
        name: "log",
        run: crate::run_log,
        help_lines: &[
            "  prikk log [path] [--limit N] [--ref REF]  Show sealed ref history including rollback blocks",
        ],
    },
    Command {
        name: "checkout",
        run: crate::run_checkout,
        help_lines: &[
            "  prikk checkout --plan-only [path] [--ref REF]      Show a safe checkout plan",
            "  prikk checkout --snapshot-plan [path] [--ref REF]  Validate snapshot manifest paths",
            "  prikk checkout --snapshot-materialize [path] [--ref REF]  Safely write snapshot files",
            "  prikk checkout --patch-plan [path] [--ref REF]  Replay supported file-level patches",
            "  prikk checkout --patch-materialize [path] [--ref REF]  Safely write patch replay files",
            "  prikk checkout --patch-delete-plan [path] [--ref REF]  Plan explicit patch deletions",
            "  prikk checkout --patch-materialize-delete [path] [--ref REF]  Write/delete patch files",
        ],
    },
    Command {
        name: "merge-evidence",
        run: crate::run_merge_evidence,
        help_lines: &[
            "  prikk merge-evidence --baseline-block ID (--left-block ID|--left-ref REF) (--right-block ID|--right-ref REF) [path]  Show read-only merge evidence",
        ],
    },
    Command {
        name: "merge-plan",
        run: crate::run_merge_plan,
        help_lines: &[
            "  prikk merge-plan --baseline-block ID (--left-block ID|--left-ref REF) (--right-block ID|--right-ref REF) [path]  Show a read-only merge plan",
        ],
    },
    Command {
        name: "merge",
        run: crate::run_merge,
        help_lines: &[
            "  prikk merge --allow-no-audit --baseline-block ID --into REF --from REF [path]  Seal a proven-confluent merge",
        ],
    },
    Command {
        name: "inverse-plan",
        run: crate::run_inverse_plan,
        help_lines: &["  prikk inverse-plan [path] [--ref REF]     Plan an unsigned inverse patch"],
    },
    Command {
        name: "rollback-preview",
        run: crate::run_rollback_preview,
        help_lines: &["  prikk rollback-preview [path] [--ref REF] Preview non-mutating rollback"],
    },
    Command {
        name: "rollback-draft",
        run: crate::run_rollback_draft,
        help_lines: &[
            "  prikk rollback-draft --append-inverse [path] [--ref REF] -m <message> Append inverse Patch",
        ],
    },
    Command {
        name: "rollback-draft-verify",
        run: crate::run_rollback_draft_verify,
        help_lines: &[
            "  prikk rollback-draft-verify [path] [--ref REF] Verify active rollback Patch",
        ],
    },
    Command {
        name: "worktree-status",
        run: crate::run_worktree_status,
        help_lines: &[
            "  prikk worktree-status [path] [--ref REF]  Report changes against snapshot baseline",
        ],
    },
    Command {
        name: "verify",
        run: crate::run_verify,
        help_lines: &[
            "  prikk verify [path]                       Verify objects, WAL, refs, and publication trust",
        ],
    },
    Command {
        name: "doctor",
        run: crate::run_doctor,
        help_lines: &[
            "  prikk doctor [path]                       Run health diagnostics",
            "  prikk doctor [path] --repair-wal-tail     Truncate incomplete trailing WAL bytes",
        ],
    },
    Command {
        name: "unlock",
        run: crate::run_unlock,
        help_lines: &[
            "  prikk unlock                              List every currently held lock",
            "  prikk unlock --lock <path> [--yes]        Clear one stale lock (asks to confirm unless --yes)",
        ],
    },
    Command {
        name: "compact",
        run: crate::run_compact,
        help_lines: &[
            "  prikk compact --pointer-index|--received-index|--trust-policy|--all [--plan-only]",
            "                                             Reclaim stale index/policy records (--plan-only previews only)",
        ],
    },
    Command {
        name: "sync",
        run: crate::run_sync,
        help_lines: &[
            "  prikk sync summary --output <file>        Write this repository's PSYNCSU1 sync summary",
            "  prikk sync compare --summary <file>       Compare local refs against a remote summary",
            "  prikk sync have <ref> --output <file>     Write a PSYNCHV1 have-list for one ref",
            "  prikk sync build <ref> --have <file> --output <file> [--force]  Build a PEXCH002 artifact closing the gap",
            "  note: a built sync artifact contains repository content in the clear -- prikk does not encrypt it; move it only over a channel you trust",
            "  prikk sync accept <file> [--claims-out <file>] [--force]  Accept a PEXCH002 artifact (prints claim ids; optionally writes them)",
            "  prikk sync pending                        List accepted-but-unsealed patches",
            "  prikk sync seal <ref> --claim <id>        Seal one accepted claim's patches into a block",
            "  prikk sync seal <ref> --claims <file>     Seal a batch of claims, ordered by parent block first",
            "  prikk sync tags                           List received tags and their resolution",
            "  prikk sync adopt-tag <name>               Create a local, receiver-signed tag from a received tag",
        ],
    },
];

/// Look up a command by name -- the single dispatch source `main.rs::run` calls into.
pub(crate) fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.name == name)
}

#[cfg(test)]
mod tests;
