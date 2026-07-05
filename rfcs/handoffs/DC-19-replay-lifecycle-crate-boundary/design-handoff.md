# DC-19 Design Handoff - Replay/Lifecycle Crate Boundary

Status: Companion for implemented DC-19; v1 architect errata folded into primary RFC
Related RFC: `../../done/DC-19-REPLAY-LIFECYCLE-CRATE-BOUNDARY.md`

## Purpose

This handoff records the design-review focus for DC-19. It is not independent implementation
authority; RFC-000 lifecycle state follows the primary DC-19 RFC.

## Review Focus

Implementation review should verify:

- whether `prikk-replay` is clearly defined as a semantic replay domain crate, not a repository
  lineage/ref/storage crate;
- whether the proposed dependency graph prevents `prikk-replay -> prikk-store`;
- whether the first implementation slice is limited to crate skeleton, `node_lifecycle`, direct tests,
  and direct lifecycle equality/consistency helpers;
- whether the object evidence reader boundary is too broad or too narrow;
- whether replay-derived cache encoding remains store-owned initially;
- whether workspace-internal API policy is explicit enough for new `pub` APIs;
- whether structured replay/evidence errors are preserved across `prikk-replay` -> `prikk-store`
  conversion;
- whether patch algebra should remain entirely in `prikk-store` until a production caller exists;
- whether any part of the plan risks behavior changes during extraction.

## Constraints

- DC-19 is accepted; implementation must stay inside the accepted first slice.
- Behavior-neutral extraction only.
- No CLI merge behavior, merge execution, public confluence API, persisted proof object, schema change,
  rollback refs, or rollback authorization.
- Do not extract worktree first.
- Do not create a standalone patch-algebra crate in the first phase.
- Do not move text-span in the first implementation slice unless review explicitly finds the
  lifecycle-only slice too small to be useful.
- Do not move repository lineage traversal, cache persistence, refs, WAL, active session, verify,
  doctor, or store-backed resolver construction into `prikk-replay`.

## Expected Reviewer Output

- Confirmation that the implementation remains inside the accepted first slice.
- Required changes to evidence-reader/blob-validation boundaries.
- Risks that should block extraction before further M2+ merge/conflict work.
