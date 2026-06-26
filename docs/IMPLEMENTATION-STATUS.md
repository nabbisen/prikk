# PRIKK Implementation Status

Version: 0.1.0 PR-005

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
- Minimal CLI for `init`, `status`, and `--version`.
- PR-005 CI feedback fixes for PR-004 logs.

## Not Implemented Yet

- RefState publication and ref logs.
- Seal transaction.
- Patch apply/inverse/commutation.
- Conflict witnesses and merge state.
- WASM plugin host.
- Audit publication policy.
- Remote sync.

## Gate Discipline

PR-005 stays within the approved M1 foundation area. It fixes PR-004 CI feedback and keeps WAL append/replay as the newest implementation boundary. It does not yet publish refs or implement seal semantics.
