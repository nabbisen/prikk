# PRIKK Implementation Status

Version: 0.1.0 PR-021

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
- Doctor diagnostics that convert verification outcomes into actionable issue codes.
- ActiveSession append API that holds `active.lock` while writing the active WAL.
- Empty-commit scaffold for manually exercising the commit/WAL path.
- Local no-audit seal scaffold that persists WAL patches, creates a Block, publishes `heads/main`, and clears the WAL after publication.
- Canonical decoding for RefState, RefUpdate, and Block payloads used by verification.
- Read-only sealed-history inspection from the current RefState chain.
- Read-only checkout planning that validates current RefState, Block, parent Block, Patch, and optional snapshot Blob references.
- Snapshot-manifest validation and conservative repository path-safety checks for snapshot materialization and status.
- Read-only worktree status against snapshot-backed baselines.
- Minimal CLI for `init`, `commit --allow-empty -m`, `seal --allow-no-audit`, `status`, `log`, `checkout --plan-only`, `checkout --snapshot-plan`, `checkout --snapshot-materialize`, `checkout --patch-plan`, `checkout --patch-materialize`, `worktree-status`, `verify`, `doctor`, `doctor --repair-wal-tail`, `doctor --repair-main-ref`, and `--version`.

## Not Implemented Yet

- Destructive worktree removals and full patch-based checkout semantics.
- Policy-aware audit/attestation publication from seal.
- Full patch algebra: text edit replay, inverse, commutation, confluence, and conflict witnesses.
- Conflict witnesses and merge state.
- WASM plugin host.
- Audit publication policy.
- Remote sync.

## Conservative Repairs Added Through PR-014

- `prikk doctor --repair-wal-tail` truncates only incomplete trailing active-WAL bytes after verification confirms that all preceding records are valid.
- `prikk doctor --repair-main-ref` reconstructs only a missing `heads/main` pointer from an already-valid ref log and RefState object.
- Repair refuses to mutate the repository when verification reports integrity errors.
- Missing-object repair, checksum-mismatch repair, object quarantine, GC, and malformed-log repair remain deferred.

## Gate Discipline

PR-021 stays within the approved foundation boundary by adding opt-in materialization from the supported patch replay result. It handles file-level CreateFile/DeleteFile/ReplaceBinary only, refuses conflicting existing files, never deletes extra files, and does not implement full algebra, audit plugin execution, policy enforcement, or remote sync.

## 0.1.0 PR-021

Added opt-in supported patch replay materialization. This is an M2 bridge scaffold, intentionally limited to file-level operations that already exist in PR-019/PR-020. It does not implement algebraic commutation, conflicted states, text edits, or destructive worktree removals.
