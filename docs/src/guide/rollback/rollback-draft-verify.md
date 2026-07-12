# Rollback Draft Verification

`prikk rollback-draft-verify` is a non-mutating verification step for rollback drafts that are waiting in
the active WAL.

```sh
prikk rollback-draft-verify [path] [--ref REF]
```

The command verifies that:

- the active WAL has no trailing partial record;
- the active WAL contains exactly one record;
- the record is a Patch envelope;
- the Patch payload carries `PatchPurpose::RollbackDraft`;
- the Patch carries an AUTHOR signature and reports its real key id;
- the Patch payload decodes under the currently supported replay subset;
- the Patch payload exactly matches the inverse Patch currently derived from the selected ref.

This makes the rollback draft path easier to audit before `seal --allow-no-audit` publishes the active WAL
into a block.

## Repository verification integration

`prikk verify` also counts active WAL records classified as rollback drafts. For those records, verification decodes the Patch payload purpose and the supported replay subset. This check is intentionally weaker than `rollback-draft-verify` because repository-level verification has no selected ref target.

`prikk verify` also counts sealed rollback Blocks and sealed rollback Patch references after a rollback draft has been sealed through the existing seal path.
The broader repository verification and doctor diagnostic boundary is described in the
[integrity and recovery diagnostics](../../reference/integrity-recovery.md) reference.

## Current limits

Rollback draft verification still does not implement:

- rollback-specific ref publication;
- rollback authorization policy;
- audit-plugin approval;
- rollback worktree mutation;
- arbitrary-span text rollback;
- commutation, confluence, or conflict witnesses.
