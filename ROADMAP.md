# PRIKK Roadmap

This repository follows the design-first PRIKK roadmap.

## Current Increment

- 0.1.0 PR-026: read-only inverse planning for the supported patch-operation subset.

## Next Increments

1. PR-027: begin conflict/inverse evidence scaffolding or expand arbitrary-span text-edit support only after the patch-engine plan is reviewed.
2. PR-028: design rollback/ref-policy boundaries before any mutating inverse command.
3. PR-029+: audit/plugin and sync work remain gated by their dedicated plans.

Final feature scope remains governed by the FDDs and RFCs.

## PR-026 Note

PR-026 adds read-only inverse planning for the supported patch-operation subset. It validates the sealed single-parent patch chain, derives an unsigned inverse Patch payload for `CreateFile`, `DeleteFile`, `ReplaceBinary`, and full-file `EditText`, and reports a deterministic unsigned Patch ID hint. Rollback refs, authorization policy, conflict witnesses, commutation, confluence, arbitrary-span inverse handling, audit plugins, and sync remain deferred.
