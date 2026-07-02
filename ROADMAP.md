# Prikk Roadmap

This repository follows the design-first Prikk roadmap. Change history is tracked in `CHANGELOG.md`;
milestone/status detail is in `rfcs/IMPLEMENTATION-STATUS.md`.

## Current Increment

- **DC-09 Phase 4.4b — genesis / first-commit authoring (released as 0.2.0).** `prikk commit` supports a genesis first commit
  on a fresh repository (empty baseline → all `CreateFile`), and the first `seal` publishes a Root block, so
  `init → commit → seal` works end to end on the default `heads/main`. Built on 4.4a node-addressed
  authoring signed with role-bound Ed25519 AUTHOR signatures (R1); the `rollback-draft` AUTHOR-role marker
  remains an internal, non-publishable development scaffold (R1R2).

## Next Increments

1. **Rollback-draft signing design pass.** Separate the rollback-draft identity marker from the AUTHOR
   signature (decide where rollback-draft identity lives — patch-kind vs WAL-record kind), then sign with a
   real AUTHOR key, so a broad "all production AUTHOR Patch signatures are real Ed25519" claim becomes true.
2. **Genesis onto non-default refs** (with branch-creation / ref-lifecycle design).
3. **Publication-grade signing / trust (later phase).** MAINTAINER publication signing, trust store, key
   management/rotation, and signature policy.
4. Conflict/inverse evidence, arbitrary-span text-edit support, audit/plugin, and sync remain gated by
   their dedicated plans and FDDs.

Final feature scope remains governed by the FDDs and RFCs.

## Historical Note — PR-030

PR-030 closed the observability gap after rollback drafts are sealed by the existing `seal --allow-no-audit`
path: sealed history labels Blocks that contain rollback-marked Patch objects, and repository verification
counts sealed rollback Blocks and sealed rollback Patch references. It did not introduce rollback-specific
refs, authorize rollback, mutate the worktree, or change seal publication semantics.
