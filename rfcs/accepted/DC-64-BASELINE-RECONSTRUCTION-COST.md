# RFC (accepted) - DC-64 Baseline Reconstruction Cost on the Commit Path

**Status.** **Accepted by the project owner on 2026-07-31**, after design review v1 discharged the blocking
prerequisites by measurement and eliminated the RFC's own leading design option. **Implementation may
begin**; handoff at `handoffs/DC-64-baseline-reconstruction-cost/implementation-handoff-v1.md`.
**Unblocked** — DC-58 and DC-61 are both complete.

Design review v1 (`.git-exclude/reviewed/prikk-dc64-design-review-v1.md`) discharged the blocking
prerequisites by measurement and returned one blocking finding: the RFC's leading design option — a cache
keyed on `(baseline_block, horizon)` — **cannot hit at all**, because the one-record active-WAL cap forces a
seal between commits and every seal changes that key. §3.1 is rewritten around the route the measurements
do support. Recorded rather than quietly amended, because the RFC exists to correct exactly this failure.
**Authored by** the architect.
**Independence.** Authored and reviewed by the architect — the standing ceiling. But unusually, this RFC's
*problem statement* is not the architect's: it was established by the DC-56 implementation's scope finding
and independently re-measured before being written up. That is stronger than the usual position.
**Arises from.** DC-56's implementation, which built its index correctly and found the requirement's
dominant violator was misidentified. Ruling at `.git-exclude/reviewed/prikk-dc56-scope-finding-ruling-v1.md`.
**Requirement.** **NFR-PERF-01** — carried from DC-56, still a **missed product-M1 gate**.
**Touches.** `crates/prikk-store/src/lifecycle_cache.rs` and `patch_replay.rs` — **archived-RFC territory**,
see §4.
**Depends on.** DC-56 closing partial; DC-62's memory axis (complete, `07b1fc8`).

## 1. The measurement

Commit reconstructs its baseline by replaying the lineage on every invocation.
`node_authoring.rs:216-225` calls `resolve_worktree_baseline`, then `replay_derived_state`, then projects
`live_nodes()` into path-keyed maps and clones the result into `working_state`.

Measured at N=10,000 files, one changed file, debug build, on `8748f00` and its parent `ca4c044`, with
probes placed independently by the architect and by the developers (agreeing to 2.7%):

| Phase | parent `ca4c044` | with DC-56's index | share of commit |
|---|---:|---:|---:|
| **baseline reconstruction** | 374.0 ms | 375.1 ms | **72%** |
| scan and plan | 159.1 ms | 127.5 ms | 25% |
| whole-process wall | 548.1 ms | 519.0 ms | — |

**Linear in repository size**: 41.0 ms at N=1,000 → 375.1 ms at N=10,000 (9.15× for 10×).

### 1.1 Where inside the phase the cost is — §4's prerequisites, discharged 2026-07-31

Measured at `5b16b54` with sub-phase probes; full detail in the design review.

| Sub-phase | 1,000 files | 10,000 files |
|---|---:|---:|
| `resolve_worktree_baseline` | 0.074 ms | 0.075 ms |
| **`replay_derived_state`** | **38.1 ms** | **370.6 ms** |
| `.state().clone()` | 0.19 ms | 1.87 ms |
| `live_nodes()` projection | 0.36 ms | 4.77 ms |
| `working_state.clone()` | 0.17 ms | 2.43 ms |

**Replay is 97.6% of the phase.** Projection and both clones are 2.4% combined — **caching the projection
saves nothing.**

**And the cost is per *operation*, not per patch.** At a fixed 1,000 files, 50 extra sealed one-operation
patches cost 2.9 ms; but 20 patches carrying 100 operations each cost **121.2 ms against 37.5 ms** for the
same 20 patches carrying one. That is **~40 µs per operation**, consistent with the genesis baseline
(1,000 files ≈ 1,000 `CreateFile` ops ≈ 36 ms). Repository size dominates only because genesis contributes
one operation per file.

**This is the number a design must move**, and it supersedes §4's original "file count *or* patch count"
framing.

**Memory follows the same shape.** Peak `VmHWM` at N=10,000 is ~19.5 MB against DC-62's 6,144 KB floor —
~13 MB above floor — while the worktree's entire content is ~2.5 MB. **The resident cost is replayed node
state, not file content.** DC-56 removed the content and memory went *up* 1.1 MB, because its index is
itself resident.

## 2. Why this is not a DC-56 amendment

Ruled 2026-07-31. Three reasons, recorded so they are not relitigated:

1. **It is a different cache with a different validity question.** DC-56's index answers "did this path's
   content change," validated against `stat`. This one answers "what is the lineage's derived state,"
   validated against **replay authority** — provenance, staleness against the horizon, and what happens
   when the cache and a replay disagree.
2. **It lands in archived-RFC territory** and needs its own design review (§4).
3. **Amending DC-56 would be the "amendment of convenience"** that RFC's own traps warn against. The
   developers cited that warning when declining to widen scope; they were right.

## 3. The hard part is not caching — it is what the cache is allowed to be believed about

**NFR-PERF-04 is the binding constraint, and it bites harder here than it did for DC-56.**
`specs/prikk-non-functional-requirements-v1.1.md:48` glosses it as "**caches are rebuildable and never
roots of trust**." DC-56's index decides *what a commit contains*, which was already serious, and it was
made safe by a `stat`-based trust condition plus divergence detection in `verify`.

A baseline-state cache is worse: it decides **what the repository's history says the state is**. If it is
wrong, the commit is authored against a fictional baseline — wrong node identities, wrong parentage — and
the result is signed. **This is the closest any cache in this project has come to a root of trust, and the
design must keep it on the correct side of that line.**

