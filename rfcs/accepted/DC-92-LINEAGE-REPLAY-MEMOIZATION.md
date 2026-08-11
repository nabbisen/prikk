# RFC (accepted) - DC-92 Lineage Replay Memoization

**Status.** **ACCEPTED by the project owner 2026-08-11.** §4's five prerequisites still precede design;
acceptance clears the investigation, not the implementation.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** The owner's direction of 2026-08-11 selecting the O(N³) `verify` cost as the next
theme. The finding itself is **owner-authorized, recorded 2026-08-08** in `FINDINGS.md` and measured
during DC-75's prerequisite investigation.
**Target.** 0.20.0. Product **M1**, alongside NFR-PERF-01.

## 1. The known cost

`prikk verify` is roughly **O(N³)** in sealed format-2 blocks — 82 ms at N=5 rising to **34.2 seconds at
N=160**, with the per-doubling ratio climbing 2.08x → 2.26x → 3.04x → 4.53x → 6.39x toward the 8x cubic
signature.

The mechanism, re-derived from source for this RFC:

- `verify.rs:410` calls `verify_block_v2_state` for **every** persisted Block — N of them.
- Each reaches `derive_next_state_root`, which for a non-genesis block runs `validate_v2_lineage(parent)`
  then `verify_v2_lineage_roots` (`block_state.rs:87-88`).
- `verify_v2_lineage_roots` (`:147-162`) loops the whole lineage and calls
  `replay_with_appended_patches` for **each** entry.
- `replay_with_appended_patches` (`lifecycle_cache/replay.rs:448-459`) calls
  `walk_lineage_to_genesis(parent)` and replays the chain from scratch. **There is no cache anywhere on
  this path** — DC-64's incremental baseline cache serves the commit path, not this one.

So verifying the block at position *i* costs O(i²), and O(N³) summed. **This is not a performance ticket
beside the product claim — it is a dependency of it.** The block chain that bounds patch-algebra cost
(prikk's structural answer to Darcs's exponential merge) is the same structure `verify` re-derives from
genesis per block. Prikk traded a merge-path cost for a verification-path cost, and verification is the
central claim in a way merge throughput is not.

## 2. The part the finding does not record, and it may matter more

**`derive_next_state_root` has three production callers, and two are not `verify`:**

- `verify.rs:410` (via `verify_block_v2_state`) — the known case.
- **`prikk-cli/src/seal.rs:156`** — every seal.
- **`merge_execute.rs:165`** — every merge.

On the seal path, `parent` is the current tip, so `validate_v2_lineage` + `verify_v2_lineage_roots` run
over the **entire ancestor chain, on every seal**. By the same arithmetic as §1, that is **O(N²) per
seal** in sealed-history length. Nothing in the path caches, and the call is unconditional.

**This is a hypothesis from control-flow reading, not a measurement**, and §4.1 exists to settle it
before anything is designed. But if it holds, two things follow:

- **NFR-PERF-01's evidence has a blind spot exactly where the cost would be.** DC-59's harness times
  `commit` and states repeatedly that its seals are **untimed** setup (`dc59_commit_benchmark.rs:27`,
  `:30`, `:118`, `:315`). Seal cost against history length has therefore never been benchmarked.
- The commit-plus-seal *cycle* would not be bounded by repository size regardless of what DC-64
  achieved on the commit half.

## 3. Direction — to evaluate, not to inherit

The finding's own note: memoize `walk_lineage_to_genesis`'s result and reuse the accumulated state
across `verify_v2_lineage_roots`'s per-block loop, which the code's shape suggests drops this to O(N).
**That is a starting proposition, not a ruling.** The architect's design assertions have needed
correction repeatedly this cycle.

**The distinction that decides the trust argument, and §4.3 turns on it:** a cache that is **in-memory
and lives only for one `verify` invocation** is a different object from a **persisted** one. DC-64's
trust-ladder ruling constrained caching on the commit path, and NFR-PERF-04 says caches are never roots
of trust. A per-invocation memo table does not persist, cannot be tampered with between runs, and cannot
be stale — so it may avoid the trust question entirely rather than needing to answer it. **Establish
that before reaching for anything persisted.**

## 4. Blocking prerequisites

1. **Measure seal.** Extend DC-59's harness, or write a sibling, that times `seal` against sealed-history
   length. Confirm or refute §2's O(N²) hypothesis with numbers at the same shape of N values the O(N³)
   measurement used. **If it is refuted, say so — that is a useful result and it narrows this increment
   to `verify` alone.**
2. **Where does memoization actually go?** Report what the code admits: memoizing
   `walk_lineage_to_genesis`, or carrying accumulated state forward across
   `verify_v2_lineage_roots`'s loop, or both. State the resulting complexity, and whether `verify`'s
   outer per-block loop needs its own reuse or falls out of the inner fix.
3. **Does a per-invocation in-memory memo avoid the trust question?** Answer §3's distinction explicitly.
   If anything persisted is proposed, it needs a trust argument against NFR-PERF-04 **before** design,
   and the architect expects the answer to be that nothing persisted is required.
4. **Do seal and merge need the same treatment or different?** They call the same function with a
   different shape of input (one derivation, not N). Report whether one fix serves all three call sites.
5. **What is `verify` still required to re-derive?** `verify`'s guarantee is that it re-derives rather
   than trusts. Memoization within one run must change **how many times** work is done, never **what is
   checked**. State precisely what invariant preserves that, before relying on it.

## 5. Acceptance criteria

1. §4 answered and reported before design.
2. **Measured before and after, at the finding's own N values (5 → 160), reported as a table** with the
   per-doubling ratio. The claim is a complexity change; the evidence has to be a curve, not a single
   number.
3. **Negative controls proving `verify` still catches what it caught.** Corrupt a block's recorded state
   root at **genesis, mid-chain, and tip** positions, and show `verify` fails in each case with the
   memoization in place. This is the criterion that matters: the failure mode of caching a verifier is
   that it stops verifying, and a faster green run looks identical to a correct one.
4. **No persisted cache without an explicit trust argument** accepted in advance (§4.3).
5. **No change to what `verify` reports** — same findings, same codes, on every existing test.
6. If §4.1 confirms the seal cost, **seal is measured before and after too**, and NFR-PERF-01's record is
   updated to reflect what was actually found. Whether NFR-PERF-01 can then be claimed is the owner's,
   not this increment's.
7. Gate set per `EXECUTION-ORDER.md` §6 rule 9.

## 6. Non-goals

- **Any change to what `verify` checks.** Faster, not laxer.
- **NFR-PERF-01's closure.** This may move it; claiming it is the owner's decision on evidence.
- **The `lifecycle_cache` persisted-cache design** (DC-64's). Out of scope unless §4.3 concludes
  something persisted is unavoidable, which would be a stop-and-report.
- **NFR-PERF-03** (merge scope bounding), still unowned and unbenchmarked.
- Windows, DC-91, and anything in the DC-87 arc.
