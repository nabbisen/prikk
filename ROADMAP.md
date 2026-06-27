# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-022: explicit patch deletion planning and opt-in removal of files deleted by supported patch replay.

## Next Increments

1. PR-023: patch replay cleanup and begin content-anchored text edit replay scaffolding, or add a reviewed patch apply/inverse boundary.
2. PR-024: patch apply/inverse foundations after the patch-engine implementation plan is reviewed.
3. PR-025+: audit/plugin and sync work remain gated by their dedicated plans.

Final feature scope remains governed by the FDDs and RFCs.

## PR-022 note

PR-022 does not add general destructive checkout pruning. It deletes only files explicitly removed by replayed patch operations, and only when the current worktree bytes still match the old Blob precondition. Modified files and unrelated untracked files are preserved.
