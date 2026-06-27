# PR-028 Rollback Draft Handoff

## Summary

PR-028 adds a conservative mutating rollback-draft boundary. It turns the supported inverse Patch
payload from PR-026 into a signed Patch envelope and appends it to the active WAL, guarded by an
explicit CLI flag and an empty-WAL requirement.

## Added API

- `append_rollback_draft(layout, ref_name, message)`
- `RollbackDraftReport`

## Added CLI

```sh
prikk rollback-draft --append-inverse [path] [--ref REF] -m "rollback message"
```

## Behavior

The command:

1. validates the supported inverse plan;
2. validates rollback preview consistency;
3. refuses empty rollback messages;
4. acquires the active-session lock;
5. refuses non-empty active WALs;
6. refuses active WALs with partial tails;
7. signs the inverse Patch with the current development placeholder author signature;
8. appends the signed Patch envelope to the active WAL.

## Safety Boundary

The command does not:

- publish refs;
- write object files directly;
- mutate worktree files;
- delete files;
- implement rollback authorization policy;
- implement rollback-specific ref naming or branch policy.

The next publication step remains the existing `prikk seal --allow-no-audit` scaffold.

## Tests

Added tests cover:

- appending a supported file-operation inverse Patch to an empty WAL;
- appending a full-file text inverse Patch draft;
- refusing rollback draft append when the active WAL is non-empty.

## Deferred Work

- rollback ref policy;
- rollback authorization and audit rules;
- rollback materialization policy;
- arbitrary-span text inverse support;
- commutation, confluence, and conflict witnesses;
- audit plugin execution;
- remote sync.
