# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-028: conservative rollback draft append to an empty active WAL.

## Next Increments

1. PR-029: design rollback/ref-policy boundaries before rollback-specific publication commands.
2. PR-030: begin conflict/inverse evidence scaffolding or expand arbitrary-span text-edit support only after the patch-engine plan is reviewed.
3. PR-031+: audit/plugin and sync work remain gated by their dedicated plans.

Final feature scope remains governed by the FDDs and RFCs.

## PR-028 Note

PR-028 adds a conservative rollback draft append. It derives the supported inverse Patch payload, verifies the rollback preview target, requires an explicit `--append-inverse` flag, and appends one signed inverse Patch envelope to an empty active WAL. It does not publish refs, write object files directly, modify the worktree, authorize rollback, or define rollback-specific ref policy. Rollback refs, authorization policy, conflict witnesses, commutation, confluence, arbitrary-span rollback, audit plugins, and sync remain deferred.
