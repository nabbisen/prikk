# PR-030 Handoff — Sealed Rollback History Classification

## Summary

PR-030 extends rollback observability from the active WAL into sealed history. After a rollback draft is sealed through the existing no-audit seal scaffold, history and repository verification can classify Blocks that contain rollback-marked Patch objects.

## Added / Changed

- `HistoryEntry` now includes:
  - `rollback_patch_count`
  - `is_rollback_block`
- `load_ref_history()` validates rollback-marked Patch payloads while building history entries.
- `RepositoryVerification` now includes:
  - `checked_rollback_blocks`
  - `checked_sealed_rollback_patches`
- `verify_repository()` validates rollback-marked Patch payloads when scanning sealed Blocks.
- `prikk log` displays rollback block classification and rollback Patch counts.
- `prikk verify` displays sealed rollback Block and Patch counts.
- Active rollback draft verification and sealed rollback classification now share rollback Patch payload validation.
- Fixed one obvious duplicate-parameter transcription defect in inverse planning source while touching rollback-adjacent code.

## Safety Boundary

This PR is read-only for history and verification. It performs no rollback publication, no worktree writes, no WAL mutation, and no authorization policy change.

Rollback-specific refs and authorization remain deferred.

## Validation Intent

The new tests cover:

- sealed rollback Patch classification in history
- sealed rollback Block/Patch counts in repository verification

Please run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Deferred

- rollback-specific ref publication policy
- rollback authorization and audit policy
- rollback worktree materialization
- arbitrary-span text rollback
- commutation / confluence / conflict witnesses
- audit plugins and sync
