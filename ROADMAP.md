# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-007: initial RefState publication primitives, flat ref pointer layout, and inline RefUpdate log verification.

## Next Increments

1. PR-008: seal transaction skeleton without plugin execution.
2. PR-009: materialize signed patch envelopes from WAL into the object store during seal.
3. PR-010: basic branch/ref publication flow around the seal skeleton.
4. PR-011+: patch apply/inverse foundations after the patch-engine implementation plan is reviewed.

Final feature scope remains governed by the FDDs and RFCs.
