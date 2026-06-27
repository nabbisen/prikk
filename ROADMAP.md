# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-029: active rollback draft verification before seal.

## Next Increments

1. PR-030: design rollback/ref-policy boundaries before rollback-specific publication commands.
2. PR-031: begin conflict/inverse evidence scaffolding or expand arbitrary-span text-edit support only after the patch-engine plan is reviewed.
3. PR-032+: audit/plugin and sync work remain gated by their dedicated plans.

Final feature scope remains governed by the FDDs and RFCs.

## PR-029 Note

PR-029 adds active rollback draft verification. It checks that the active WAL contains exactly one rollback draft Patch, validates the dedicated rollback signature marker, decodes the Patch under the supported replay subset, and compares the payload with the inverse Patch currently derived from the selected ref. It does not publish refs, write object files directly, modify the worktree, authorize rollback, or define rollback-specific ref policy. Rollback refs, authorization policy, conflict witnesses, commutation, confluence, arbitrary-span rollback, audit plugins, and sync remain deferred.
