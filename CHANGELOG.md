# Changelog

## 0.1.0 PR-030

Sealed rollback block/history classification after normal seal.

- Added sealed rollback block classification after rollback drafts are sealed by the existing seal path.
- `load_ref_history()` now reports `rollback_patch_count` and `is_rollback_block` for each history entry.
- `prikk log` now displays rollback block classification for sealed history entries.
- `verify_repository()` now counts sealed rollback blocks and sealed rollback-marked Patch objects.
- `prikk verify` now displays sealed rollback block and rollback Patch counts in addition to active rollback draft WAL records.
- Shared rollback Patch payload validation between active WAL verification and sealed Block/history classification.
- Fixed an obvious duplicate-parameter transcription defect in inverse planning source while touching rollback-adjacent code.
- Kept rollback-specific ref publication, rollback authorization, worktree rollback writes, arbitrary-span rollback, commutation, confluence, audit plugins, and sync deferred.

## 0.1.0 PR-029

Active rollback draft verification before seal.

- Added active rollback draft verification for the supported patch-operation subset.
- Added `verify_active_rollback_draft()` and `RollbackDraftVerification`.
- Added CLI command `prikk rollback-draft-verify [path] [--ref REF]`.
- Rollback drafts now use a dedicated development signature marker key: `dev-placeholder-rollback-author`.
- Repository verification now counts rollback draft WAL records and validates that rollback draft payloads decode under the supported replay subset.
- Kept seal publication, rollback refs, rollback authorization, worktree mutation, arbitrary-span rollback, commutation, confluence, audit plugins, and sync deferred.

## 0.1.0 PR-028

Conservative rollback draft append to an empty active WAL.

- Added conservative rollback draft append for the supported patch-operation subset.
- Added `append_rollback_draft()` and `RollbackDraftReport`.
- Added CLI command `prikk rollback-draft --append-inverse [path] [--ref REF] -m <message>`.
- Requires an explicit `--append-inverse` flag, a non-empty message, an empty active WAL, and no partial WAL tail.
- Appends a signed inverse Patch envelope to the active WAL only; ref publication remains the existing `seal --allow-no-audit` path.
- Kept rollback ref policy, authorization, worktree mutation, arbitrary-span rollback, commutation, confluence, audit plugins, and sync deferred.

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

## Earlier PRs

See `rfcs/IMPLEMENTATION-STATUS.md` and `rfcs/done/PR-*-HANDOFF.md` for earlier implementation history.
