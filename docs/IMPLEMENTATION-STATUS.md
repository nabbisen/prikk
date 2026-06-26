# PRIKK Implementation Status

Version: 0.1.0 PR-011

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
- Read-only repository verification for persisted object files, sealed block references, ref pointers, ref logs, and active WAL records.
- Read-only doctor diagnostics that convert verification outcomes into actionable issue codes.
- ActiveSession append API that holds `active.lock` while writing the active WAL.
- Empty-commit scaffold for manually exercising the commit/WAL path.
- Local no-audit seal scaffold that persists WAL patches, creates a Block, publishes `heads/main`, and clears the WAL after publication.
- Canonical decoding for RefState, RefUpdate, and Block payloads used by verification.
- Minimal CLI for `init`, `commit --allow-empty -m`, `seal --allow-no-audit`, `status`, `verify`, `doctor`, and `--version`.

## Not Implemented Yet

- Real worktree diff capture and worktree state materialization.
- Policy-aware audit/attestation publication from seal.
- Patch apply/inverse/commutation.
- Conflict witnesses and merge state.
- WASM plugin host.
- Audit publication policy.
- Remote sync.

## Gate Discipline

PR-011 stays within the approved foundation boundary by adding read-only doctor diagnostics. It does not perform destructive repair and does not implement real worktree diff capture, patch algebra, audit plugin execution, policy enforcement, or remote sync.
