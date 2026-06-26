# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-010: verification hardening for the local no-audit seal scaffold. Verification now checks block references, RefUpdate-to-RefState links, target block existence, and persisted WAL patch counts.

## Next Increments

1. PR-011: add a read-only `doctor` dry-run report for common repository problems.
2. PR-012: add safe worktree snapshot/root scaffolding before real diff capture.
3. PR-013: patch apply/inverse foundations after the patch-engine implementation plan is reviewed.
4. PR-014+: audit/plugin and sync work remain gated by their dedicated plans.

Final feature scope remains governed by the FDDs and RFCs.
