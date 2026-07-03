# Prikk Roadmap

This repository follows the design-first Prikk roadmap. Change history is tracked in `CHANGELOG.md`;
milestone/status detail is in `rfcs/IMPLEMENTATION-STATUS.md`.

## Current Increment

- **DC-10 — rollback-draft identity and AUTHOR signing (v0.3.0).** The current design-first increment
  separates rollback-draft identity from the fake AUTHOR-signature marker and makes rollback drafts use
  real role-bound Ed25519 AUTHOR signing. The design record is
  `rfcs/proposed/DC-10-ROLLBACK-DRAFT-SIGNING.md`.

## Last Released Increment

- **DC-09 Phase 4.4b — genesis / first-commit authoring (released as 0.2.0).** `prikk commit` supports a genesis first commit
  on a fresh repository (empty baseline → all `CreateFile`), and the first `seal` publishes a Root block, so
  `init → commit → seal` works end to end on the default `heads/main`. Built on 4.4a node-addressed
  authoring signed with role-bound Ed25519 AUTHOR signatures (R1); the `rollback-draft` AUTHOR-role marker
  remains an internal, non-publishable development scaffold (R1R2).

## Next Increments

1. **Genesis onto non-default refs** (with branch-creation / ref-lifecycle design).
2. **Publication-grade signing / trust (later phase).** MAINTAINER publication signing, trust store, key
   management/rotation, and signature policy.
3. Conflict/inverse evidence, arbitrary-span text-edit support, audit/plugin, and sync remain gated by
   their dedicated plans and FDDs.

Final feature scope remains governed by the FDDs and RFCs.

## Historical Note — PR-030

PR-030 closed the observability gap after rollback drafts are sealed by the existing `seal --allow-no-audit`
path: sealed history labels Blocks that contain rollback-marked Patch objects, and repository verification
counts sealed rollback Blocks and sealed rollback Patch references. It did not introduce rollback-specific
refs, authorize rollback, mutate the worktree, or change seal publication semantics.
