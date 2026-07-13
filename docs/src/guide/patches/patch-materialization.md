# Supported Patch Materialization

PR-021 added an explicit materialization command for the supported patch replay result:

```sh
prikk checkout --patch-materialize [path] [--ref REF]
```

This command reuses the read-only replay support from `checkout --patch-plan` and writes the
resulting file manifest into the worktree through the same conservative materializer used by
snapshot checkout.

PR-022 adds deletion-aware materialization as a separate opt-in command:

```sh
prikk checkout --patch-materialize-delete [path] [--ref REF]
```

Supported operation subset:

- `CreateFile`
- `DeleteFile`
- deterministic arbitrary-span `EditText`

Safety boundaries:

- Existing files with identical bytes are left unchanged.
- Existing files with different bytes are refused.
- `--patch-materialize` never deletes files.
- `--patch-materialize-delete` deletes only files explicitly removed by replayed `DeleteFile` operations.
- Deletion is refused unless the current worktree bytes still match the old Blob precondition.
- Extra untracked files are never deleted.
- Symlinked parents, symlink targets, non-file targets, and `.prikk/` metadata paths remain refused.
- `ReplaceBinary`, renames, chmod, symlinks, merge conflicts, inverse logic, and full patch algebra remain later increments.

This command is useful for exercising the current Prikk object/WAL/ref/block pipeline end-to-end,
but it is not yet a complete checkout implementation. For the shared write-safety boundary and its
race caveats, see [path and worktree safety](../../reference/path-safety.md).
