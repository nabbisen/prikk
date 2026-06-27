# Supported Patch Deletions

PR-022 adds an explicit deletion plan for the supported patch replay result:

```sh
prikk checkout --patch-delete-plan [path] [--ref REF]
```

It also adds an opt-in materialization mode that removes eligible files:

```sh
prikk checkout --patch-materialize-delete [path] [--ref REF]
```

Deletion is intentionally narrow. PRIKK deletes only files that the replayed patch chain removed
with a `DeleteFile` operation. Before removing a worktree file, PRIKK checks that the current file
bytes still match the operation's old Blob precondition.

Safety boundaries:

- Arbitrary untracked files are never deleted.
- Modified deleted files are refused.
- Symlink targets and non-file targets are refused.
- General checkout pruning remains deferred.
- Text edits, renames, chmod, symlinks, merge conflicts, inverse logic, and full patch algebra remain later increments.
