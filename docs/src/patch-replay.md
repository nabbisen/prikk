# Supported Patch Replay Planning

PR-026 keeps read-only patch replay for the current conservative operation subset.

The command is:

```sh
prikk checkout --patch-plan [path] [--ref REF]
```

It walks the single-parent block chain from oldest to newest, loads any snapshot Blob attached to a
block, and applies supported Patch operations in block patch order.

Supported operations in PR-026:

- `CreateFile`
- `DeleteFile`
- `ReplaceBinary`
- `EditText` for full-file exact-span replacements only (`anchor_id = "full-file"`)

Unsupported operations still fail the plan clearly:

- arbitrary `EditText` anchors
- `RenamePath`
- `ChangePerm`
- `CreateSymlink`
- merge/conflict algebra

This command does not write the worktree. It only proves that the current sealed history can be
replayed into an in-memory snapshot manifest using the operation subset implemented so far.
