# PR-018 Worktree Status Handoff

## Scope

PR-018 adds a read-only worktree status scaffold against snapshot-backed blocks.
It compares the current worktree with the validated snapshot manifest referenced by a published ref.
It does not create patch operations, apply patch algebra, or modify the worktree.

## Implementation Notes

- Added `prikk_store::worktree_status()`.
- Added `WorktreeStatusReport`, `WorktreeChange`, and `WorktreeChangeKind`.
- Added CLI command `prikk worktree-status [path] [--ref REF]`.
- The scanner ignores `.prikk/` metadata.
- The scanner reports:
  - missing tracked files;
  - modified tracked files;
  - untracked worktree files;
  - unsupported paths that cannot be represented as safe PRIKK paths.
- Status is based only on snapshot manifests. Patch-replay baselines remain deferred.

## Acceptance / QA Checklist

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Manual smoke path:

```sh
cargo run -p prikk -- init ./sample-repo
# publish a snapshot-backed block through tests/fixtures or a later helper, then:
cargo run -p prikk -- worktree-status ./sample-repo
```

Expected behavior:

- clean snapshot materialization reports no changes;
- modifying a tracked file reports `modified`;
- deleting a tracked file reports `missing`;
- adding an untracked file reports `untracked`;
- unsafe paths are reported instead of silently used.

## Deferred

- Real diff capture into PatchPayload operations.
- Patch replay status for non-snapshot blocks.
- Unicode NFC normalization beyond the current conservative ASCII path subset.
- Ignore patterns.
- Rename detection.
- Audit plugins and sync.
