# Checkout Planning

PR-017 includes a read-only checkout plan:

```sh
prikk checkout --plan-only [path] [--ref heads/main]
```

The command validates the current RefState target and reports whether checkout would need snapshot
materialization or patch application.

For snapshot-backed blocks, first validate the snapshot manifest:

```sh
prikk checkout --snapshot-plan [path] [--ref heads/main]
```

Then explicitly materialize validated snapshot files:

```sh
prikk checkout --snapshot-materialize [path] [--ref heads/main]
```

Snapshot materialization writes only validated regular files. Supported patch replay and
materialization are available through `--patch-plan`, `--patch-materialize`, and
`--patch-materialize-delete`, but full patch algebra remains deferred. For the shared path and
worktree safety boundary, see the [path and worktree safety](../../reference/path-safety.md) reference.
