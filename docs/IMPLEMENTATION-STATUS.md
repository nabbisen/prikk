# PRIKK Implementation Status

Version: 0.1.0 PR-009

## Implemented

- Rust workspace scaffold.
- Shared error taxonomy.
- First-party SHA-256 implementation for early object identity tests.
- Deterministic canonical TLV encoder seed.
- Object IDs and object envelopes.
- Core payload shape seeds.
- Persistent `.prikk/` repository layout.
- File-backed object store with identity verification on read.
- Active-session lock scaffold.
- Active-session WAL append/replay for signed patch envelopes.
- Ref-specific lock scaffold.
- RefState object publication primitive.
- Flat hashed ref pointer paths under `refs/by-id/`.
- Inline signed RefUpdate log append/replay.
- Read-only repository verification for persisted object files, ref pointers, ref logs, and active WAL records.
- ActiveSession append API that holds `active.lock` while writing the active WAL.
- Empty-commit scaffold for manually exercising the commit/WAL path.
- Local no-audit seal scaffold that persists WAL patches, creates a Block, publishes `heads/main`, and clears the WAL after publication.
- RefState canonical decoding for current-head parent discovery.
- Minimal CLI for `init`, `commit --allow-empty -m`, `seal --allow-no-audit`, `status`, `verify`, and `--version`.

## Not Implemented Yet

- Real worktree diff capture and worktree state materialization.
- Policy-aware audit/attestation publication from seal.
- Patch apply/inverse/commutation.
- Conflict witnesses and merge state.
- WASM plugin host.
- Audit publication policy.
- Remote sync.

## Gate Discipline

PR-009 stays within the approved M1/M3 foundation boundary by adding a local no-audit seal scaffold
for exercising object persistence and ref publication. It does not yet implement real worktree diff
capture, patch algebra, audit plugin execution, policy enforcement, or remote sync.
