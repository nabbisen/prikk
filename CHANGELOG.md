# Changelog

## 0.1.0 PR-027

Non-mutating rollback preview for the supported patch-operation subset.

- Added non-mutating rollback preview for the supported patch-operation subset.
- Added `prepare_rollback_preview()` and `RollbackPreviewPlan`.
- Added CLI command `prikk rollback-preview [path] [--ref REF]`.
- Combines unsigned inverse planning with supported patch replay validation.
- Compares the current replayed target state with the latest snapshot baseline and reports `would-create`, `would-delete`, and `would-replace` file-level changes.
- Kept rollback refs, authorization policy, worktree writes, commutation, confluence, arbitrary-span rollback, audit plugins, and sync deferred.

## 0.1.0 PR-026

Read-only inverse planning for the supported patch-operation subset.

- Added read-only inverse planning for the supported patch-operation subset.
- Added `prepare_patch_inverse_plan()` and `PatchInversePlan`.
- Added CLI command `prikk inverse-plan [path] [--ref REF]`.
- Derives unsigned inverse Patch payloads for supported `CreateFile`, `DeleteFile`, `ReplaceBinary`, and full-file `EditText` operations.
- Reports an unsigned inverse Patch ID hint without writing or publishing it.
- Kept rollback refs, authorization policy, conflict witnesses, commutation, confluence, arbitrary-span inverse handling, audit plugins, and sync deferred.

## 0.1.0 PR-025

Opt-in full-file `EditText` generation from UTF-8 worktree modifications.

- Added opt-in full-file `EditText` generation from snapshot-baseline worktree modifications.
- Added `WorktreePatchCommitOptions` and `commit_worktree_changes_with_options()`.
- Added CLI support for `prikk commit --from-worktree --text-edits -m <message>`.
- Kept default `commit --from-worktree` behavior compatible: modified tracked files still emit `ReplaceBinary` unless text mode is requested.
- Text mode emits `EditText` only when both baseline and current file bytes are valid UTF-8; binary or invalid UTF-8 modifications fall back to `ReplaceBinary`.
- Added worktree patch tests for text edit emission and binary fallback.
- Kept arbitrary span discovery, text diff minimization, inverse, commutation, conflict witnesses, audit plugins, and sync deferred.

## 0.1.0 PR-024

Conservative full-file `EditText` replay for exact-span replacements.

- Added conservative `EditText` replay for full-file exact-span replacements.
- Added canonical decode support for `EditText` patch operations in the supported patch replay decoder.
- Added `full-file` anchor replay validation: current file bytes must be valid UTF-8 and must hash to the recorded `old_span_hash`.
- Split supported patch-operation decoding into `patch_replay/decode.rs` to keep the replay module within the project file-size guidance.
- Added a patch replay test for full-file text edit replay.
- Kept worktree text diff generation, arbitrary span discovery, inverse, commutation, conflict witnesses, audit plugins, and sync deferred.

## 0.1.0 PR-023

Explicit patch deletion planning and opt-in removal of files deleted by supported patch replay.

- Added a content-anchored text edit payload validation scaffold.
- Added fixed `TEXT_SPAN_HASH_BYTES = 32` and `text_span_hash(bytes)`.
- Added `validate_text_anchor_id()` for v1 anchor identifier validation.
- Changed `EditText.old_span_hash` to a fixed 32-byte value.
- Added tests for anchor validation, stable span hashing, and invalid anchor rejection.
- Fixed a replay-source transcription defect in the supported `ReplaceBinary` branch.
- Kept worktree text diff generation, text replay, inverse, commutation, conflict witnesses, audit plugins, and sync deferred.

## 0.1.0 PR-022

Explicit patch deletion planning and opt-in removal of files deleted by supported patch replay.

- Added read-only explicit deletion planning via `prikk checkout --patch-delete-plan`.
- Added opt-in deletion during supported patch materialization via `prikk checkout --patch-materialize-delete`.
- Deletion is limited to files explicitly removed by replayed `DeleteFile` operations.
- Deletion is refused unless the current worktree file bytes still match the operation's old Blob bytes.
- Arbitrary untracked files and modified deleted files are never removed.
- Added deletion planning/materialization tests and documentation.
- Kept general destructive pruning, text edits, renames, chmod, symlinks, merge/conflict algebra, audit plugins, and sync deferred.

