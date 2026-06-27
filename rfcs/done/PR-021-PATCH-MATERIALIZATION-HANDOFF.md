# PR-021 Patch Materialization Handoff

## Scope

PR-021 adds opt-in materialization from the supported patch replay result:

- `prikk checkout --patch-materialize [path] [--ref REF]`
- `materialize_patch_checkout()`
- `PatchMaterializationReport`

The command materializes only the replayed final manifest for the current supported operation subset:
`CreateFile`, `DeleteFile`, and `ReplaceBinary`.

## Safety Rules

- Refuses existing files whose bytes differ from the replay result.
- Leaves identical existing files unchanged.
- Never deletes extra worktree files.
- Uses the existing safe materialization boundary for parent directories and symlink checks.
- Keeps text edits, renames, chmod, symlinks, conflicts, and destructive removal deferred.

## PR Plan

1. Expose an internal replay-to-manifest boundary from `patch_replay`.
2. Reuse the shared safe manifest materializer.
3. Add `patch_checkout` module and public report type.
4. Add CLI parsing/output for `--patch-materialize`.
5. Add tests for write, idempotent, and conflict-refusal cases.
6. Update docs and status.

## Acceptance / QA Checklist

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `prikk checkout --patch-plan` still works.
- `prikk checkout --patch-materialize` writes replay-result files when the worktree is empty or already identical.
- `prikk checkout --patch-materialize` refuses conflicting existing files.
- Extra files are not removed.

## Deferred

- File deletion/removal during checkout.
- Patch replay for text edits, renames, chmod, and symlinks.
- Merge/conflict algebra.
- Audit/plugin enforcement.
- Remote sync.
