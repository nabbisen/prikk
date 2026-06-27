# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-025: opt-in full-file `EditText` generation from UTF-8 worktree modifications.

## Next Increments

1. PR-026: reviewed patch apply/inverse foundations for the supported operation subset.
2. PR-027: begin conflict/inverse evidence scaffolding or expand arbitrary-span text-edit support only after the patch-engine plan is reviewed.
3. PR-028+: audit/plugin and sync work remain gated by their dedicated plans.

Final feature scope remains governed by the FDDs and RFCs.

## PR-025 Note

PR-025 connects the PR-024 full-file `EditText` replay shape to opt-in worktree patch generation. `prikk commit --from-worktree --text-edits -m <message>` emits full-file `EditText` for modified tracked files only when both the snapshot baseline and current worktree content are valid UTF-8. Binary or invalid UTF-8 modifications continue to fall back to `ReplaceBinary`. Arbitrary span discovery, minimized text diffs, inverse generation, commutation, and conflict witnesses remain deferred.
