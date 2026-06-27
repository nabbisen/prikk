# Sealed Rollback History

PR-030 adds read-only classification for rollback Patch objects after they are sealed into a normal Block by the existing seal path.

Rollback drafts remain ordinary Patch payloads until sealed. The only marker currently used by the development scaffold is the dedicated author key ID:

```text
dev-placeholder-rollback-author
```

When a sealed Block references a Patch envelope carrying that marker, Prikk verifies that the Patch payload decodes under the supported replay subset and then classifies the Block as a rollback Block for history and verification output.

## CLI

```sh
prikk log [path] [--ref REF]
prikk verify [path]
```

`prikk log` reports, per history entry:

```text
rollback-block: true|false
rollback-patches: N
```

`prikk verify` reports repository-wide counts:

```text
checked rollback blocks: N
checked sealed rollback patches: N
checked rollback draft WAL records: N
```

The sealed counts cover persisted Blocks and Patch objects. The active draft count covers the active WAL before seal.

## Scope

PR-030 is intentionally classification-only. It does not introduce rollback-specific refs, rollback authorization, rollback worktree writes, or rollback-specific seal semantics.

Deferred work remains:

- rollback-specific ref policy
- rollback authorization and audit policy
- worktree rollback writes
- arbitrary-span text rollback
- commutation / confluence / conflict witnesses
- audit plugins and sync
