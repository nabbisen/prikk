//! RFC 118 stage 2 — the join gate (`rfcs/accepted/118-derive-never-transcribe.md` §8).
//!
//! **Ruled: no serialization boundary.** This is a `#[test]` in `prikk-cli` reading
//! [`super::COMMANDS`] directly (the registry is already Rust; the documents are text on disk) —
//! not a `release-policy` check, not an emitted inventory, no build artifact, no parser, no new
//! dependency. The precedent is Gate A (RFC 114's completeness guard), also a `#[test]` colocated
//! with the thing it guards.
//!
//! Two rules (RFC 118 §8):
//! - **(A)** every documented `prikk <command>` in a declared document names a real registry entry.
//! - **(B)** every registry entry is explained somewhere, or declared undocumented with a reason.
//!
//! **What "explained" means, decided and stated (§5)**: a command's name appears in a declared
//! document's *code context* (a fenced ``` block or an inline `` ` `` span — verified empirically,
//! before writing this gate, that every real `prikk <command>` mention across every declared
//! document lives in one of those two forms; bare prose never coincides with a real command name)
//! **outside** `README.md`'s `## Useful Commands` section, which is a bare listing with no prose —
//! the one bare-listing block among the declared documents. A mention there alone does not count
//! as an explanation.

#![allow(clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use super::COMMANDS;

/// `README.md` plus every `docs/src/` page found (by direct search, not glob) to mention a real
/// `prikk <command>` — declared, not scanned by wildcard (§3), so a new `docs/` file cannot
/// silently escape the gate and a deleted one fails loudly (`every_declared_document_exists`
/// below). **32 files under `docs/src/`** (31 found in the original search, plus
/// `docs/src/guide/status.md`, added along with the page itself to close the one gap this gate
/// found — review v1 §1/§4).
const DECLARED_DOCUMENTS: &[&str] = &[
    "docs/src/guide/checkout/checkout.md",
    "docs/src/guide/checkout/snapshot-checkout.md",
    "docs/src/guide/checkout/snapshot-materialization.md",
    "docs/src/guide/history.md",
    "docs/src/guide/merge-evidence.md",
    "docs/src/guide/merge.md",
    "docs/src/guide/merge-plan.md",
    "docs/src/guide/patches/patch-deletions.md",
    "docs/src/guide/patches/patch-inverse.md",
    "docs/src/guide/patches/patch-materialization.md",
    "docs/src/guide/patches/patch-replay.md",
    "docs/src/guide/patches/text-edits.md",
    "docs/src/guide/patches/worktree-patch.md",
    "docs/src/guide/rollback/rollback-draft.md",
    "docs/src/guide/rollback/rollback-draft-verify.md",
    "docs/src/guide/rollback/rollback-preview.md",
    "docs/src/guide/rollback/sealed-rollback-history.md",
    "docs/src/guide/security-setup.md",
    "docs/src/guide/status.md",
    "docs/src/guide/sync.md",
    "docs/src/guide/worktree-status.md",
    "docs/src/reference/architecture.md",
    "docs/src/reference/concurrency-locking.md",
    "docs/src/reference/data-model-lifecycle.md",
    "docs/src/reference/data-model.md",
    "docs/src/reference/integrity-recovery.md",
    "docs/src/reference/patch-algebra.md",
    "docs/src/reference/path-safety.md",
    "docs/src/reference/platform-support.md",
    "docs/src/reference/release-compatibility.md",
    "docs/src/reference/repository-layout.md",
    "docs/src/reference/trust-threat-model.md",
    "README.md",
];

/// `(command, reason)` pairs rule (B) accepts as deliberately unexplained — the same shape as
/// `signature_contract_tests::vectors::RFC114_ADMITTED_BUT_UNWRITTEN` next to Gate A, per the
/// handoff's explicit model. Every entry needs a real reason, not a placeholder: this is where the
/// gate's own honesty lives, since it is the one place a genuine gap could be quietly declared away
/// instead of fixed.
///
/// **Empty, deliberately.** `status` briefly lived here (review v1 §1): the reviewer ruled that an
/// accidental documentation gap is not what this list is for — `RFC114_ADMITTED_BUT_UNWRITTEN`
/// declares pairs no production code path has ever constructed, a *fact about the code*, not "we
/// have not written the page yet." `docs/src/guide/status.md` was written instead, closing the gap
/// directly. The constant stays, with its shape established, for a genuine future case — a command
/// that *cannot* be documented, not merely one that has not been yet — rather than being removed
/// only to be re-added under pressure the next time a gap appears.
const DECLARED_UNDOCUMENTED: &[(&str, &str)] = &[];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("prikk-cli's manifest dir has a workspace root two levels up")
        .to_path_buf()
}

/// Every declared path must exist -- a deleted declared document must fail loudly, not silently
/// stop being scanned (§3).
#[test]
fn every_declared_document_exists() {
    let root = repo_root();
    for path in DECLARED_DOCUMENTS {
        assert!(
            root.join(path).is_file(),
            "declared document is missing: {path}"
        );
    }
}

