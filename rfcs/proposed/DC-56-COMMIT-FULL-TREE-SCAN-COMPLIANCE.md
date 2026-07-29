# RFC (proposed) - DC-56 Commit Full-Tree Scan Compliance

**Status.** Proposed. Requires design review before implementation may begin.
**Supersedes.** Item 1 of DC-42 (`rfcs/archive/DC-42-PERFORMANCE-MAINTAINABILITY-GATES.md`).
**Requirement.** **NFR-PERF-01** — `specs/prikk-non-functional-requirements-v1.1.md` §4.5, restated
functionally in `specs/prikk-app-requirements-v1.2.md` §6.2.
**Gate status.** Product **M1**. **Missed and carried** — see `MILESTONES.md` § "Two milestone schemes"
before resolving that label; it is not this file's corrective M1.
**Depends on.** **DC-59** (`rfcs/proposed/DC-59-COMMIT-BENCHMARK-HARNESS.md`) for the cost curve. DC-56
may not begin until DC-59's report exists.
**Touches.** `crates/prikk-store/src/worktree_patch/node_authoring/` and — only if the outcome is
compliance-by-design under the reading settled in §2 — the commit path's traversal model.

## Problem

The requirement is explicit and has two clauses:

> **NFR-PERF-01.** Commit cost is bounded by patch construction, signature, WAL append, and fsync; **no
> plugin scan or full-tree scan.** Gate: M1. Evidence: commit benchmark report.

`specs/prikk-app-requirements-v1.2.md` §6.2 states it as product behaviour: "Commit must not run audit
plugins or scan the full worktree."

**The full-tree clause is violated in production code today.** Verified during DC-42 design review v1:
`worktree_patch/node_authoring.rs:266` calls `enumerate_worktree_files(layout)`, which resolves to
`walk_dir` in `worktree_patch/node_authoring/worktree_files.rs:24`. That function recurses through
`list_directory` over the entire worktree mutation root, unconditionally, on every commit. Cost therefore
scales with repository size rather than with change size, which is precisely what the requirement forbids.

This is a **missed gate**, not scheduled work. The gate is product M1; product M1's capabilities shipped
long ago and the project has released through 0.17.7 without meeting it. DC-42 obscured this by presenting
it as ordinary corrective-M2 scope.

## Design

### 1. Measurement is DC-59's, and is a precondition

**Depends on DC-59** (`rfcs/proposed/DC-59-COMMIT-BENCHMARK-HARNESS.md`), which produces the cost curve
this decision rests on. Split out per design review v1 finding B2: the benchmark is a self-contained piece
of work with its own design questions, and DC-56 had allocated it a sentence.

**The report now exists** — DC-59 implemented at `a9c2fe0`, accepted 2026-07-29, report at
`rfcs/handoffs/DC-59-commit-benchmark-harness/benchmark-report-v1.md`. This precondition is satisfied.

**What it measured.** Axis A holds the change set at exactly one file and varies repository size:

| Repository size | Median commit |
|---:|---:|
| 10 files | 4.22 ms |
| 100 files | 8.86 ms |
| 1,000 files | 58.20 ms |
| 10,000 files | 516.46 ms |

Asymptotically linear in repository size — 6.6× cost for 10× size between 100 and 1,000, 8.9× between
1,000 and 10,000. Since the change set is fixed at one file throughout, neither patch construction nor
signing explains the growth; both scale with the change set and appear in Axis B instead
(1 → 1,000 changed files costs 57.77 → 1,682.98 ms). At 10,000 files, roughly **99% of commit cost is
attributable to something other than the change being committed.**

The full-tree scan is now measured rather than inferred.

**One caveat before drawing conclusions.** The run is on **tmpfs**. That is a reasonable choice for
isolating traversal cost — it removes fsync noise — and it does not affect Axis A's *shape*, which is the
claim under test. But NFR-PERF-01's bound explicitly names fsync, so **absolute** cost figures need a
journaling-filesystem run before this RFC relies on them for anything beyond the scan finding.

### 2. The requirement's meaning must be settled before the design is chosen

**This is DC-56's central unresolved question, and it is not the implementer's to answer.**

NFR-PERF-01 forbids a full-tree scan. NFR-PERF-04, in the same table, requires that "indexes and caches
improve performance but are **rebuildable and never authoritative**," with cache deletion/rebuild tests as
its evidence.

These pull against each other. A rebuildable changed-path index must be reconstructible from the
repository, and reconstructing it means reading the worktree — a full scan. So an index-based design
complies **only in the warm case**. Cold start, invalidation, or the deliberate cache deletion NFR-PERF-04
requires to be *tested* each fall back to the scan NFR-PERF-01 forbids.

Resolving this needs an answer to a question the requirement text does not settle:

> **Does NFR-PERF-01 bound steady-state commit cost, or every commit including the first?**

Both readings are defensible. Under the steady-state reading, an index complies and the cold-start scan is
an accepted rebuild cost. Under the strict reading, no cache-based design complies and only §3's route B
does.

**This is an owner decision** where it resolves toward reinterpreting or amending a requirement. DC-56's
design review must put it to the owner with DC-59's curve in hand — not resolve it silently, and not pass
it to implementation.

