# DC-64 Baseline Reconstruction Cost - Handoff

**Cleared to start.** Accepted by the project owner on 2026-07-31, at
`rfcs/accepted/DC-64-BASELINE-RECONSTRUCTION-COST.md`.
**Authored by** the architect. Design review v1 discharged the blocking measurements and killed the RFC's
own leading design option — read §2 before anything else.
**Size:** large, and the riskiest increment currently open. It touches what a commit believes its baseline
is.
**Touches:** `crates/prikk-store/src/lifecycle_cache.rs`, `patch_replay.rs`, and the commit path's baseline
reconstruction in `worktree_patch/node_authoring.rs`.

**This closes — or honestly reports — NFR-PERF-01**, a missed product-M1 gate outstanding since before
0.17.7. DC-56 tried and moved a fifth of the problem.

## 1. The measurements are done. Do not redo them; do not re-derive the target from scratch

Design review discharged all three blocking prerequisites at `5b16b54`. You inherit numbers, not homework:

| Sub-phase, 10,000 files | Cost |
|---|---:|
| `resolve_worktree_baseline` | 0.075 ms |
| **`replay_derived_state`** | **370.6 ms — 97.6% of the phase** |
| `.state().clone()` | 1.87 ms |
| `live_nodes()` projection | 4.77 ms |
| `working_state.clone()` | 2.43 ms |

**The cost is per operation replayed, ~40 µs each.** At a fixed 1,000 files: 50 extra sealed
one-operation patches cost 2.9 ms, but 20 patches carrying 100 operations each cost 121.2 ms against
37.5 ms for the same 20 carrying one. Repository size dominates only because the genesis patch contributes
one `CreateFile` per file.

**Consequences you can rely on:** caching the projection or the clones is worth 2.4% and is not the
increment. The target is `replay_lineage`, and the unit to reduce is **operations replayed**.

## 2. The route is settled, and one route is already eliminated

**Do not build a cache keyed on `(baseline_block, horizon)`.** It cannot hit. The active WAL is capped at
one record, so `commit` refuses to run twice without an intervening `seal`; every seal advances the ref to
a new block; so **every commit presents a key no previous commit has seen** — a 100% miss rate plus a
write on every commit.

That was the RFC's leading proposal in its first draft. It was wrong, design review caught it, and it is
recorded rather than quietly removed so nobody re-proposes it.

**Build incremental replay from a cached predecessor state.** Between consecutive commits the lineage grows
by one patch — a handful of operations at ~40 µs each, against replaying every operation ever made. A keyed
cache is legitimate **only as the storage layer** for that predecessor, never as the mechanism.

## 2a. The trust-ladder question — asked, ruled, and settled 2026-07-31

You escalated before writing code, having found rung-4 `certified_compared_cache` running a full replay as
its certification step. **The report was accurate and the escalation was right.** Ruling in full at
`.git-exclude/reviewed/prikk-dc64-trust-ladder-ruling-v1.md`; the short version:

**The precedent does not bind this increment.** Rung 4 gates `node_id` **reuse** and
**restoration-equivalence**. Both are consumed by `patch_algebra` from the **merge** path
(`merge_evidence.rs`) — never by commit. The commit path builds `baseline_files` from `live_nodes()` only,
so it never resurrects a node id; deleted-then-recreated paths always mint fresh.

**Also, your argument against yourself was stronger than it needed to be.** Full replay is self-correcting
against *state-persistence* faults, not against *computation* faults — it recomputes the identical fold
with the identical functions, so a latent `apply_state_effect` bug corrupts it just as thoroughly. What
incremental application adds is exposure to persistence and serialization faults, which your checksum,
retained `from_replay`, bounded reanchor, and `verify` comparison are proportionate to. **This holds only
because you reuse the existing application functions** — a parallel implementation of the fold would have
been ruled the other way.

**Build your §3 design, under four binding conditions:**

1. **`seen_ids` persisted complete, never truncated.** It is the sole input to the commit path's mint
   collision guard (`node_id_gen.rs:124`) and the one thing that grows with cumulative history. If size
   pressure makes truncation tempting, **that is a finding to report** — it would be a change of safety
   posture disguised as an optimisation.
2. **Scoped to the commit path.** If `patch_algebra` or `merge_evidence` ever consumes this cache, rung 4's
   full-replay certification applies and that is a new design question, not an extension. State this in
   your design document.
3. **`from_replay` stays in the path, unmodified.**
4. **Fallback stays total** — cache absent, corrupt, wrong horizon, parent mismatch, multi-parent, or
   reanchor bound reached ⇒ unmodified full replay, cache reset. **State the reanchor bound with a reason**;
   it is the only control on how long a fault that survives checksum-and-structure can live.

