# PR-029 Handoff — Rollback Draft Verification

## Summary

PR-029 adds a conservative pre-seal verification path for rollback drafts. It does not publish refs, append WAL entries, write objects, or mutate the worktree.

## Added API

- `prikk_store::verify_active_rollback_draft(layout, ref_name)`
- `prikk_store::RollbackDraftVerification`

The API verifies that the active WAL contains exactly one rollback draft Patch and that its canonical payload exactly matches the inverse Patch derived from the selected published ref.

## CLI

```sh
prikk rollback-draft-verify [path] [--ref REF]
```

## Repository verification integration

`verify_repository()` now reports `checked_rollback_draft_records`. Rollback draft WAL records are classified by the dedicated development signature marker and decoded under the supported replay subset.

## Safety notes

- Rollback drafts now use the dedicated marker key `dev-placeholder-rollback-author`.
- `rollback-draft-verify` refuses non-empty mixed WALs.
- `rollback-draft-verify` refuses partial WAL tails.
- Repository verification validates rollback draft decoding but does not compare against a ref unless the explicit verification API/CLI is used.

## Deferred

- rollback-specific ref policy;
- rollback authorization and audit rules;
- rollback worktree writes;
- arbitrary-span rollback;
- commutation, confluence, and conflict witnesses;
- remote sync.