/// Extract every fenced (``` ... ```) and inline (`` `...` ``) code region from `text`, in the
/// order they start. Deliberately simple (byte-offset scanning against `text` directly, no
/// Markdown parser, no dependency): this project's docs never nest one inside the other, and
/// totality (never panicking on arbitrary text) matters more than handling every theoretical
/// Markdown edge case a hand-authored doc page will never actually contain.
fn code_regions(text: &str) -> Vec<&str> {
    let mut fenced_ranges = Vec::new();
    let mut regions = Vec::new();
    let mut offset = 0usize;
    while let Some(rel_start) = text[offset..].find("```") {
        let start = offset + rel_start;
        let after_open = start + 3;
        let Some(rel_end) = text[after_open..].find("```") else {
            // An unterminated fence: everything from here on is still "inside" it as far as this
            // scan can tell, so there is nothing left to inline-scan either.
            regions.push(&text[start..]);
            fenced_ranges.push((start, text.len()));
            break;
        };
        let end = after_open + rel_end + 3;
        regions.push(&text[start..end]);
        fenced_ranges.push((start, end));
        offset = end;
    }

    // Inline spans, skipping any that start inside an already-collected fenced range -- so a
    // `prikk x` sitting inside a fenced block is not also scanned as (or split by) an inline span.
    let mut offset = 0usize;
    while let Some(rel_start) = text[offset..].find('`') {
        let start = offset + rel_start;
        if fenced_ranges
            .iter()
            .any(|&(range_start, range_end)| start >= range_start && start < range_end)
        {
            offset = start + 1;
            continue;
        }
        let Some(rel_end) = text[start + 1..].find('`') else {
            break;
        };
        let end = start + 1 + rel_end + 1;
        regions.push(&text[start..end]);
        offset = end;
    }
    regions
}

/// Every distinct token immediately following `"prikk "` in `region` — a run of ASCII
/// alphanumerics/hyphens starting with a letter, so a meta-arm like `--version` (starts with `-`)
/// never matches.
fn command_tokens(region: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut rest = region;
    while let Some(rel) = rest.find("prikk ") {
        let after = &rest[rel + "prikk ".len()..];
        let token_len = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(after.len());
        let token = &after[..token_len];
        if token
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic())
        {
            tokens.push(token);
        }
        // Always advance by at least one byte, even when `token_len == 0` (the character right
        // after "prikk " didn't match at all), so an empty token can never stall the loop.
        let advance = token_len.max(1).min(after.len());
        rest = &after[advance..];
    }
    tokens
}

/// `README.md` only: everything inside the `## Useful Commands` section is not a genuine
/// explanation (it is the one bare listing among the declared documents, per this file's own doc
/// comment) -- return the text with that section's body removed so `explained_outside_readme_list`
/// never counts a mention found only there.
fn strip_readme_command_listing(text: &str) -> String {
    let Some(start) = text.find("## Useful Commands") else {
        return text.to_string();
    };
    let after_heading = &text[start + "## Useful Commands".len()..];
    let end = after_heading
        .find("\n## ")
        .map_or(text.len(), |rel| start + "## Useful Commands".len() + rel);
    let mut result = String::with_capacity(text.len());
    result.push_str(&text[..start]);
    result.push_str(&text[end..]);
    result
}

fn document_text(root: &Path, path: &str) -> String {
    let text = fs::read_to_string(root.join(path))
        .unwrap_or_else(|err| panic!("declared document {path} must read: {err}"));
    if path == "README.md" {
        strip_readme_command_listing(&text)
    } else {
        text
    }
}

/// Rule (A): every `prikk <command>` mentioned in code context in a declared document names a
/// real registry entry.
#[test]
fn rule_a_every_documented_command_names_a_real_registry_entry() {
    let root = repo_root();
    for path in DECLARED_DOCUMENTS {
        let text = fs::read_to_string(root.join(path))
            .unwrap_or_else(|err| panic!("declared document {path} must read: {err}"));
        for region in code_regions(&text) {
            for token in command_tokens(region) {
                assert!(
                    COMMANDS.iter().any(|command| command.name == token),
                    "{path} documents `prikk {token}`, which is not a real registry command name"
                );
            }
        }
    }
}

fn is_explained(root: &Path, name: &str) -> bool {
    DECLARED_DOCUMENTS.iter().any(|path| {
        let text = document_text(root, path);
        code_regions(&text)
            .iter()
            .any(|region| command_tokens(region).contains(&name))
    })
}

/// Rule (B): every registry entry is explained somewhere, or declared undocumented with a reason.
#[test]
fn rule_b_every_registry_entry_is_explained_or_declared_undocumented() {
    let root = repo_root();
    for command in COMMANDS {
        if DECLARED_UNDOCUMENTED
            .iter()
            .any(|(name, _)| *name == command.name)
        {
            continue;
        }
        assert!(
            is_explained(&root, command.name),
            "`{}` is neither explained in a declared document outside README's bare command \
             listing, nor declared undocumented with a reason in DECLARED_UNDOCUMENTED",
            command.name
        );
    }
}

/// Self-guard on the escape hatch itself: every `DECLARED_UNDOCUMENTED` name must be a real
/// registry entry -- a stale or misspelled entry there would silently exempt nothing (or the wrong
/// command) from rule (B) forever.
#[test]
fn declared_undocumented_names_are_real_registry_entries() {
    for (name, reason) in DECLARED_UNDOCUMENTED {
        assert!(
            COMMANDS.iter().any(|command| &command.name == name),
            "DECLARED_UNDOCUMENTED names `{name}`, which is not a real registry command"
        );
        assert!(
            !reason.trim().is_empty(),
            "DECLARED_UNDOCUMENTED entry for `{name}` has no reason"
        );
    }
}
