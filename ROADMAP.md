# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-008: narrow empty-commit scaffold that appends a signed patch envelope to the active
  WAL under `active.lock`.

## Next Increments

1. PR-009: materialize signed patch envelopes from WAL into the object store as a seal prerequisite.
2. PR-010: seal transaction skeleton without plugin execution.
3. PR-011: basic branch/ref publication flow around the seal skeleton.
4. PR-012+: patch apply/inverse foundations after the patch-engine implementation plan is reviewed.

Final feature scope remains governed by the FDDs and RFCs.
