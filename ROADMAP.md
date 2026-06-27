# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-020: read-only supported patch replay planning for file-level operations.
- 0.1.0 PR-021: opt-in supported patch replay materialization without destructive removals.

## Next Increments

1. PR-022: patch replay cleanup and removal-safety design, or begin content-anchored text edit replay scaffolding.
2. PR-023: patch apply/inverse foundations after the patch-engine implementation plan is reviewed.
3. PR-024+: audit/plugin and sync work remain gated by their dedicated plans.

Final feature scope remains governed by the FDDs and RFCs.


## PR-021 note

PR-021 adds opt-in worktree materialization from the supported patch replay result. It still refuses conflicting existing files and never deletes extra files. Full patch algebra remains a later milestone.
