# PRIKK

![Status](https://img.shields.io/badge/status-early--implementation-orange)
![License](https://img.shields.io/badge/license-Apache--2.0-blue)
![Rust](https://img.shields.io/badge/rust-2024-orange)

**A next-generation, design-first VCS built around block-oriented patch theory.**

## Overview

PRIKK is an experimental distributed version control system focused on ease of use, safety,
resilience, flexibility, and long-term performance. The implementation follows the approved FDD
sequence: object identity and storage first, then WAL/ref durability, patch algebra, plugins, and
sync.

## Why / When

Use PRIKK development builds when evaluating the architecture or contributing to the implementation.
Do not use PRIKK for real project history yet.

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
cargo run -p prikk -- worktree-status ./sample-repo
cargo run -p prikk -- verify ./sample-repo
cargo run -p prikk -- doctor ./sample-repo
# If doctor reports only incomplete trailing WAL bytes:
# cargo run -p prikk -- doctor ./sample-repo --repair-wal-tail
# If doctor reports only a missing heads/main pointer recoverable from the ref log:
# cargo run -p prikk -- doctor ./sample-repo --repair-main-ref
```

## Design Notes

Current implementation drop: **0.1.0 PR-024**.

Implemented:

- Rust workspace scaffold.
- Deterministic canonical object identity seed.
- Object envelopes with signatures outside identity.
- Persistent `.prikk/` layout and object store.
- Active-session WAL append/replay for signed patch envelopes.
- Read-only repository verification for objects, block references, ref pointers, ref logs, and active WAL.
- `doctor` diagnostics layered on top of verification, with opt-in safe WAL tail and missing-ref-pointer repair.
- Read-only sealed-history inspection from the current RefState chain.
- Snapshot-manifest validation, path-safety checks, opt-in snapshot materialization, and read-only worktree status.
- Initial RefState publication primitives with flat hashed ref pointer paths.
- Narrow empty-commit and snapshot-baseline worktree commit scaffolds.
- Local no-audit seal scaffold that persists WAL patches, creates a Block, and advances `heads/main`.
- Supported patch replay planning and materialization for `CreateFile`, `DeleteFile`, `ReplaceBinary`, and conservative full-file `EditText`.
- Explicit deletion planning and opt-in deletion of patch-removed files whose bytes still match the old blob.
- Content-anchored `EditText` scaffold with fixed 32-byte span hashes, anchor-id validation, and conservative full-file exact-span replay for `anchor_id = "full-file"`.
- Minimal CLI commands: `init`, `commit --allow-empty -m`, `commit --from-worktree -m`, `seal --allow-no-audit`, `status`, `log`, `checkout --plan-only`, `checkout --snapshot-plan`, `checkout --snapshot-materialize`, `checkout --patch-plan`, `checkout --patch-materialize`, `checkout --patch-delete-plan`, `checkout --patch-materialize-delete`, `worktree-status`, `verify`, `doctor`, `doctor --repair-wal-tail`, `doctor --repair-main-ref`, and `--version`.

Not implemented yet:

- Rename detection, arbitrary text-span discovery/generation, full patch algebra, and general destructive checkout pruning.
- Policy-aware audit/attestation publication through seal.
- Plugin/audit execution.
- Remote sync.

## More Detail

Full documentation is kept under `docs/src` and is structured for mdBook.
