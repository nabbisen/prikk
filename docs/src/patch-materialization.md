# Supported Patch Materialization

PR-021 adds an explicit materialization command for the supported patch replay result:

```sh
prikk checkout --patch-materialize [path] [--ref REF]
```

This command reuses the read-only replay support from `checkout --patch-plan` and writes the
resulting file manifest into the worktree through the same conservative materializer used by
snapshot checkout.

Supported operation subset:

- `CreateFile`
- `DeleteFile`
- `ReplaceBinary`

Safety boundaries:

- Existing files with identical bytes are left unchanged.
- Existing files with different bytes are refused.
- Extra worktree files are never deleted.
- Symlinked parents, symlink targets, non-file targets, and `.prikk/` metadata paths remain refused.
- Text edits, renames, chmod, symlinks, merge conflicts, inverse logic, and full patch algebra remain later increments.

This command is useful for exercising the current PRIKK object/WAL/ref/block pipeline end-to-end,
but it is not yet a complete checkout implementation.
