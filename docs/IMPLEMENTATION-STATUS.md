# PRIKK Implementation Status

Version: 0.1.0 PR-029

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
- Explicit deletion planning and opt-in deletion for files removed by supported patch replay.
- Content-anchored `EditText` validation scaffold with fixed 32-byte span hashes, anchor ID validation, conservative full-file exact-span replay, and opt-in worktree generation for full-file UTF-8 edits.
- Read-only unsigned inverse planning, non-mutating rollback preview, and conservative rollback draft append and verification for the supported patch-operation subset.
- Minimal CLI for `init`, `commit --allow-empty -m`, `commit --from-worktree [--text-edits] -m`, `seal --allow-no-audit`, `status`, `log`, `checkout --plan-only`, `checkout --snapshot-plan`, `checkout --snapshot-materialize`, `checkout --patch-plan`, `checkout --patch-materialize`, `checkout --patch-delete-plan`, `checkout --patch-materialize-delete`, `inverse-plan`, `rollback-preview`, `rollback-draft --append-inverse`, `rollback-draft-verify`, `worktree-status`, `verify`, `doctor`, `doctor --repair-wal-tail`, `doctor --repair-main-ref`, and `--version`.

## Not Implemented Yet

- General destructive worktree pruning and full patch-based checkout semantics.
- Policy-aware audit/attestation publication from seal.
- Full patch algebra: arbitrary text-span replay, rollback ref publication, commutation, confluence, and conflict witnesses.
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

PR-029 stays within the approved foundation boundary by verifying only an active rollback draft already present in the WAL. It does not publish rollback refs, authorize rollback, modify the worktree, discover arbitrary spans, minimize text diffs, commute patches, resolve conflicts, or implement audit plugin execution, policy enforcement, or remote sync.

## 0.1.0 PR-022

Added explicit deletion planning and opt-in deletion during supported patch materialization. This is an M2 bridge scaffold, intentionally limited to files removed by replayed DeleteFile operations whose current bytes still match the recorded old Blob. It does not implement algebraic commutation, conflicted states, text edits, or general destructive pruning.

## 0.1.0 PR-021

Added opt-in supported patch replay materialization. This is an M2 bridge scaffold, intentionally limited to file-level operations that already exist in PR-019/PR-020. It does not implement algebraic commutation, conflicted states, text edits, or destructive worktree removals.

## 0.1.0 PR-023

Added a content-anchored text edit validation scaffold. `EditText` now uses a fixed 32-byte old-span hash, anchor IDs are validated, and tests pin basic span-hash stability. Text diff generation, text replay, inverse, commutation, and conflicted merge states remain deferred.


## 0.1.0 PR-024

Added conservative full-file `EditText` replay. Only `anchor_id = "full-file"` is supported, and replay requires the current full file bytes to match the recorded `old_span_hash`. Arbitrary content-span discovery, text-diff generation, inverse, commutation, and conflict witnesses remain deferred.


## 0.1.0 PR-025

Added opt-in full-file `EditText` generation from worktree modifications. The default worktree commit path remains coarse file-level `ReplaceBinary` for modified tracked files. With `--text-edits`, modified tracked files become `EditText` only when both old and new bytes are valid UTF-8; otherwise they fall back to `ReplaceBinary`. Arbitrary span discovery, minimized text diffs, inverse generation, commutation, and conflict witnesses remain deferred.

## 0.1.0 PR-026

Added read-only inverse planning for the supported patch-operation subset. `prikk inverse-plan [path] [--ref REF]` validates the sealed single-parent chain, derives an unsigned inverse Patch payload for `CreateFile`, `DeleteFile`, `ReplaceBinary`, and full-file `EditText`, and reports a deterministic unsigned Patch ID hint. Rollback refs, authorization policy, commutation, confluence, arbitrary-span inverse handling, audit plugins, and sync remain deferred.


## 0.1.0 PR-027

Added non-mutating rollback preview for the supported patch-operation subset. `prikk rollback-preview [path] [--ref REF]` derives the unsigned inverse plan, validates supported replay, and reports file-level `would-create`, `would-delete`, and `would-replace` changes against the latest snapshot baseline. Rollback refs, authorization policy, worktree writes, commutation, confluence, arbitrary-span rollback, audit plugins, and sync remain deferred.


## 0.1.0 PR-028

Added conservative rollback draft append for the supported patch-operation subset. `prikk rollback-draft --append-inverse [path] [--ref REF] -m <message>` derives the supported inverse Patch, validates rollback-preview consistency, requires an empty active WAL, and appends one signed inverse Patch envelope to the active WAL. Rollback refs, authorization policy, worktree writes, arbitrary-span rollback, commutation, confluence, audit plugins, and sync remain deferred.

## 0.1.0 PR-029

Added active rollback draft verification for the supported patch-operation subset. `prikk rollback-draft-verify [path] [--ref REF]` requires an active WAL containing exactly one rollback draft, validates the dedicated rollback signature marker, decodes the Patch payload under the supported replay subset, and compares it with the inverse Patch currently derived from the selected ref. It performs no writes and leaves seal publication, rollback refs, authorization policy, worktree writes, arbitrary-span rollback, commutation, confluence, audit plugins, and sync deferred.
