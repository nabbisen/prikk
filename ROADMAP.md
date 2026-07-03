# Prikk Roadmap

This repository follows the design-first Prikk roadmap. Change history is tracked in `CHANGELOG.md`;
milestone/status detail is in `rfcs/IMPLEMENTATION-STATUS.md`.

## Current Increment

- **DC-13 - non-default ref genesis (v0.6.0 candidate).** First-commit genesis on explicit non-default
  branch refs is implemented with branch-ref validation, active-WAL ref ownership, and `seal --ref`
  publication rules. The design record is `rfcs/proposed/DC-13-NONDEFAULT-REF-GENESIS.md`.

## Last Released Increment

- **DC-12 - arbitrary-span text edits (released as 0.5.0).** Worktree text edits are authored and
  replayed as deterministic arbitrary spans while keeping inverse/rollback, commutation, confluence,
  and conflict witnesses deferred.

- **DC-11 - publication signing and minimal trust store (released as 0.4.0).** Publication objects carry
  real role-bound Ed25519 MAINTAINER signatures verified against a minimal repository-local trust
  policy.

- **DC-10 - rollback-draft identity and AUTHOR signing (released as 0.3.0).** Rollback-draft identity is
  `PatchPurpose::RollbackDraft`, not a reserved AUTHOR key id, and rollback drafts carry real role-bound
  Ed25519 AUTHOR signatures. This closed the broad claim for AUTHOR-role Patch signatures produced by
  production commands.

## Next Increments

1. **Direct inverse/rollback for arbitrary-span text edits**, gated by round-trip vectors.
2. **M2+ patch algebra after arbitrary spans**: commutation, confluence, conflict witnesses, and merge
   evidence.
3. Branch copy/fork, branch switching, tags/remotes, rollback refs, conflict/inverse evidence,
   rollback authorization, audit/plugin, key lifecycle, and sync remain gated by
   their dedicated plans and FDDs.

Final feature scope remains governed by the FDDs and RFCs.

## Historical Note — PR-030

PR-030 closed the observability gap after rollback drafts are sealed by the existing `seal --allow-no-audit`
path: sealed history labels Blocks that contain rollback-marked Patch objects, and repository verification
counts sealed rollback Blocks and sealed rollback Patch references. It did not introduce rollback-specific
refs, authorize rollback, mutate the worktree, or change seal publication semantics.
