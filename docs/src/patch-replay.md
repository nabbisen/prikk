# Supported Patch Replay Planning

Read-only patch replay handles the current conservative operation subset, including DC-12
arbitrary-span text edits.

The command is:

```sh
prikk checkout --patch-plan [path] [--ref REF]
```

It walks the single-parent block chain from oldest to newest, loads any snapshot Blob attached to a
block, and applies supported Patch operations in block patch order.

Supported operations:

- `CreateFile`
- `DeleteFile`
- `EditText` for deterministic content-anchored arbitrary spans

Unsupported operations still fail the plan clearly:

- `ReplaceBinary`
- `RenamePath`
- `ChangePerm`
- `CreateSymlink`
- direct inverse/rollback for arbitrary-span text edits
- merge/conflict algebra

This command does not write the worktree. It only proves that the current sealed history can be
replayed into an in-memory snapshot manifest using the operation subset implemented so far.
