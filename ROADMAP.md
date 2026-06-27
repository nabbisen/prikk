# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-027: non-mutating rollback preview for the supported patch-operation subset.

## Next Increments

1. PR-028: design rollback/ref-policy boundaries before any mutating inverse command.
2. PR-029: begin conflict/inverse evidence scaffolding or expand arbitrary-span text-edit support only after the patch-engine plan is reviewed.
3. PR-030+: audit/plugin and sync work remain gated by their dedicated plans.

Final feature scope remains governed by the FDDs and RFCs.

## PR-027 Note

PR-027 adds a non-mutating rollback preview. It derives the unsigned inverse plan, validates the supported replay target, and compares the current replayed state with the latest snapshot baseline. It reports what files rollback would create, delete, or replace, but it does not write objects, append WAL records, publish refs, modify the worktree, or authorize rollback. Rollback refs, authorization policy, conflict witnesses, commutation, confluence, arbitrary-span rollback, audit plugins, and sync remain deferred.
