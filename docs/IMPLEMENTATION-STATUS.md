# PRIKK Implementation Status

Version: 0.1.0 PR-008

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
- Minimal CLI for `init`, `commit --allow-empty -m`, `status`, `verify`, and `--version`.

## Not Implemented Yet

- Seal transaction.
- Policy-aware ref publication from seal.
- Patch apply/inverse/commutation.
- Conflict witnesses and merge state.
- WASM plugin host.
- Audit publication policy.
- Remote sync.

## Gate Discipline

PR-008 stays within the approved M1 foundation area. It adds a narrow active-session commit
scaffold that appends a signed patch envelope to the WAL, but it does not yet implement real
worktree diff capture, seal semantics, patch algebra, plugin execution, or remote sync.
