# Rollback Preview

PR-027 adds a non-mutating rollback preview for the supported patch-operation subset.

The command is:

```sh
prikk rollback-preview [path] [--ref REF]
```

The preview performs two read-only validations:

1. derive the unsigned inverse Patch payload for the supported operation subset;
2. replay the current target state from the supported single-parent block chain.

It then compares the current replayed state with the latest snapshot baseline in that replay
window. The result is a file-level preview of what rollback would need to create, delete, or
replace.

No repository state is changed. The command does not write objects, append WAL records, publish
refs, or modify the worktree.

Supported operation subset:

- `CreateFile`
- `DeleteFile`
- deterministic arbitrary-span `EditText`

Deferred:

- mutating rollback commands
- rollback ref publication policy
- authorization and audit policy for rollback
- commutation, confluence, and conflict witnesses
- plugin execution and remote sync
