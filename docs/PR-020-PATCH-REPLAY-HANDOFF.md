# PR-020 Patch Replay Handoff

## Summary

PR-020 adds a read-only patch replay planning boundary for the operation subset currently emitted by
`prikk commit --from-worktree`: `CreateFile`, `DeleteFile`, and `ReplaceBinary`.

## Implementation scope

- Add `prepare_patch_replay_plan()`.
- Walk the current ref's single-parent block chain oldest-to-newest.
- Load snapshot Blob baselines when present.
- Decode supported patch operations from persisted Patch objects.
- Apply operations into an in-memory snapshot manifest.
- Verify `old_blob_id` preconditions for delete and replace operations.
- Add CLI `prikk checkout --patch-plan`.

## Out of scope

- Worktree writes.
- Text-span edit replay.
- Rename/chmod/symlink replay.
- Multi-parent merge replay.
- Conflict witnesses and algebraic commutation.
- Audit plugins and sync.

## QA checklist

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- Manual smoke: snapshot block -> worktree patch commit -> seal -> `checkout --patch-plan`.
