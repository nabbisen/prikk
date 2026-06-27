# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-024: conservative full-file `EditText` replay for exact-span replacements.

## Next Increments

1. PR-025: reviewed patch apply/inverse foundations for the supported operation subset.
2. PR-026: begin conflict/inverse evidence scaffolding or expand text-edit support only after the patch-engine plan is reviewed.
3. PR-027+: audit/plugin and sync work remain gated by their dedicated plans.

Final feature scope remains governed by the FDDs and RFCs.

## PR-024 Note

PR-024 supports only `EditText` operations whose `anchor_id` is `full-file`. Replay verifies the current full file bytes against `old_span_hash` and replaces the whole file with the UTF-8 replacement text. Arbitrary span discovery, offset-based replay, text-diff generation, inverse generation, commutation, and conflict witnesses remain deferred.