The `lifecycle_cache.rs` doc comment already names `replay_derived_state` as "the only sanctioned way to
obtain a `ReplayDerivedLifecycleState`," validated through `from_replay` before use. **Any design that
lets a cached state bypass that validation is rejected in advance.**

### 3.1 The route — settled by measurement, not preference

Design review v1 (`.git-exclude/reviewed/prikk-dc64-design-review-v1.md`) put three routes to the
measurements in §1.1. One is now **eliminated**:

> **A cache keyed on `(baseline_block, horizon)` alone can never hit.** The active WAL is capped at one
> record, so `commit` refuses to run twice without an intervening `seal`; every seal advances the ref to a
> new block; so **every commit presents a `baseline_block` no previous commit has seen.** Such a cache
> misses 100% of the time and adds a write to every commit for nothing. This was the RFC's own leading
> proposal in its first draft, and it was wrong — the one-record cap is the same fact that put DC-57 on
> hold.

**The route is incremental replay from a cached predecessor state.** Between consecutive commits the
lineage grows by exactly one patch — a handful of operations at ~40 µs each, against a full replay of every
operation ever made. That is three to four orders of magnitude, and it is the only route the measurements
support. A keyed cache survives **only as the storage layer** for the predecessor state, never as the
mechanism itself.

**Route (c) — reporting NFR-PERF-01 as inherent — remains permitted and is now sharper:** if incremental
replay cannot be made safe without verification costing as much as the replay it replaces, that is the
finding, and stating it with evidence closes the requirement as legitimately as code would.

### 3.2 Incremental application makes the trust problem harder, not easier

A full replay is self-correcting: every commit reconstructs from the horizon, so an error cannot persist.
An incrementally-maintained state has no such property — **errors compound silently across cycles.** The
design must therefore state:

- how a cached predecessor state is **validated before use**, given `from_replay` is currently the only
  sanctioned constructor of a `ReplayDerivedLifecycleState`;
- what happens when the incremental result and an authoritative replay **disagree**, and which wins;
- how often, if ever, a full replay is forced to re-anchor the chain.

## 4. What this requires that does not exist yet

**The mandatory prerequisites section.** DC-56, DC-59, and DC-60 each shipped a design whose premise had
not been checked; this section exists so it does not happen a fourth time.

| Prerequisite | State | Consequence if missed |
|---|---|---|
| A live governing RFC for `lifecycle_cache.rs` | **Absent.** DC-09 Phase 4.4-2b.1 is in `rfcs/archive/` | Design changes there have no owner and no current invariant statement |
| A statement of what the 848 unwired trust-ladder lines are for | **Partial** — a source comment only; `MILESTONES.md` records it as unowned | This increment may need exactly that machinery, or may duplicate it |
| Whether `replay_derived_state`'s cost is replay or projection | **DISCHARGED 2026-07-31 — replay, 97.6%.** See §1.1 | A cache aimed at the wrong half saves nothing — DC-56's exact failure |
| Whether cost scales with patch count as well as file count | **DISCHARGED 2026-07-31 — neither: it scales with *operations*, ~40 µs each.** See §1.1 | A design tuned to one may be defeated by the other |
| Whether a `(baseline_block, horizon)` cache can hit at all | **DISCHARGED 2026-07-31 — it cannot**, one-record WAL cap; see §3.1 | An entire increment spent on a cache with a 0% hit rate |

**The three measured rows were blocking and are now discharged** (§1.1, §3.1), by the architect at design
review rather than handed to the implementer — they were prerequisites for *designing*, not for building.
They cost three probes and four runs, and they eliminated the RFC's own leading design proposal. **Measure
before designing.**

The first two rows remain open and are **not** blocking for this increment: they are ownership gaps in
`lifecycle_cache.rs`, recorded in `MILESTONES.md`, that DC-64 may or may not need to resolve. If the design
finds it needs the trust ladder's unbuilt slices, that is a finding to report, not scope to absorb.

## 5. Acceptance criteria

1. ~~The two blocking prerequisites in §4 are measured and reported before a design is proposed.~~
   **Discharged at design review, 2026-07-31** — §1.1 and §3.1. Nothing carries to the implementer.
2. The design states, explicitly, how a cached baseline is prevented from becoming a root of trust —
   including what happens when cache and replay disagree, and which one wins.
3. `from_replay` validation is not bypassed, or the design explains what replaces it.
4. Divergence between cached baseline state and authoritative replay is **detectable and reported**, on the
   DC-56 pattern.
5. Cache deletion and rebuild leave commit outcome byte-identical, tested — the NFR-PERF-04 obligation, on
   DC-56's `deleting_the_index_does_not_change_commit_outcome` pattern.
6. **Axis A flattened**, shown by re-running DC-59's harness — the criterion DC-56 could not meet. **The
   improvement must be shown to survive consecutive commit+seal cycles**, not one commit against a freshly
   prepared repository. That distinction is what exposes a cache whose key changes every cycle; a design
   measured only on a prepared repository can post a flat Axis A and still miss on every real commit.
7. **Memory above DC-62's floor no longer tracks repository size**, and DC-56's accepted ~1.1 MB index cost
   is either absorbed or restated.
8. **NFR-PERF-01 ends in exactly one recorded state**: implemented and evidenced, or **reported as
   inherent** with the evidence that makes that a finding rather than a concession.
9. `MILESTONES.md`'s missed-gate row and both `lifecycle_cache.rs` rows updated.
10. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

**Criterion 1 is load-bearing.** This RFC exists because a design was scoped against an unverified
attribution. It will not repeat that.

## 6. Non-goals

- Wiring the trust ladder's unbuilt slices. If this increment needs them, that is a finding to report.
- Changing what a commit *means* — node identity, parentage, canonical ordering.
- Revisiting DC-56's index. It works and stays.
