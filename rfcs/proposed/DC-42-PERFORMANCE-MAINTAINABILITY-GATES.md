# RFC (proposed) - DC-42 Performance and Maintainability Gates

**Status.** Proposed; design review required.
**Target milestone.** M2 - post-correction assurance milestone.
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
