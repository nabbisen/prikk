# PRIKK Implementation Status

Version: 0.1.0 PR-006

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
- Read-only repository verification for persisted object files and active WAL records.
- Minimal CLI for `init`, `status`, `verify`, and `--version`.

## Not Implemented Yet

- RefState publication and ref logs.
- Seal transaction.
- Patch apply/inverse/commutation.
- Conflict witnesses and merge state.
- WASM plugin host.
- Audit publication policy.
- Remote sync.

## Gate Discipline

PR-006 stays within the approved M1 foundation area. It verifies existing persisted objects and WAL
records but does not yet publish refs or implement seal semantics.
