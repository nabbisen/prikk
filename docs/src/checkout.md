# Checkout Planning

PR-016 includes a read-only checkout plan:

```sh
prikk checkout --plan-only [path] [--ref heads/main]
```

The command validates the current RefState target and reports whether a future checkout would need
snapshot materialization or patch application. It intentionally does not modify the worktree.

For snapshot-backed blocks, use:

```sh
prikk checkout --snapshot-plan [path] [--ref heads/main]
```

That command validates the snapshot manifest and path-safety constraints, but still does not write
files.
