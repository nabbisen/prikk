# History Inspection

PR-014 adds a small read-only history view for early sealed repositories.

```sh
prikk log [path] [--limit N] [--ref REF]
```

The command follows the current `RefState` chain from newest to oldest and validates that each
entry targets a persisted Block object. It does not yet perform full block-DAG traversal,
path-aware history queries, or patch algebra.