## 0.1.0 PR-021

Opt-in supported patch replay materialization without destructive removals.

- Added opt-in supported patch replay materialization via `prikk checkout --patch-materialize`.
- Added `materialize_patch_checkout()` and `PatchMaterializationReport`.
- Reuses the PR-020 supported replay subset: `CreateFile`, `DeleteFile`, and `ReplaceBinary`.
- Writes only validated replay-result files through the same conservative materializer used by snapshot checkout.
- Refuses conflicting existing files and never deletes extra worktree files.
- Keeps destructive removal, content-anchored text edit replay, renames, chmod, symlinks, merge/conflict algebra, audit plugins, and sync deferred.

## 0.1.0 PR-020

Minimal worktree-to-patch draft generation for missing, modified, and untracked files, still without patch replay or full algebra.

- Added read-only supported patch replay planning via `prikk checkout --patch-plan`.
- Added `prepare_patch_replay_plan()` and `PatchReplayPlan`.
- Replays single-parent block chains from oldest to newest.
- Loads snapshot Blob baselines and applies supported `CreateFile`, `DeleteFile`, and `ReplaceBinary` operations.
- Verifies `old_blob_id` preconditions for delete/replace operations.
- Keeps text-span edits, renames, chmod, symlinks, merge/conflict algebra, and worktree writes deferred.

## 0.1.0 PR-019

Minimal worktree-to-patch draft generation for missing, modified, and untracked files, still without patch replay or full algebra.

- Added minimal worktree-to-patch draft generation from snapshot-baseline changes.
- Added `prikk commit --from-worktree -m <message>`.
- Emits file-level `CreateFile`, `DeleteFile`, and `ReplaceBinary` operations only.
- Writes Blob objects referenced by generated operations before appending the Patch envelope to WAL.
- Keeps rename detection, content-anchored text-span edits, patch replay, audit plugins, and sync deferred.

## 0.1.0 PR-018

Read-only worktree status against snapshot-backed baselines, still without patch operation generation or patch replay.

- Added read-only worktree status via `prikk worktree-status [path] [--ref REF]`.
- Added `worktree_status()`, `WorktreeStatusReport`, `WorktreeChange`, and `WorktreeChangeKind`.
- Status compares the current worktree with a snapshot manifest baseline.
- Reports missing tracked files, modified tracked files, untracked files, and unsupported unsafe paths.
- Ignores `.prikk/` metadata and performs no writes.
- Added tests for clean, modified, missing, and untracked snapshot-baseline worktrees.
- Kept patch operation generation, patch replay status, Unicode NFC normalization, audit plugins, and sync deferred.

## 0.1.0 PR-017

Opt-in snapshot materialization via `prikk checkout --snapshot-materialize`, still without patch replay.

- Added opt-in snapshot materialization via `prikk checkout --snapshot-materialize [path] [--ref REF]`.
- Added `materialize_snapshot_checkout()` and `SnapshotMaterializationReport`.
- Snapshot materialization writes only files from validated snapshot manifests.
- Refuses to overwrite existing files with different content.
- Refuses symlinked parent directories, symlink targets, non-file targets, and paths under `.prikk/`.
- Added tests for writing new files, idempotent materialization, conflicting existing files, and metadata-path rejection.
- Kept patch replay, Unicode NFC normalization, audit plugins, and sync deferred.

## 0.1.0 PR-016

snapshot-manifest validation and path-safety scaffolding via `prikk checkout --snapshot-plan`.

- Added conservative repository-relative path validation for future worktree materialization.
- Added snapshot-manifest decode/encode scaffolding stored inside Blob objects.
- Added read-only snapshot checkout planning via `prepare_snapshot_checkout_plan()`.
- Added CLI command `prikk checkout --snapshot-plan [path] [--ref REF]`.
- Snapshot planning validates absolute/traversal paths, Windows reserved names, duplicate paths, and case-insensitive collisions.
- Kept actual worktree writes, snapshot materialization, real diff capture, patch apply/inverse/commutation, audit plugins, and sync deferred.

