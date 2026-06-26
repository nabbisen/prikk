# PRIKK Implementation Status

Version: 0.1.0 PR-007

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
- Minimal CLI for `init`, `status`, `verify`, and `--version`.

## Not Implemented Yet

- End-user commit command.
- Seal transaction.
- Policy-aware ref publication from seal.
- Patch apply/inverse/commutation.
- Conflict witnesses and merge state.
- WASM plugin host.
- Audit publication policy.
- Remote sync.

## Gate Discipline

PR-007 stays within the approved M1 foundation area. It adds ref-state and ref-log storage
primitives but does not yet implement full seal semantics, patch algebra, plugin execution, or
remote sync.
