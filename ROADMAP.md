# Prikk Roadmap

This repository follows the design-first Prikk roadmap.

## Current Increment

- 0.1.0 PR-030: sealed rollback block/history classification after normal seal.

## Next Increments

1. PR-031: design rollback/ref-policy boundaries before rollback-specific publication commands.
2. PR-032: begin conflict/inverse evidence scaffolding or expand arbitrary-span text-edit support only after the patch-engine plan is reviewed.
3. PR-033+: audit/plugin and sync work remain gated by their dedicated plans.

Final feature scope remains governed by the FDDs and RFCs.

## PR-030 Note

PR-030 closes the observability gap after rollback drafts are sealed by the existing `seal --allow-no-audit` path. Sealed history now labels Blocks that contain rollback-marked Patch objects, and repository verification counts sealed rollback Blocks and sealed rollback Patch references. It does not introduce rollback-specific refs, authorize rollback, mutate the worktree, or change seal publication semantics. Rollback refs, authorization policy, conflict witnesses, commutation, confluence, arbitrary-span rollback, audit plugins, and sync remain deferred.
