# DC-95 Stage 1, Round 12 — Review v1, and Stage 1's Close

**Reviewing:** `ba52fa1`, `5b3bafc`, `251f5ff` on
`dc-95-verify-coverage-and-finding-accumulation`.

**Accepted, no conditions. Stage 1 is complete.** All 41 rows classified: 34 resolved, 4 excluded,
3 unreachable, **0 remaining**. Verified independently, including the deliverable.

## 1. Reproduced the hardest row

Suppressed the horizon comparison — `lifecycle_cache/replay.rs:264`,
`if false && expected_horizon.is_some_and(…)`:

```
verify_reports_a_lifecycle_cache_with_a_horizon_not_in_its_own_lineage_as_divergence ... FAILED
  dc64_baseline_cache.rs:33: lifecycle-cache divergences: 0
```

**A fully clean report despite a tampered horizon.** Load-bearing, exactly as reported — and the
assertion message is itself the report inspection rather than a bare `Ok`/`Err`, which is the shape
round 8 asked for and this row could actually use.

Gates at `251f5ff`: fmt clean, clippy **0**, **641** prikk-store tests, **5** `dc64_baseline_cache`
tests. Inventory: 34 + 4 + 3 + 0 = 41. Worktree removed, primary tree clean.

## 2. The abandoned first attempt is the better part of the work

Deleting the cache's claimed baseline block does not isolate this check: `verify_objects`
unconditionally re-derives every V2 block's `state_merkle_root`, and that walk shares
`apply_candidate_patches` — **defined in `replay.rs`, imported by `block_state.rs`, not duplicated** —
so any deletion fails state-root re-derivation first, with a hard `Err`, before
`verify_lifecycle_cache_divergence` is reached.

**They did not stop at "unreachable."** They dispatched a focused comparison of the two replay paths
before concluding, and the answer was that *there is no second implementation to diverge from* — the
asymmetry is structural, not semantic: `replay.rs`'s walk is horizon-anchored, `block_state.rs`'s has no
horizon concept and walks to genesis. That is what made the cache-file-side fixture the right
construction and the object-store-side one a dead end.

**This is the third instance of the upstream-gate pattern** — rounds 10, 11, and now this — and the
first where it was resolved by proving the two paths *cannot* differ rather than by finding where they
do. Reaching for a targeted investigation instead of declaring unreachability is the difference between
a correct answer and a lucky one.

## 3. The deliverable landed, and in the right form

`verify.rs`'s module doc now carries the coverage summary: one table per cluster, classification only,
cross-referencing the test files that hold the per-row reasoning rather than duplicating it. 55 table
lines. `cargo doc` clean of new warnings.

**Cross-referencing rather than duplicating is the correct call**, and it is what makes this survive:
a summary that restated every probe would drift from the tests within two increments. The
review-request copy stays as the round-by-round record of *how* each classification was reached; the
in-code copy is the authoritative statement of *what* is currently true.

This discharges the round 7 ruling, the classified-inventory ruling §5, and round 11's instruction that
Stage 1 must not close as a review-request document. **Stage 1's durable output is now the
classification, in the code, where a future reader will find it** — which is the position this review
has held since round 2.

## 4. Stage 1, assessed

Twelve rounds. The arc worth recording:

- **Rounds 1–5** established that reasoning about classification is unreliable and probing settles it —
  at the cost of a classification pass that had to re-verify everything already claimed.
- **Rounds 6–8** established that probes themselves can be confounded, and that a probe's *provenance*
  matters as much as its verdict.
- **Rounds 9–12** established the upstream-gate rule: **a check's own code being present does not
  establish that a defect reaches it.** Three independent instances, now generalised in
  `verify.rs`'s own doc.

**The last of those is the finding with value beyond DC-95.** It is a fact about this codebase that
will mislead anyone writing verification tests, and it is now written where they will be looking.

## 5. Standing

- **Round 12: accepted. DC-95 Stage 1: complete.**
- **Green three-platform CI required before merge** — this is filesystem-backed state and the standing
  rule applies to the whole branch, not to the last commit.
- **Stage 2 is next and is not cleared by this.** It was scoped as two pieces in the original inventory
  ruling and remains blocked on its own scoping; do not start it from Stage 1's momentum.
- **The `accepted/`→`done/` migration is now unblocked**, per its scheduling. It needs its own
  increment covering three batches: my 18 RFC moves, the `crates/` doc-reference slice the dev team must
  make, and a ruling on the 20 `accepted/` RFCs with no `EXECUTION-ORDER` row.
