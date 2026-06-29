# Prikk

![Status](https://img.shields.io/badge/status-early--implementation-orange)
[![license](https://img.shields.io/crates/l/prikk.svg)](LICENSE)
[![matten docs.rs](https://img.shields.io/docsrs/prikk?label=prikk%20docs)](https://docs.rs/prikk)
[![matten crates.io](https://img.shields.io/crates/v/prikk.svg?label=prikk)](https://crates.io/crates/prikk)

**A next-generation, design-first VCS built around block-oriented patch theory.**

## Overview

Prikk is an experimental distributed version control system focused on ease of use, safety,
resilience, flexibility, and long-term performance. The implementation follows the approved FDD
sequence: object identity and storage first, then WAL/ref durability, patch algebra, plugins, and
sync.

## Why / When

Use Prikk development builds when evaluating the architecture or contributing to the implementation.
Do not use Prikk for real project history yet.

## Quick Start

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p prikk -- init ./sample-repo
(cd ./sample-repo && ../target/debug/prikk commit --allow-empty -m "initial scaffold")
(cd ./sample-repo && ../target/debug/prikk seal --allow-no-audit)
cargo run -p prikk -- log ./sample-repo
cargo run -p prikk -- checkout --plan-only ./sample-repo
cargo run -p prikk -- checkout --patch-plan ./sample-repo
cargo run -p prikk -- checkout --patch-materialize ./sample-repo
cargo run -p prikk -- checkout --patch-delete-plan ./sample-repo
cargo run -p prikk -- inverse-plan ./sample-repo
cargo run -p prikk -- rollback-preview ./sample-repo
cargo run -p prikk -- rollback-draft --append-inverse ./sample-repo -m "draft rollback"
cargo run -p prikk -- rollback-draft-verify ./sample-repo
# After sealing that rollback draft, `log` and `verify` classify the sealed rollback block.
cargo run -p prikk -- worktree-status ./sample-repo
# After editing a UTF-8 tracked file inside ./sample-repo:
# (cd ./sample-repo && ../target/debug/prikk commit --from-worktree --text-edits -m "edit text")
cargo run -p prikk -- verify ./sample-repo
cargo run -p prikk -- doctor ./sample-repo
# If doctor reports only incomplete trailing WAL bytes:
# cargo run -p prikk -- doctor ./sample-repo --repair-wal-tail
# If doctor reports only a missing heads/main pointer recoverable from the ref log:
# cargo run -p prikk -- doctor ./sample-repo --repair-main-ref
```

## Design Notes

Current implementation drop: **0.1.0 PR-030**.

Implemented:

- Rust workspace scaffold.
- Deterministic canonical object identity seed.
- Object envelopes with signatures outside identity.
- Persistent `.prikk/` layout and object store.
- Active-session WAL append/replay for signed patch envelopes.
- Read-only repository verification for objects, block references, sealed rollback Patch classification, ref pointers, ref logs, and active WAL.
- `doctor` diagnostics layered on top of verification, with opt-in safe WAL tail and missing-ref-pointer repair.
- Read-only sealed-history inspection from the current RefState chain, including rollback block labels.
- Snapshot-manifest validation, path-safety checks, opt-in snapshot materialization, and read-only worktree status.
- Initial RefState publication primitives with flat hashed ref pointer paths.
- Narrow empty-commit and snapshot-baseline worktree commit scaffolds. As of the DC-09 §9.3 operation-record reconciliation, worktree patch authoring (`commit --from-worktree`) is fail-closed pending the node model: every §9.3 mutation operation is node-addressed and needs node-id tracking/minting (not yet implemented).
- Local no-audit seal scaffold that persists WAL patches, creates a Block, and advances `heads/main`.
- Supported patch replay planning and materialization for `CreateFile` and `DeleteNode`. `EditText` and `ReplaceBinary` are reconciled to their FDD-03 §9.3 node-addressed records but their application is deferred to the node model; `RenamePath`/`ChangePerm`/`CreateSymlink` records are reconciled and read-validated, application deferred.
- Explicit deletion planning and opt-in deletion of patch-removed files whose bytes still match the old blob.
- Read-only unsigned inverse planning, non-mutating rollback preview, conservative rollback draft append, active rollback draft verification, and sealed rollback block classification for the supported patch-operation subset.
- Content-anchored `EditText` scaffold with fixed 32-byte span hashes, anchor-id validation, conservative full-file exact-span replay for `anchor_id = "full-file"`, and opt-in worktree generation for modified UTF-8 files.
- Minimal CLI commands: `init`, `commit --allow-empty -m`, `commit --from-worktree [--text-edits] -m`, `seal --allow-no-audit`, `status`, `log`, `checkout --plan-only`, `checkout --snapshot-plan`, `checkout --snapshot-materialize`, `checkout --patch-plan`, `checkout --patch-materialize`, `checkout --patch-delete-plan`, `checkout --patch-materialize-delete`, `inverse-plan`, `rollback-preview`, `rollback-draft --append-inverse`, `rollback-draft-verify`, `worktree-status`, `verify`, `doctor`, `doctor --repair-wal-tail`, `doctor --repair-main-ref`, and `--version`.

Not implemented yet:

- Rename detection, arbitrary text-span discovery/generation, rollback refs, rollback authorization, commutation, full patch algebra, and general destructive checkout pruning.
- Policy-aware audit/attestation publication through seal.
- Plugin/audit execution.
- Remote sync.

## More Detail

Full documentation is kept under `docs/src` and is structured for mdBook.
