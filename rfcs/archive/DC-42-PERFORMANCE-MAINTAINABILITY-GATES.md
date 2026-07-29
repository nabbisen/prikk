# RFC (archive - superseded) - DC-42 Performance and Maintainability Gates

**Status.** **Superseded on 2026-07-29 by DC-56, DC-57, and DC-58.** Never implemented; no code was
written against it.

Design review v1 and v2 (`.git-exclude/reviewed/prikk-dc42-design-review-v1.md`, `…-v2.md`) returned
*Design Revision Required*. The blocking reason was scoping, not technical judgement: DC-42 bundled three
unrelated increments — a possible architecture change, a workspace-wide refactor, and a user-visible CLI
feature — against the standing "one increment per candidate" rule. Each successor takes one:

| Successor | Takes |
|---|---|
| **DC-56** | NFR-PERF-01 — commit must not scan the full worktree |
| **DC-57** | NFR-PERF-02 — active-Patch warn/block thresholds |
| **DC-58** | Source-structure and ELOC audit |

**What DC-42 got right, and what the successors inherit:** that measurement alone does not close
NFR-PERF-01; that compliance is the default outcome and deferral carries the burden; that optimisation
changing traversal semantics, caching, or repository authority is a behaviour change needing its own
design amendment; and that tests must not be weakened to satisfy line counts. All four survive in the
successors.

**What review corrected.** DC-42 presented both requirements as ordinary corrective-M2 scope. Both are in
fact **missed product gates** — NFR-PERF-01 at product M1, NFR-PERF-02 at product M3 — a fact obscured by
two milestone schemes sharing the labels M0–M3. See `MILESTONES.md` § "Two milestone schemes".

Retained unedited below as the historical record.

---

**Status (original).** Proposed; design review required.
**Target milestone.** M2 - post-correction assurance milestone.
**Schedule position.** Second remaining post-M1 increment, after DC-41 establishes the released
evidence baseline. This program order is not independent implementation authority.
**Tracks.** Architect review N5 and project Rust development/testing rules.
**Touches.** Commit performance benchmark, source/test module boundaries, ELOC reporting, and CI or
maintainer gate documentation. No semantic feature work.

## Design

Establish three measurable gates:

1. A repeatable commit/worktree-authoring benchmark records repository size, changed-path count,
   elapsed time, and filesystem assumptions. It must reveal whether authoring scans the full tree and
   define the accepted experimental threshold before optimization begins. Measurement alone does not
   close NFR-PERF-01: DC-42 must either implement a changed-path/index design that removes the full-tree
   scan or obtain an explicit requirements amendment that defers/replaces the no-scan rule. The default
   outcome is compliance; deferral requires architect review and remains visible in `MILESTONES.md`.
2. A source-structure audit reports implementation and test-module ELOC. Implementation files over 300
   ELOC require a recorded split decision; files over 500 ELOC are split unless architect review accepts
   a concrete cohesion exception. Inline test modules under `src/` move to sibling test modules in
   accordance with the project testing guidelines.
3. NFR-PERF-02 is implemented or explicitly amended through design review: warn at 800 active Patches
   and hard-block at 1000 by default unless an accepted configuration design overrides the default.
   Tests cover 799/800/999/1000/1001 boundaries and all authoring/seal paths that define the active
   Patch count.

Mechanical extraction must preserve public module paths and behavior. Performance optimization follows
measurement and receives a focused design amendment if it changes traversal semantics, caches, or
repository authority.

## Initial audit scope

At minimum review `lifecycle_cache.rs`, `patch_replay/decode.rs`,
`worktree_patch/node_authoring.rs`, `payload/patch.rs`, `lifecycle_cache/replay.rs`, `verify.rs`, and
all inline `mod tests` occurrences identified by the architect review.

## Non-goals

- No arbitrary crate split, public API redesign, benchmark marketing claim, or unrelated refactor.
- No weakening of tests to satisfy line-count targets.

## Acceptance criteria

Baseline measurements are reproducible, exceptions are explicit, prohibited inline tests are removed,
critical oversized files have reviewed boundaries, and normal behavior/gates remain unchanged except
for separately accepted performance/active-bound behavior. NFR-PERF-01 and NFR-PERF-02 each end in one
explicit state: implemented and evidenced, or amended/deferred by reviewed requirements authority.
