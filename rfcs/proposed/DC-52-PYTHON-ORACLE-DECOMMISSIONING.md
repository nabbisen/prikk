# RFC (proposed) - DC-52 Python and Oracle Decommissioning

**Status.** Proposed; design review required. Executes retirement obligations that DC-45 created and that
currently exist only as prose across `MILESTONES.md`, `rfcs/IMPLEMENTATION-STATUS.md`, and architect
review records.
**Target milestone.** M2 - post-correction assurance milestone.
**Schedule position.** After the Rust policy command is authoritative (done — cutover accepted
2026-07-21) and after its later-commit stability evidence is accepted. Must complete before the retained
Python files can be removed.
**Tracks.** DC-45's retirement schedule and its two carried preconditions.
**Touches.** `release/` Python authoring/verification files, the frozen oracle material, and
`tools/release-policy`'s responsibility map. No product code.

## Problem

DC-45 made the Rust release-policy command authoritative but deliberately retained the Python
implementation and the frozen oracle as a rollback path, subject to a staged retirement schedule. That
schedule imposed obligations which remain open and are tracked only in review prose:

1. **Responsibility-map executable correspondence.** The 50-entry responsibility map is not mechanically
   bound to an executed Rust check registry. DC-45 requires this closed before Python oracle retirement.
2. **`defaults.run` nested-key validation.** The governed-procedure YAML extractor skips an empty `run`
   value whose parent key is `defaults`, relying on the GitHub Actions schema forbidding an executable
   scalar there. That is a correct but *assumed* invariant; architect review v11 required it be made
   explicit before Python retirement.
3. **Five-file decommissioning review.** Five Python authoring/verification files are retained through the
   first Rust-gated release. Each must be individually removed or carry an owner-approved, event-bound
   exception.
4. **Eight frozen contract/evidence files.** These remain until an equivalence-backed
   replacement/consolidation review or an explicit final-retirement review closes migration and rollback
   needs.

Leaving these in prose risks the retained Python being either removed prematurely (losing the rollback
path) or retained indefinitely (defeating the consolidation DC-45 paid eleven review rounds for).

## Design

Execute the obligations as ordered, separately reviewable steps:

1. **Bind the responsibility map** to an executed check registry so that a map entry without a
   corresponding executed Rust check, or vice versa, fails closed.
2. **Make the `defaults.run` invariant explicit** — validate that the block nested under `defaults.run`
   contains only known configuration keys (`shell`, `working-directory`) and error otherwise. This
   converts a schema assumption into an enforced rule and protects the exception against future extractor
   changes.
3. **Decommission the five Python files** individually, each with either removal plus equivalence evidence
   or a recorded owner-approved, event-bound exception. Exhaustive: no file may be silently retained.
4. **Rule on the eight frozen contract/evidence files** — replacement, consolidation, or final retirement,
   with the rollback consequence of each stated.

Steps 1 and 2 are preconditions for step 3. Step 4 may follow independently.

## Non-goals

- No change to the selected authoritative policy command (Rust remains authoritative).
- No change to the public policy schema, the product publication graph, or the differential oracle
  semantics.
- No removal of rollback capability before its replacement evidence is accepted.
- No signer, release-lane, or publication action.

## Acceptance criteria

The responsibility map is mechanically bound and fails closed on divergence; `defaults.run` accepts only
known configuration keys; each of the five Python files is removed with evidence or carries an explicit,
event-bound, owner-approved exception; the eight frozen files have a recorded disposition; and the
release-policy gates (`check`, `boundary-check`, `reference-check`) plus the full oracle case set continue
to pass at every step. Each step is separately reviewable; no step may be bundled with another.
