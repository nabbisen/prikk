# Worktree Status

PR-018 adds read-only worktree status against snapshot-backed baselines.

```sh
prikk worktree-status [path] [--ref REF]
```

The command compares the current worktree with the snapshot manifest referenced by the selected ref.
It reports missing, modified, untracked, and unsupported paths.
It does not write the worktree. Patch generation is handled separately by `prikk commit --from-worktree`.

The scanner is intentionally conservative:

- `.prikk/` metadata is ignored;
- existing path-safety validation is reused;
- non-ASCII paths remain unsupported until Unicode NFC normalization is implemented;
- no writes are performed.

For the exact repository path validator rules, see
[path and worktree safety](../reference/path-safety.md).
