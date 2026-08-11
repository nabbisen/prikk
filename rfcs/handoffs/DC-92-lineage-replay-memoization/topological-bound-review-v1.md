# DC-92 — §4.2 Topological Bound: Implementation Review v1

**Reviewing:** `4bb851d` on `dc-92-lineage-replay-memoization`.

**The implementation is correct and the results are excellent. ACCEPT, conditional on one test (§4).**
That test covers a gap which **predates DC-92** — I proved that — but which this increment is the
right moment to close, and the reason is proximity, not blame.

## 1. Correctness, checked where it could hide

**Coverage is preserved and completeness is enforced.** Phase A pushes every `CurrentV2` block's
already-decoded payload unconditionally; Phase B's `verify_blocks_topological` ends with
`processed.len() != blocks.len()` → error, so a block that never becomes ready is reported rather than
silently skipped. That check is the one that makes the restructuring safe, and it is present.

**The eviction ordering is right, and it is the subtle part.** A parent's `remaining_children` is
decremented *after* `verify_block_v2_state` has already run for the child, so the parent's memo entry is
still live for the walk that needs it and is evicted only once its last dependent has consumed it. Off
by one step in either direction and this would either leak or fail.

**Single-parent guarantees no double-enqueue.** Because `state_derivation_parent` yields exactly one
edge — including for `Merge`, via `mainline_parent_id` — each child appears under exactly one parent in
the children map and is enqueued once. On a general DAG this loop would enqueue a child per parent; on
this tree it cannot. Their reliance on that property is sound and I verified the property itself.

**Blocks whose state-parent is not in the batch** are marked ready and fall through to the ordinary
store-backed lineage walk inside `verify_block_v2_state` — correct behaviour, simply unaccelerated.

**Phase A retains everything it had** — existence checks, rollback-patch counting, merge-baseline
re-derivation. Only the state check moved.

## 2. Controls, verified rather than accepted

- **Frontier bound is load-bearing.** I disabled the parent eviction and
  `multi_branch_history_bounds_peak_memo_size_by_branch_count_not_block_count` **failed**, alone. The
  test measures the bound, not merely correctness — which was the point of asking for it.
- **All six controls pass** — the four original corruption/shape tests plus the two new ones — and the
  cycle test reaches a genuinely different code path (Kahn's readiness, never touching the store).
  Their note that a real content-hash cycle is cryptographically unconstructible, so fabricated ids are
  the only way to exercise it, is correct.
- **Gates re-run by me at `4bb851d`:** fmt, clippy, both toolchains, **613** prikk-store tests,
  `git diff --check`, `cargo audit`, all three release-policy checks — clean.

## 3. The measurements

**Timing survived the restructuring** — every ratio at or below 1.00 against the pre-phasing numbers.
That was my addition to their test set and it earned its place: it is the check that would have caught a
memory fix costing back the time win.

**Memory is bounded, and the shape is right.** 66,452 KB → 1,996 KB at N=160; the reduction *factor*
grows with N (1.9x → 33.3x), which is the signature of removing an N term rather than a constant. The
after-column's own doubling ratios (1.10x, 1.06x) are nearly flat. **599 MB → 15.1 MB at the worst
measured corner.**

**The content term is retired, not merely unmeasured.** The edit-heavy axis is flat and tracks the churn
axis, confirming the synthesis from the previous ruling: bounding live entries bounds `TextCache` too,
whatever the file size. §4.3 is no longer needed for that purpose.

Peak memory still scales with tree size, as they say — one frontier entry is inherently O(tree_size).
That is the honest residual and it is correctly stated rather than glossed.

## 4. Condition: one end-to-end control that `verify` actually state-checks blocks

**I disabled Phase A's collection entirely — making `verify` perform no block state verification at
all — and the whole workspace suite passed.** All six controls call `verify_block_v2_state` or
`verify_blocks_topological` directly; none goes through `verify_objects`. Nothing proves the wiring
between them exists.

**This is not a regression.** I checked before raising it: removing the inline
`verify_block_v2_state` call on pre-DC-92 `main` also passes the entire suite. The hole has always been
there.

**Why it is a condition anyway.** DC-92 converts a single inline call into a collect-then-batch handoff
across three files — strictly more places for the wiring to break — and this increment is the one
restructuring that path. The same standard I applied to DC-87's `platform-support.md`: proximity
justifies closing a small pre-existing gap in the code you are already rewriting; it does not justify a
sweep. **One test**, at the `verify_repository` level: a repository containing a block whose recorded
`state_merkle_root` does not match its replay, asserting verification fails. Constructed the way the
existing controls are — built, not byte-corrupted, since content addressing makes post-hoc corruption
produce a different valid object.

If that turns out to be materially harder at repository level than it looks, report that rather than
forcing it — but the four existing corruption tests suggest the construction is already solved and only
the level changes.

Registered in `FINDINGS.md` as the broader gap: `verify`'s own wiring has never been covered end to end.

## 5. Standing

- **DC-92: accept on §4.** Then a green **three-platform** CI run, then merge.
- On merge this closes an O(N³) → O(N) verification cost, an O(N²)-per-call seal cost, and a memory
  bound that was introduced and removed within the same increment — the last of those found only
  because the first fix was measured on an axis nobody asked for at the outset.
