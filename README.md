# Prikk

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
cargo run -p prikk -- verify ./sample-repo
```

## Design Notes

Current implementation drop: **0.1.0 PR-009**.

Implemented:

- Rust workspace scaffold.
- Deterministic canonical object identity seed.
- Object envelopes with signatures outside identity.
- Persistent `.prikk/` layout and object store.
- Active-session WAL append/replay for signed patch envelopes.
- Read-only repository verification for objects, ref pointers, ref logs, and active WAL.
- Initial RefState publication primitives with flat hashed ref pointer paths.
- Inline signed RefUpdate log append/replay.
- Narrow empty-commit scaffold that appends a signed patch envelope to the active WAL.
- Local no-audit seal scaffold that persists WAL patches, creates a Block, and advances `heads/main`.
- Minimal CLI commands: `init`, `commit --allow-empty -m`, `seal --allow-no-audit`, `status`, `verify`, and `--version`.

Not implemented yet:

- Real worktree diff capture and worktree state materialization.
- Policy-aware audit/attestation publication through seal.
- Patch apply/commutation.
- Plugin/audit execution.
- Remote sync.

## More Detail

Full documentation is kept under `docs/src` and is structured for mdBook.
