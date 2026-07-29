# RFC (proposed) - DC-56 Commit Full-Tree Scan Compliance

**Status.** Proposed. Requires design review before implementation may begin.
**Supersedes.** Item 1 of DC-42 (`rfcs/archive/DC-42-PERFORMANCE-MAINTAINABILITY-GATES.md`).
**Requirement.** **NFR-PERF-01** — `specs/prikk-non-functional-requirements-v1.1.md` §4.5, restated
functionally in `specs/prikk-app-requirements-v1.2.md` §6.2.
**Gate status.** Product **M1**. **Missed and carried** — see `MILESTONES.md` § "Two milestone schemes"
before resolving that label; it is not this file's corrective M1.
**Touches.** `crates/prikk-store/src/worktree_patch/node_authoring/`, a new benchmark, and — only if the
outcome is compliance-by-design — the commit path's traversal model.

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

### 1. Measure before changing anything

Build a repeatable benchmark recording, at minimum: repository size (file count and total bytes), changed-
path count, elapsed commit time, and the filesystem assumptions in effect. Publish the numbers before
proposing any change.

The benchmark must **demonstrate the scan**, not merely time a commit: show that commit cost tracks
repository size at a fixed change size. A single timing figure cannot distinguish a full scan from an
efficient one and would leave the central claim unevidenced.

DC-55 re-measured SHA-256 throughput on release hardware; that baseline is now stable and this benchmark
should be read against it rather than re-deriving it.

### 2. Then decide, with compliance as the default

Exactly one of:

- **Comply** — a changed-path or index design that removes the full-tree scan from the commit path.
- **Amend** — a reviewed requirements amendment against `specs/`, approved by the project owner, that
  defers or replaces the no-scan rule with a stated rationale.

**Compliance is the default and deferral carries the burden of argument.** Inherited from DC-42, which was
right about this.

An amendment is now an ordinary reviewed commit against `specs/` (DC-42 review v2 finding B2), so
"amended" is an executable outcome rather than an untracked assertion. It requires an accepted RFC stating
the rationale and the owner's approval, because requirement changes are a reserved decision.

### 3. The plugin clause

The requirement forbids plugin scans as well as full-tree scans. `PluginResultEntry` exists in the data
model (`crates/prikk-object/src/payload/attestation.rs`) but no commit-path plugin execution appears to
exist. **Verify this and record the finding** — the clause is very likely vacuous today, but half a
requirement must not go unmentioned. If a plugin scan does reach the commit path, it is in scope here.

### 4. Traversal semantics are a behaviour change

If the chosen design alters traversal semantics, caching, or repository authority, it is a **production
behaviour change** and requires a focused design amendment reviewed before coding — not a performance PR.
Caches introduced here are bound by **NFR-PERF-04**: rebuildable, never authoritative.

This is the boundary DC-42 drew correctly in advance and it is retained verbatim in intent.

## Non-goals

- No ELOC or source-structure work — that is DC-58.
- No active-Patch threshold work — that is DC-57.
- No benchmark marketing claim, public API redesign, or unrelated refactor.
- No change to object identity, canonical encoding, or any persisted byte.

## Risks

**The obvious failure mode is measuring and stopping.** Publishing a benchmark, observing that commit is
fast enough on a small repository, and treating the requirement as addressed. Measurement alone does not
close NFR-PERF-01; the requirement is about cost *bounds*, not observed latency on one repository.

**The second is an amendment of convenience** — amending the requirement because compliance is harder than
expected, rather than because the requirement is wrong. The amendment path exists for a genuinely
mistaken requirement, not for an inconvenient one.

## Acceptance criteria

1. A repeatable benchmark exists, is committed, and demonstrates commit cost against repository size at
   fixed change size — not merely a single latency figure.
2. The plugin clause is verified and the finding recorded.
3. NFR-PERF-01 ends in exactly one recorded state: **implemented and evidenced**, or **amended** by a
   reviewed commit against `specs/` with owner approval.
4. If implemented: commit cost demonstrably tracks changed-path count rather than repository size, shown
   by the same benchmark.
5. If any cache or index is introduced, NFR-PERF-04 compliance is evidenced — deletion and rebuild leave
   behaviour unchanged.
6. `MILESTONES.md`'s missed-gate row is updated to its resolved state.
7. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

Criteria 1, 4, and 5 are verifiable from the repository by a reviewer. Criterion 3's amendment branch is
verifiable as a commit. State which criteria a reviewer must take on report — there should be none.
