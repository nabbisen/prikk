# Checkout Planning

PR-015 adds a read-only checkout plan:

```sh
prikk checkout --plan-only [path] [--ref heads/main]
```

The command validates the current RefState target and reports whether a future checkout would need
snapshot materialization or patch application. It intentionally does not modify the worktree.
