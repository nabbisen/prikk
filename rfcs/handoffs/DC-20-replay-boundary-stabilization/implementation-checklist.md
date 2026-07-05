# DC-20 Implementation Checklist - Replay Boundary Stabilization

Status: Companion for implemented DC-20; inherits lifecycle state from the primary RFC
Related RFC: `../../done/DC-20-REPLAY-BOUNDARY-STABILIZATION.md`

## Purpose

This checklist folds the v1 architect review errata into the implementation-review surface for DC-20.
It is not independent design authority; RFC-000 lifecycle state follows the primary DC-20 RFC.

## Required Implementation Scope

DC-20 implementation must remain a boundary-audit, documentation, and focused-test hardening pass. It
must not move `text_span`, patch algebra, store-backed resolver construction, lifecycle-cache
persistence, worktree behavior, CLI behavior, schema, refs/WAL, trust, or public merge/confluence
surfaces into `prikk-replay`.

## Compatibility Wrapper Inventory

Implementation review must include a table with these columns:

| Surface | Current path | Semantic owner | Keep / migrate / remove later | Reason |
|---|---|---|---|---|

The initial inventory is:

| Surface | Current path | Semantic owner | Keep / migrate / remove later | Reason |
|---|---|---|---|---|
| Lifecycle compatibility imports | `crates/prikk-store/src/node_lifecycle.rs` | `prikk-replay` | Keep during DC-20 | Import-only compatibility for existing store modules and tests. |
| Repository path compatibility reexports | `crates/prikk-store/src/path.rs` and `prikk_store::RepoPath` | `prikk-replay` for lexical path identity; `prikk-store` for integration use | Keep during DC-20 | Preserves the current integration surface while lexical validation lives in `prikk-replay`. |

Any added, removed, or changed wrapper must be listed. A wrapper must not contain semantic lifecycle or
path-policy implementation.

## `RepoPath` Boundary Check

Implementation review must verify that `RepoPath` remains lexical in `prikk-replay`:

- parse, normalize, validate, order, compare, and expose repository-relative path strings;
- support lifecycle/path-occupancy state;
- reject repository-private or platform-ambiguous lexical paths.

The following must remain outside `prikk-replay`:

- filesystem-root joining;
- checkout/materialization writes;
- worktree scanning;
- host-platform filesystem repair or recovery policy;
- repository layout decisions.

## Dependency Evidence

Implementation review must include:

- `cargo tree -p prikk-replay`;
- `cargo metadata --format-version 1` or an equivalent dependency summary proving:
  - normal dependencies of `prikk-replay` do not include `prikk-store`;
  - dev-dependencies of `prikk-replay` do not include `prikk-store`, unless explicitly justified;
  - no feature flag can enable `prikk-replay -> prikk-store`;
  - `crates/prikk-replay/Cargo.toml` keeps `publish = false`.

## Behavior-Neutral Evidence

Implementation review must include:

- moved/edited-file list;
- logic-change versus import/doc/test-only summary;
- identity/vector test evidence;
- focused lifecycle test evidence;
- store-level replay/cache test evidence;
- statement that CLI output did not change, or exact wording-only changes if review accepted them;
- explicit statement that no schema, repository layout, ref/WAL, trust, or worktree behavior changed.

## Gate Evidence

Implementation review should include observed output for:

- `cargo fmt --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo test --workspace`;
- `git diff --check`;
- file-size and test-module placement audit.

## Release-Note Deferrals

Release notes and implementation status must explicitly say the following remain deferred:

- `text_span` extraction;
- patch-algebra extraction;
- store-backed resolver movement;
- lifecycle-cache persistence movement;
- worktree extraction;
- public `prikk-replay` API stabilization;
- public merge, confluence, and conflict surfaces.
