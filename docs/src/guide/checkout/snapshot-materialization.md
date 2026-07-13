# Snapshot Materialization

PR-017 adds an explicit, opt-in snapshot materialization path:

```sh
prikk checkout --snapshot-materialize [path] [--ref REF]
```

The command writes files only from a validated snapshot manifest. It does not apply patch algebra,
does not remove extra files, and refuses to overwrite existing files with different content. It also
refuses symlinked parent directories and symlink targets so snapshot checkout cannot be used to
write outside the repository worktree.

The path validator remains conservative: non-ASCII paths are deferred until Unicode NFC
normalization is implemented, and paths targeting `.prikk/` are rejected. For the exact validator and
write-safety caveats, see [path and worktree safety](../../reference/path-safety.md).