## 0.1.0 PR-015

Read-only checkout planning via `prikk checkout --plan-only`.

- Added read-only checkout planning via `prepare_checkout_plan()`.
- Added CLI command `prikk checkout --plan-only [path] [--ref REF]`.
- Checkout planning validates the current RefState, target Block, parent Block references, Patch references, and optional snapshot Blob references.
- The plan reports why real worktree materialization is still blocked: unpublished ref, no-op block, snapshot materialization, or patch engine.
- Kept real worktree writes, diff capture, patch apply/inverse/commutation, audit plugins, and sync deferred.

## 0.1.0 PR-014

Read-only sealed-history inspection from the current RefState chain via `prikk log`.

- Added read-only sealed-history inspection via `load_ref_history()`.
- Added CLI command `prikk log [path] [--limit N] [--ref REF]`.
- History follows the current RefState chain newest-first and validates each target Block.
- Added history tests for newest-first ordering and limit handling.
- Kept real worktree diff capture, patch apply/inverse/commutation, audit plugins, and sync deferred.

## 0.1.0 PR-013

Opt-in safe doctor repair for incomplete active-WAL trailing bytes and missing `heads/main` pointer reconstruction from verified ref-log data.

- Added guarded ref-pointer reconstruction for a missing `heads/main` pointer.
- Added `RefRecoveryCandidate` and `RefRecoveryRepair`.
- Added doctor warning `PRIKK-DOCTOR-REF-POINTER-MISSING` when the ref log and RefState object are valid but the pointer is absent.
- Added `prikk doctor [path] --repair-main-ref` for opt-in pointer reconstruction.
- Added tests for direct RefStore reconstruction and doctor-driven reconstruction.
- Kept malformed-log repair, missing-object repair, ref-log rewriting, patch algebra, plugins, and sync deferred.

## 0.1.0 PR-012

Opt-in safe doctor repair for incomplete active-WAL trailing bytes. Doctor remains conservative and refuses repair when verification reports integrity errors.

- Added opt-in safe doctor repair via `prikk doctor [path] --repair-wal-tail`.
- Added `WalRepair` and `Wal::truncate_trailing_partial()` for incomplete final WAL records.
- Added `DoctorRepairOptions`, `DoctorRepairReport`, and `repair_repository()`.
- Doctor repair refuses to mutate the repository when verification reports integrity errors.
- Added tests for preserving valid WAL records while truncating only incomplete trailing bytes.
- Split `prikk-store` tests into logical modules under `src/tests/` to follow project file-size guidance.
- Kept ref reconstruction, missing-object repair, checksum-mismatch repair, patch algebra, plugins, and sync deferred.

## 0.1.0 PR-011

Read-only doctor diagnostics for repository health. Doctor wraps verification results into actionable issue codes without modifying repository data.

- Added read-only doctor diagnostics on top of repository verification.
- Added `DoctorReport`, `DoctorIssue`, and `DoctorSeverity` for structured health reporting.
- Added CLI command `prikk doctor [path]`.
- Doctor reports verification errors as actionable diagnostics without modifying the repository.
- Doctor warns about trailing partial active-WAL bytes while leaving repair deferred.
- Added doctor tests for healthy repositories, partial WAL warnings, and verification errors.
- Kept destructive repair, real diff capture, patch algebra, audit plugins, and sync deferred.

## 0.1.0 PR-010

Verification hardening for the local no-audit seal scaffold. Verification now checks block references, RefUpdate-to-RefState links, target block existence, and persisted WAL patch counts.

- Strengthened read-only repository verification after the no-audit seal scaffold.
- Added BlockPayload canonical decoding.
- Verification now checks that persisted Block objects reference existing patch, parent-block, and snapshot-blob objects.
- Added RefUpdatePayload canonical decoding.
- Ref-log verification now validates decoded RefUpdate payloads against their referenced RefState objects and target blocks.
- `prikk verify` now reports checked block count and active WAL records already persisted as patch objects.
- Kept real worktree materialization, audit plugins, patch algebra, and sync deferred.

