# Sealed Rollback History

Prikk provides read-only classification for rollback Patch objects after they are sealed into a normal
Block by the existing seal path.

Rollback identity lives in the Patch payload as `PatchPurpose::RollbackDraft`; it is not encoded in an
AUTHOR key id. When a sealed Block references such a Patch, Prikk verifies that the Patch payload decodes
under the supported replay subset and then classifies the Block as a rollback Block for history and
verification output.

Pre-DC-10 rollback drafts that used the old development key-id marker are pre-stability artifacts. Current
classification does not recognize that marker in production logic.

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
