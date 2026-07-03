# Prikk Roadmap

This repository follows the design-first Prikk roadmap. Change history is tracked in `CHANGELOG.md`;
milestone/status detail is in `rfcs/IMPLEMENTATION-STATUS.md`.

## Current Increment

- **DC-11 — publication signing and minimal trust store (v0.4.0).** Publication objects now carry real
  role-bound Ed25519 MAINTAINER signatures verified against a minimal repository-local trust policy.
  The design record is `rfcs/proposed/DC-11-MAINTAINER-TRUST-STORE.md`.

## Last Released Increment

- **DC-10 — rollback-draft identity and AUTHOR signing (released as 0.3.0).** Rollback-draft identity is
  `PatchPurpose::RollbackDraft`, not a reserved AUTHOR key id, and rollback drafts carry real role-bound
  Ed25519 AUTHOR signatures. This closed the broad claim for AUTHOR-role Patch signatures produced by
  production commands.

## Next Increments

1. **Genesis onto non-default refs** (with branch-creation / ref-lifecycle design).
2. **Arbitrary-span text edits and M2+ patch algebra** after the publication-trust boundary is settled.
3. Conflict/inverse evidence, rollback authorization, audit/plugin, key lifecycle, and sync remain gated by
   their dedicated plans and FDDs.

Final feature scope remains governed by the FDDs and RFCs.

## Historical Note — PR-030

PR-030 closed the observability gap after rollback drafts are sealed by the existing `seal --allow-no-audit`
path: sealed history labels Blocks that contain rollback-marked Patch objects, and repository verification
counts sealed rollback Blocks and sealed rollback Patch references. It did not introduce rollback-specific
refs, authorize rollback, mutate the worktree, or change seal publication semantics.
