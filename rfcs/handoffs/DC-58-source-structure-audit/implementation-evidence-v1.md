# DC-58 Source-Structure Audit - Implementation Evidence v1 (batch 1)

**Date:** 2026-07-29
**Handoff followed:** `implementation-handoff-v1.md`, cleared to start after project-owner acceptance
of `rfcs/accepted/DC-58-SOURCE-STRUCTURE-AUDIT.md`.
**This is batch 1 of a staged increment**, per the RFC's own instruction ("Stage the work: report
first, then splits in reviewable batches. 23 files is too large for one review unit"). See
`source-structure-report-v1.md`'s "Batch status" section for exactly what is and is not done.

## What this batch delivers

1. The full source-structure report (`source-structure-report-v1.md`), correcting the inherited
   7-over-500/16-between-300-500 baseline to the properly-scoped 6/14 figures, with the discrepancy
   explained (the inherited count appears to have included `vectors/hard.rs`, a test-support file,
   before this RFC's own exclusion rule was applied — 6 + 624-line `hard.rs` = 7, matching exactly).
2. One complete over-500 split: `crates/prikk-store/src/patch_replay.rs` (537 lines) into
   `patch_replay.rs` (245), `patch_replay/read.rs` (134, new), `patch_replay/apply.rs` (206, new).
3. Two of three inline `mod tests` relocations: `crates/prikk-object/src/id.rs` and `canonical.rs`,
   each moved to a sibling `tests.rs`. The third (`frozen_outgoing.rs`) is explicitly excluded as
   DC-55 frozen evidence — reasoning in the report.
4. Every file over 300 ELOC has a recorded decision (split, deferred, or leave-as-is with reason) —
   all 20, not a subset.

## What this batch does not deliver (queued, not silently dropped)

Three of the six over-500 files are recommended for splitting but not yet split this batch:
`lifecycle_cache.rs` (974), `patch_replay/decode.rs` (733), `payload/patch.rs` (652),
`text_span.rs` (552) — four, not three; see the report's table. `node_authoring.rs` (601) stays
deferred per the handoff's explicit instruction (DC-56 dependency), not queued.

## Why only one split landed this batch

Time-boxed deliberately. Each of the four remaining over-500 files is either identity-adjacent
(`payload/patch.rs`, `text_span.rs` — both sit on paths DC-41/DC-55 built evidence around) or large
enough (`lifecycle_cache.rs` at 974 lines, nearly double the next-largest) that a rushed split risks
exactly the failure mode the RFC warns against: a diff too large to review for completeness, or a
subtle behavior change hiding in a big mechanical move. One complete, fully-verified split
demonstrates the pattern works end to end (compiles, clippy-clean, identical test count, `fmt`-clean)
without compromising rigor on the remaining four.

## Test counts, before / after

| Crate | Before | After | Delta |
|---|---:|---:|---|
| `prikk-store` | 543 | 543 | 0 |
| `prikk-object` | 76 | 76 | 0 |
| `prikk-replay` | 44 | 44 | 0 |
| `prikk-hash` | 14 | 14 | 0 |
| `prikk-crypto` | 5 | 5 | 0 |
| `prikk-release-policy` | 59 | 59 | 0 |
| `prikk` (prikk-cli) | 27 passed, 1 ignored | 27 passed, 1 ignored | 0 |

**Identical across the board** — this is the batch's own correctness claim, per the RFC's acceptance
criterion 5, and it holds.

## What did not change

- No public module path changed: `patch_replay::{PatchReplayPlan, prepare_patch_replay_plan}` and
  the `pub(crate)` items keep their same paths; only new sibling-private (`pub(super)`) items were
  introduced inside the new `read.rs`/`apply.rs` files, not visible outside `patch_replay`.
  `crates/prikk-object/src/id.rs` and `canonical.rs` keep their same public items; only their test
  modules moved.
- No identity artifact changed: `git status --short` on `vectors/snapshot.txt`, `vectors/hard.rs`,
  `state_root/tests/vectors.rs`, `text_span/vectors.rs` is empty.
- No behavior change: every moved item is verbatim (visibility modifiers aside), confirmed by
  identical test counts and a clean `git diff --check`.
- `node_authoring.rs` untouched.
- Locked package count and `Cargo.lock` unchanged (no dependency touched by this increment).

## Gate output

All green, both toolchains:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked` — all crates pass, counts above
- `cargo +1.85.0 test --workspace --locked` — identical counts
- `git diff --check`
- release-policy `check` — all 154 oracle cases passed
- release-policy `boundary-check --format json` — `valid: true`
- release-policy `reference-check --format json` — `valid: true`

## Acceptance criteria, against the accepted RFC's list (this batch)

1. Source-structure report committed. **Met.**
2. Test-support exclusions enumerated with reasons, including `vectors/hard.rs` and
   `frozen_outgoing.rs`. **Met.**
3. Every file over 300 has a recorded split decision; every file over 500 is split or carries an
   accepted cohesion exception. **Partially met** — all 20 files have a recorded decision (full
   compliance with the first clause); of the 6 over-500 files, 1 is split, 1 is deferred by design,
   4 are recommended-and-queued rather than split-or-exempted (partial compliance with the second
   clause, explicitly not claimed as complete).
4. The 3 inline `mod tests` blocks relocated. **2 of 3**, third excluded with reasoning recorded.
5. Public module paths and observable behaviour unchanged. **Met**, evidenced above.
6. Full gate set and test counts before/after. **Met.**