**Route (c) is not triggered.** NFR-PERF-01 is not inherent on this evidence.

## 2b. Criterion 7 is amended — your §5 finding was right

You noted `latest_tombstone_by_id` and `seen_ids` accumulate every node ever created or deleted, so
resident state is bounded by cumulative history rather than repository size. Correct, and it made
criterion 7 unachievable as written.

**Amended:** memory must no longer track **worktree content or the live node set on the hot path**.
History-proportional state is inherent to a replay-based identity model and is not yours to eliminate.
**Measure and report the history-proportional component separately.** If it dominates, that is the next
finding and it belongs to the unowned tombstone/`seen_ids` retention question — not to you.

That criterion was mine and was wrong in the same way DC-56's were. Amended now rather than at your review.

## 3. The part that will actually be hard

**A full replay is self-correcting. Yours will not be.**

Today, every commit reconstructs from the horizon, so a wrong state cannot persist past one commit. An
incrementally-maintained state **compounds errors silently across cycles** — and the thing it decides is
what the repository's history says the state is. If it drifts, the commit is authored against a fictional
baseline, with wrong node identities and wrong parentage, **and then signed.**

This is the closest any cache in this project has come to a root of trust. NFR-PERF-04's gloss at
`specs/prikk-non-functional-requirements-v1.1.md:48` is "caches are rebuildable and never **roots of
trust**" — cite that, not the shorter "never authoritative" phrasing.

Your design must state, in the document, not only in code:

- **How a cached predecessor is validated before use.** `from_replay` is currently the only sanctioned
  constructor of a `ReplayDerivedLifecycleState`, and `lifecycle_cache.rs` says so in its own doc comment.
  Either it stays in the path, or you explain what replaces it — a design that quietly bypasses it is
  rejected.
- **What happens when the incremental result and an authoritative replay disagree, and which wins.**
- **How often a full replay re-anchors the chain**, so drift has a bounded lifetime.

## 4. "It cannot be done safely" is a permitted, and respectable, outcome

If incremental replay cannot be made safe without verification costing as much as the replay it replaces,
**report that**. With these measurements behind it, that is a finding that closes NFR-PERF-01 as
legitimately as code would — not a concession, and not a failure to deliver.

**Say it early rather than late.** DC-56 spent an entire increment before its scope finding surfaced; the
finding was still worth more than the code, but it would have been worth more sooner.

## 5. Traps

- **A cache keyed on the baseline block.** §2. It cannot hit.
- **Optimising the projection or the clones.** Worth 2.4%; you will have flattened nothing.
- **Measuring on a freshly prepared repository only.** A design whose key changes every cycle posts a
  perfect Axis A that way and misses on every real commit. **Measure across consecutive commit+seal
  cycles** — criterion 6 requires it, and it is the specific test the eliminated route would have failed.
- **Bypassing `from_replay`** to make the numbers work.
- **Absorbing the trust ladder's unbuilt slices.** `lifecycle_cache.rs` carries an unowned `MILESTONES.md`
  finding — 848 test-only lines awaiting blob-kind verification, provenance-vs-baseline staleness, and
  replay reconstruction/compare. You may well need exactly that machinery. **If you do, stop and report
  it**; it is a design question with no live owner, not scope to quietly take on.
- **Assuming the ~40 µs/op figure is release-mode.** It is debug. The shape is what the design turns on,
  but re-measure before any absolute claim.

## 6. Definition of done

Incremental replay from a validated cached predecessor, with a stated validation rule, a stated
disagreement resolution, and a bounded re-anchor interval; divergence detectable and reported; cache
deletion and rebuild leaving commit outcome byte-identical; **Axis A flattened across consecutive
commit+seal cycles**; memory above DC-62's floor no longer tracking repository size; NFR-PERF-01 recorded
in exactly one state; `MILESTONES.md` rows updated — **or** a reported finding under §4 with the evidence
that makes it one.

## 7. Submit with

The diff; the validation/disagreement/re-anchor design as a document, not only as code; DC-59's harness
re-run **across consecutive commit+seal cycles**, not a single prepared commit; DC-62's memory axis; the
deletion-and-rebuild test; a divergence test with a deliberately stale cached state; test counts per
touched crate before and after; an explicit statement of what did not change; and the full gate set from
`rfcs/EXECUTION-ORDER.md` §6 rule 9 run on a **clean checkout of the commit**, stated as such.

## 8. Standing request

Four increments in this program were redesigned or rescoped because implementation found what design review
missed — DC-57, DC-60, DC-61, and DC-56. **This RFC's own first draft is a fifth**, caught at design review
instead. If something here contradicts what the code actually does, stop and report it; that has been worth
more than the code every time.
