# DC-42 Performance and Maintainability Gates - Implementation Handoff

**Prepared in advance.** Implementation may **not** begin until `rfcs/proposed/DC-42-…` moves to
`rfcs/accepted/` through design review.
**Authored by** the architect (function-designer role). Implementation review remains independent.
**Size:** large — the biggest remaining M2 increment. **Stage it** (see §5); do not submit as one
candidate.
**Touches:** benchmark harness, source/test structure, and potentially `worktree_patch` traversal
semantics. The third of those is a production behaviour change requiring its own design amendment.

## Why this is not a cleanup task

DC-42 contains two genuine **requirements-compliance** decisions, not just tidying. Each must end in one
explicit state — implemented and evidenced, **or** amended by reviewed requirements authority. Silent
deferral is not an outcome.

## 1. NFR-PERF-01 — commit must not scan the full worktree

The requirement (Requirements v1.2 §6.2, NFR-PERF-01) says commit cost is bounded by diff construction,
signing, WAL append, and fsync — with **no full-tree scan**. Today, worktree authoring recursively walks
the tree (`crates/prikk-store/src/worktree_patch/node_authoring.rs`, ~601 lines).

Order of work matters:

1. **Measure first.** Build a repeatable benchmark recording repository size, changed-path count, elapsed
   time, and filesystem assumptions. Publish the numbers before proposing any change.
2. **Then decide.** Either implement a changed-path/index design that removes the scan, or obtain an
   explicit reviewed requirements amendment that defers or replaces the no-scan rule.
3. The default outcome is **compliance**. Deferral requires architect review and stays visible in
   `MILESTONES.md`.

**Trap:** optimisation that changes traversal semantics, caching, or repository authority is a production
behaviour change and needs a focused design amendment before coding — not a performance PR.

## 2. NFR-PERF-02 — active-Patch thresholds

Warn at 800 active Patches, hard-block at 1000 by default, unless an accepted configuration design
overrides the defaults. Implement or amend explicitly.

Tests must cover the **799 / 800 / 999 / 1000 / 1001** boundaries across every authoring and seal path
that defines the active-Patch count.

**Trap:** DC-41 stage 4 introduces *test-generation* bounds (op count, path depth, segment length,
content size) that are deliberately unrelated to these thresholds. Do not let the two converge in code or
prose — the RFC calls this out specifically.

## 3. Source-structure audit — current measured state

Implementation files over 300 physical lines, measured at this baseline:

| Lines | File | Note |
|---|---|---|
| 974 | `prikk-store/src/lifecycle_cache.rs` | over the 500 strong-split threshold |
| 733 | `prikk-store/src/patch_replay/decode.rs` | over 500 |
| 638 | `prikk-object/src/payload/patch.rs` | over 500 |
| 624 | `prikk-object/src/vectors/hard.rs` | vector data, not logic — judge separately |
| 601 | `prikk-store/src/worktree_patch/node_authoring.rs` | over 500; also the §1 target |
| 552 | `prikk-store/src/text_span.rs` | over 500 |
| 537 | `prikk-store/src/patch_replay.rs` | over 500 |
| 497 | `prikk-cli/src/main.rs` | under 500 |
| 459 | `prikk-store/src/text_span/vectors.rs` | vector data |
| 413 | `prikk-store/src/test_support.rs` | test support |
| 411 | `prikk-cli/src/args.rs` | under 500 |
| 404 | `prikk-store/src/lifecycle_cache/replay.rs` | under 500 |

Distinguish **logic** files from **vector/test-support** files: a 624-line table of test vectors is not
the same maintainability problem as a 974-line cache implementation, and splitting the former by line
count alone would be cargo-culting the rule.

Remaining inline `mod tests {` blocks: **two** — `prikk-object/src/canonical.rs` and
`prikk-object/src/id.rs`. (DC-41 stage 2 already extracted `prikk-hash`'s.) Extract both to sibling
`tests.rs` modules as pure moves, verified behaviour-neutral before any other edit — the same technique
that worked in DC-41 stage 2.

**Trap:** the RFC is explicit — **no weakening of tests to satisfy line-count targets**. Mechanical
extraction must preserve public module paths and behaviour.

## 4. Recommended staging

| Stage | Content | Risk |
|---|---|---|
| 1 | Inline-test extraction (2 files) + ELOC audit report | none — pure moves plus a document |
| 2 | Commit benchmark harness + published baseline numbers | none — measurement only |
| 3 | NFR-PERF-02 thresholds, or the amendment | behaviour change |
| 4 | NFR-PERF-01 outcome — index design, or the amendment | largest; may need its own design amendment |
| 5 | Agreed file splits from the stage-1 audit | mechanical, but touches large files |

Stages 1-2 are safe and unblock the decisions in 3-4 with real data. Do not start stage 4 before stage 2's
numbers exist.

## 5. Definition of done

- Both NFR-PERF-01 and NFR-PERF-02 end in one recorded state each: implemented+evidenced, or
  amended/deferred by reviewed requirements authority, visible in `MILESTONES.md`.
- Benchmark is reproducible and its assumptions stated.
- ELOC audit published; every file over 500 lines either split or carrying a recorded cohesion exception.
- Zero inline `mod tests {` under `src/`.
- No test weakened; no public module path changed by extraction.
- Test counts reported before/after per touched crate.
- Full gate set green (`rfcs/EXECUTION-ORDER.md` §6.8).

## 6. Submit with

Per stage: diff; evidence note (measurements, decisions taken, exceptions recorded with reasons); gate
output; explicit statement of what did not change. Stage 3 and 4 candidates must state plainly whether
they implement or amend the requirement.