## 0.1.0 PR-009

local no-audit seal scaffold that persists active WAL patch envelopes, creates a Block, publishes `heads/main`, and clears the active WAL after success.

- Added `prikk seal --allow-no-audit` as a local no-audit seal scaffold.
- Seal persists signed patch envelopes from the active WAL into the object store.
- Seal creates a signed Block object with deterministic scaffold state root.
- Seal publishes `heads/main` through signed RefState and inline RefUpdate records.
- Seal truncates the active WAL only after the ref publication succeeds.
- Added RefState canonical decoding for current-head parent discovery.
- Kept real worktree materialization, audit plugins, patch algebra, and sync deferred.

## 0.1.0 PR-008

Narrow empty-commit scaffold that appends a signed patch envelope to the active WAL under `active.lock`.

- Added `ActiveSession` as the locked default active-session append boundary.
- Added CLI `prikk commit --allow-empty -m <message>` as a narrow commit-path scaffold.
- The commit scaffold writes a signed patch envelope to the active WAL and reports the WAL sequence.
- Updated `status` and documentation to reflect PR-008.
- Kept real diff capture, seal, patch algebra, plugin/audit, and sync deferred.

## 0.1.0 PR-007

Initial RefState publication primitives, flat ref pointer layout, and inline RefUpdate log verification.

- Added `RefStore` for initial RefState publication primitives.
- Added flat `refs/by-id/<sha256(ref_name)>.ref` pointer layout helpers.
- Added ref-specific lock scaffold via `RefLock`.
- Added durable ref pointer candidate write and atomic promotion.
- Added inline signed RefUpdate log append/replay with checksums.
- Extended repository verification to check ref pointers and ref-log records.
- Updated CLI `status` and `verify` output with ref-state/ref-log information.
- Added PR-007 ref publication handoff notes.

## 0.1.0 PR-006

Read-only repository verification for persisted objects and active WAL records.

- Added read-only repository verification in `prikk-store`.
- Added object-store scan across persisted object type directories.
- Added verification that object file paths match computed object IDs and canonical fanout paths.
- Added verification that object envelope types match their object directories.
- Added active WAL replay verification summary.
- Added CLI command `prikk verify [path]`.
- Added PR-006 verification handoff notes.

## 0.1.0 PR-005

Storage cleanup and active-session WAL append/replay.

- Fixed `cargo fmt --check` drift reported against PR-004.
- Removed unused `ByteCursor::remaining` so strict Clippy dead-code checks pass.
- Fixed `prikk init [path]` argument handling so CLI tests compile.
- Updated implementation status and handoff notes for the CI feedback round.

## 0.1.0 PR-004

Storage cleanup and active-session WAL append/replay.

- Aligned workspace metadata with project Rust instructions: Rust 2024, Apache-2.0, author nabbisen.
- Split large `prikk-store` and `prikk-object` payload files by logical boundaries.
- Added `LICENSE`, `NOTICE`, and mdBook-compatible `docs/src` seed.
- Fixed PR-003 storage transcription defects before extending storage work.
- Added active-session file lock scaffold.
- Added file-backed WAL append/replay for signed patch envelopes.
- Added WAL tests for signed patch round-trip and unsigned-patch rejection.
- Updated CLI status output to report active WAL replay state.

## 0.1.0 PR-003

- Added `.prikk/` repository layout creation.
- Added file-backed object store.
- Added persistent object envelope file codec.
- Added minimal `prikk init` and `prikk status` commands.

## 0.1.0 PR-002

- Fixed formatting and strict Clippy warnings from PR-001 logs.
- Added missing documentation in `prikk-error` and CLI crate.

## 0.1.0 PR-001

- Added initial Rust workspace scaffold.
- Added object ID formula seed.
- Added deterministic canonical encoding seed.
- Added object envelope and in-memory object store.
