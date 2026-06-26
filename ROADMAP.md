# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-009: local no-audit seal scaffold that persists active WAL patch envelopes,
  creates a Block, publishes `heads/main`, and clears the active WAL after success.

## Next Increments

1. PR-010: strengthen seal verification and repeated-seal scenarios.
2. PR-011: add safe worktree snapshot/root scaffolding before real diff capture.
3. PR-012: patch apply/inverse foundations after the patch-engine implementation plan is reviewed.
4. PR-013+: audit/plugin and sync work remain gated by their dedicated plans.

Final feature scope remains governed by the FDDs and RFCs.
