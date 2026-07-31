# RFC (proposed) - DC-64 Baseline Reconstruction Cost on the Commit Path

**Status.** **Proposed.** Authorized by the project owner on 2026-07-31, **scheduled behind DC-61 and
DC-58 closure**. Implementation may not begin until this RFC is accepted.
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

**Open design question, to be answered before acceptance:** is the sound form of this
(a) a **verified cache** — store the derived state keyed by `(baseline_block, horizon)`, and cheaply
*verify* rather than *trust* it, or (b) an **incremental replay** — replay only the delta since the cached
point, or (c) **neither**, if the honest answer is that reconstruction cost is inherent to a replay-based
model and NFR-PERF-01 needs amending instead. **(c) is a permitted outcome of this RFC**, and stating it
with evidence would close NFR-PERF-01 as legitimately as code would. DC-56 has already shown what happens
when a route is assumed rather than established.

## 4. What this requires that does not exist yet

**The mandatory prerequisites section.** DC-56, DC-59, and DC-60 each shipped a design whose premise had
not been checked; this section exists so it does not happen a fourth time.

| Prerequisite | State | Consequence if missed |
|---|---|---|
| A live governing RFC for `lifecycle_cache.rs` | **Absent.** DC-09 Phase 4.4-2b.1 is in `rfcs/archive/` | Design changes there have no owner and no current invariant statement |
| A statement of what the 848 unwired trust-ladder lines are for | **Partial** — a source comment only; `MILESTONES.md` records it as unowned | This increment may need exactly that machinery, or may duplicate it |
| Whether `replay_derived_state`'s cost is replay or projection | **Unmeasured.** The 375 ms is not yet split between replaying patches and projecting `live_nodes` into maps | A cache aimed at the wrong half saves nothing — DC-56's exact failure |
| Whether cost scales with patch count as well as file count | **Unmeasured.** Every measurement so far varies file count at a fixed short lineage | A design tuned to one may be defeated by the other |

**The third and fourth rows are blocking.** They are two more `Instant` probes and one benchmark axis —
the same cheap instrumentation that found this defect. **Measure before designing.**

## 5. Acceptance criteria

1. The two blocking prerequisites in §4 are measured and reported **before** a design is proposed.
2. The design states, explicitly, how a cached baseline is prevented from becoming a root of trust —
   including what happens when cache and replay disagree, and which one wins.
3. `from_replay` validation is not bypassed, or the design explains what replaces it.
4. Divergence between cached baseline state and authoritative replay is **detectable and reported**, on the
   DC-56 pattern.
5. Cache deletion and rebuild leave commit outcome byte-identical, tested — the NFR-PERF-04 obligation, on
   DC-56's `deleting_the_index_does_not_change_commit_outcome` pattern.
6. **Axis A flattened**, shown by re-running DC-59's harness — the criterion DC-56 could not meet.
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
