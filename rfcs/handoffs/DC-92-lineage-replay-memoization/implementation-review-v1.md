# DC-92 — Implementation Review v1

**Reviewing:** `d4ecf66` on `dc-92-lineage-replay-memoization` (on `98b6c12`'s harness).

**Verdict: the fix is correct and the result is real — independently reproduced. ACCEPT, conditional
on §4: the memo's memory is unbounded and unmeasured, and the harness cannot see it.** The condition is
"measure it, and bound it if the measurement says so," not "redo this."

## 1. The performance claim, reproduced independently

I ran the committed harness myself rather than reading their table:

| N | my run (ms) | ratio |
|---:|---:|---:|
| 5 | 91.33 | — |
| 10 | 179.53 | 1.97x |
| 20 | 352.73 | 1.96x |
| 40 | 687.84 | 1.95x |
| 80 | 1368.23 | 1.99x |
| 160 | 2722.73 | 1.99x |

**Flat at 2x across every doubling — the O(N) signature**, against a before-curve climbing 2.19x → 6.33x
toward the cubic 8x. 2.72 s at N=160 where the before-run measured 46.4 s. Their seal numbers reproduce
too (my 1.03x → 1.51x against their 1.06x → 1.45x).

This is a real complexity change, not a constant-factor win, and it is the shape the design predicted
rather than an unexplained improvement.

## 2. Correctness, checked where it could hide

**Every block still gets its own full check.** I looked specifically for a memo short-circuit in
`verify`'s outer loop and there is none: `verify_block_v2_state` runs `validate_block_v2_shape` *before*
touching the memo, and always recomputes and compares that block's own state root. The memo removes
*ancestor re-derivation*, never a block's own verification. That is the correct place to draw the line.

**The walk's stopping rule is sound.** `validate_v2_lineage` follows `state_derivation_parent` and stops
at the first memoized ancestor; `verify_v2_lineage_roots` then resumes from exactly that boundary's
memoized state, or an empty state at true genesis. Since the walk follows the same pointer the resume
uses, the boundary is always the parent of the deepest unresolved entry — the two cannot disagree.

**Shape and schema validation survive the walk shortening**, which was my §4 condition on the
prerequisite ruling. `validate_v2_lineage` still shape-validates and schema-checks every entry it
collects, and blocks below a memo boundary were validated when they were memoized. `LineageStateMemo`'s
doc states the invariant in the corrected, wider form I asked for.

**The negative controls are non-confounded — verified, not accepted.** I disabled
`validate_block_v2_shape` and re-ran: `shape_violation_at_a_lineage_member_position_is_caught` **failed**
(with two pre-existing shape matrices), while all three state-root corruption tests still passed. That
is precisely the discrimination the condition wanted — the shape test tests shape, the root tests test
roots. Their own account of finding and fixing a confounded first draft, via the `naive_continue`
helper, is the right instinct and they reported it rather than quietly fixing it.

**The deletions are safe.** `walk_lineage_to_genesis` and `replay_with_appended_patches` are genuinely
unreferenced — I grepped the workspace.

**The `TextCache` regression they found is the good kind of finding**: caught by an existing test
(DC-65's five-sealed-edits case), root-caused against `incremental.rs`'s own module doc, and fixed by
carrying the cache rather than copying DC-64's fallback — with the reason why the two situations differ
stated explicitly. `apply_one_block` left untouched for its existing caller.

**Gates re-run by me at `d4ecf66`:** fmt, clippy, both toolchains, **611** prikk-store tests,
`git diff --check`, `cargo audit`, all three release-policy checks — all clean.

## 3. What the report does not address

`LineageStateMemo` is `BTreeMap<ObjectId, (NodeLifecycleState, TextCache)>`, inserted into on every
verified block, **never evicted**, living for the whole `verify` run. I checked for any eviction,
capacity bound, or retention rule: there is none.

What each entry holds:

- **`NodeLifecycleState`** — four maps, including `seen_ids`, which DC-69 established never shrinks
  ("prikk does not forget"). Its size grows with *cumulative* history, not the live tree.
- **`TextCache = BTreeMap<NodeId, Vec<u8>>`** — **materialized file contents.**

So a `verify` run now holds, simultaneously, one full lifecycle state **and one copy of the materialized
text content** per sealed block. For a repository with many text files and a long history, that product
is the dominant term, and nothing bounds it.

**The harness structurally cannot detect this.** It measures time only, and it holds the live tree fixed
and small by churn — the right choice for isolating history depth, and exactly the choice that hides a
per-block state clone. DC-62 exists because memory is an axis this project already decided to track;
this change has not been measured on it.

I have **not** measured this, and I am not asserting a specific figure. I am asserting that the
structure is unbounded by construction, that nothing in the evidence addresses it, and that the
instrument used cannot.

## 4. Condition

**Measure the memory cost of the memo, and bound it if the measurement warrants.** Specifically:

1. **Measure with a non-trivial tree.** The churn harness's fixed small tree is the wrong instrument
   here. Report peak memory for `verify` against both history depth *and* a realistic file count — DC-62
   established the axis, and DC-59's tree sizes are the natural scale to borrow.
2. **If it grows unboundedly in N, bound it, and report the shape before implementing.** Two leads,
   offered as leads and not as design: (a) a memo entry is only needed while some unverified block's
   lineage may still reach it, which is decidable in `verify`'s own loop; (b) the `TextCache` half may
   not need per-block retention at all — `replay.rs`'s own comment says a fresh cache is safe because a
   miss falls back, and establishing whether that holds on *this* path would remove the dominant term
   without touching the state half.
3. **If the measurement shows it is fine at realistic scale, say so with numbers and this closes.**

**Why this is a condition and not a follow-up.** Before this change, `verify` on a large repository was
slow. After it, it may not complete. Slow is recoverable and honest; an out-of-memory failure in the one
command whose completion *is* the product's central claim is a different kind of failure. Trading an
unbounded time cost for an unbounded memory cost would not be a fix, and the increment currently has no
evidence either way.

## 5. Not conditions

- **`derive_next_state_root`'s unchanged public signature**, with a thin wrapper constructing a fresh
  memo. Six call sites needed no change; that is the right seam.
- **Seal not sharing a memo across invocations.** Correct — that would be a persisted cache and a trust
  question nobody asked for. The measured 1.03x → 1.51x curve is the design's own prediction, and saying
  so rather than claiming "faster" is the right framing.
- **NFR-PERF-01 not claimed.** Correct; that remains the owner's on evidence, and the seal table is now
  the evidence that never existed.

## 6. Standing

- **Merges after §4 and a green three-platform CI run** — this touches filesystem-backed state.
- The harness surviving as a committed, re-runnable instrument is the right outcome of step zero: the
  next person to ask this does not have to rebuild it, which is precisely what DC-75 failed to leave
  behind.
