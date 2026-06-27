# PR-022 Patch Deletion Handoff

## Implementation Handoff

PR-022 adds explicit deletion planning and opt-in deletion for the supported patch replay subset.
Deletion is intentionally limited to files removed by replayed `DeleteFile` operations. It is not a
general worktree prune.

Implemented commands:

- `prikk checkout --patch-delete-plan [path] [--ref REF]`
- `prikk checkout --patch-materialize-delete [path] [--ref REF]`

## Task Breakdown / PR Plan

1. Extend supported patch replay to remember files removed by `DeleteFile` operations.
2. Add read-only deletion planning that classifies removed files as deletable, already absent, or refused.
3. Refuse deletion of symlinks, non-files, and files whose current bytes do not match the old Blob precondition.
4. Add opt-in deletion materialization after successful preflight.
5. Add CLI parsing and output for the new commands.
6. Add tests for safe deletion, already existing materialization behavior, and conflict refusal.
7. Update README, roadmap, mdBook docs, and implementation status.

## Acceptance / QA Checklist

- `cargo fmt --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- `--patch-delete-plan` reports only explicit patch deletions.
- `--patch-materialize-delete` removes a deleted file only when bytes match the old Blob.
- Modified deleted files are refused before write/delete work begins.
- Arbitrary untracked files are preserved.
- Full patch algebra remains deferred.
