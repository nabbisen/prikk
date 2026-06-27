# Supported Patch Replay Planning

PR-020 adds a read-only patch replay plan for the currently supported file-level operation subset.

The command is:

```sh
prikk checkout --patch-plan [path] [--ref REF]
```

It walks the single-parent block chain from oldest to newest, loads any snapshot Blob attached to a
block, and applies supported Patch operations in block patch order.

Supported operations in PR-020:

- `CreateFile`
- `DeleteFile`
- `ReplaceBinary`

Unsupported operations still fail the plan clearly:

- `EditText`
- `RenamePath`
- `ChangePerm`
- `CreateSymlink`
- merge/conflict algebra

This command does not write the worktree. It only proves that the current sealed history can be
replayed into an in-memory snapshot manifest using the operation subset implemented so far.