### 3. The design space, stated honestly

| Route | Removes the scan? | Status |
|---|---|---|
| **A — changed-path index or cache** | Warm case only; cold start rebuilds by scanning | Viable **only** under the steady-state reading. Binds NFR-PERF-04's evidence obligation |
| **B — explicit paths or a staging step** | Yes, unconditionally | **A user-visible CLI and workflow change.** `CommitArgs` (`crates/prikk-cli/src/args.rs:16-23`) carries only `message`, `ref_name`, and a compatibility `text_edits` flag — commit accepts no paths today. This is a product decision, not an architecture one |
| **C — amend the requirement** | N/A | Reviewed commit against `specs/`, owner-approved |

**Route B is out of DC-56's scope to perform.** If the interpretation question resolves strictly and route
B becomes the only compliant design, DC-56's outcome is to record that finding and hand off to a separate
product RFC. DC-56 does not change the CLI surface.

**Compliance remains the default and deferral carries the burden of argument.** Inherited from DC-42,
which was right about this.

An amendment is now an ordinary reviewed commit against `specs/` (DC-42 review v2 finding B2), so
"amended" is an executable outcome rather than an untracked assertion. It requires an accepted RFC stating
the rationale and the owner's approval, because requirement changes are a reserved decision.

**An amendment must state what the requirement *should* say and why the original was mistaken** — not
merely that compliance proved costly. A requirement is amended because it was wrong, not because it was
inconvenient.

### 4. The plugin clause

The requirement forbids plugin scans as well as full-tree scans. `PluginResultEntry` exists in the data
model (`crates/prikk-object/src/payload/attestation.rs`) but no commit-path plugin execution appears to
exist. The clause is very likely vacuous today, but half a requirement must not go unmentioned.

**State the verification procedure, do not leave it to judgement** (per review v1 N2): trace every call
reachable from `author_worktree_patch` and confirm none dispatches to plugin execution or attestation
evaluation; record the method and the result. If a plugin scan does reach the commit path, it is in scope
here.

### 5. Traversal semantics are a behaviour change

If the chosen design alters traversal semantics, caching, or repository authority, it is a **production
behaviour change** and requires a focused design amendment reviewed before coding — not a performance PR.
Caches introduced here are bound by **NFR-PERF-04**: rebuildable, never authoritative — and inherit its
stated evidence obligation, "cache deletion/rebuild tests"
(`specs/prikk-non-functional-requirements-v1.1.md` §4.5). That evidence is required, not optional, and
must not be re-derived at implementation time.

This is the boundary DC-42 drew correctly in advance and it is retained verbatim in intent.

## Non-goals

- **No benchmark construction — that is DC-59.** DC-56 consumes its report.
- **No CLI or workflow change.** Route B (explicit paths or staging) is recorded and handed off, never
  performed here.
- No ELOC or source-structure work — that is DC-58.
- No active-Patch threshold work — that is DC-57.
- No benchmark marketing claim, public API redesign, or unrelated refactor.
- No change to object identity, canonical encoding, or any persisted byte.

## Risks

**The obvious failure mode is measuring and stopping.** Reading DC-59's report, observing that commit is
fast enough on a small repository, and treating the requirement as addressed. Measurement alone does not
close NFR-PERF-01; the requirement is about cost *bounds*, not observed latency on one repository.

**The subtler one is answering §2's interpretation question by implementing.** Building an index and
declaring the warm-case curve compliant *is* an answer to the steady-state-versus-strict question — made
silently, by an implementer, without the owner deciding. That is the specific route this RFC's revision
exists to close.

**The second is an amendment of convenience** — amending the requirement because compliance is harder than
expected, rather than because the requirement is wrong. The amendment path exists for a genuinely
mistaken requirement, not for an inconvenient one.

## Acceptance criteria

1. **DC-59's report exists and is cited.** DC-56 does not build its own benchmark.
2. **The interpretation question in design §2 is answered on the record** — steady-state or strict — by
   the project owner, with DC-59's curve available. No design is chosen before this.
3. The plugin clause is verified by the stated procedure and the finding recorded.
4. NFR-PERF-01 ends in exactly one recorded state: **implemented and evidenced**, **amended** by a
   reviewed commit against `specs/` with owner approval, or **handed to a product RFC** where the strict
   reading makes route B the only compliant design.
5. If a cache or index is introduced: NFR-PERF-04's own evidence obligation is discharged — deletion and
   rebuild leave behaviour unchanged, tested.
6. If the outcome is implementation, the same DC-59 harness re-run shows Axis A flattened — commit cost no
   longer tracking repository size at fixed change count. *Conditional on outcome 4; this criterion does
   not presume compliance was achieved.*
7. `MILESTONES.md`'s missed-gate row is updated to its resolved state.
8. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

Criteria 1, 3, 5, 6, 7 are verifiable from the repository. Criterion 2 is verifiable as a recorded owner
decision and criterion 4 as a commit. **No criterion here requires trusting the implementer's report** —
the pattern DC-55's implementation review proved out.
