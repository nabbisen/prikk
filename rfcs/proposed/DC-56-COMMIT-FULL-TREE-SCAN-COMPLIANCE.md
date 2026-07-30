# RFC (proposed) - DC-56 Commit Full-Tree Scan Compliance

**Status.** Proposed. Requires design review before implementation may begin.
**Owner ruling 2026-07-30:** NFR-PERF-01 bounds **steady-state** commit cost, not every commit. This
resolves the RFC's central open question and selects route A (changed-path index). It carries a binding
obligation — see §2's cache-validity requirement.
**Supersedes.** Item 1 of DC-42 (`rfcs/archive/DC-42-PERFORMANCE-MAINTAINABILITY-GATES.md`).
**Requirement.** **NFR-PERF-01** — `specs/prikk-non-functional-requirements-v1.1.md` §4.5, restated
functionally in `specs/prikk-app-requirements-v1.2.md` §6.2.
**Gate status.** Product **M1**. **Missed and carried** — see `MILESTONES.md` § "Two milestone schemes"
before resolving that label; it is not this file's corrective M1.
**Depends on.** DC-59 for the cost curve — **satisfied**; implemented `a9c2fe0`, report at
`rfcs/handoffs/DC-59-commit-benchmark-harness/benchmark-report-v1.md`.
**Touches.** `crates/prikk-store/src/worktree_patch/node_authoring/`, the commit path's traversal **and
content-read** model, and a new changed-path index under `cache_dir()` with its validity and
divergence-detection rules.

## Problem

The requirement is explicit and has two clauses:

> **NFR-PERF-01.** Commit cost is bounded by patch construction, signature, WAL append, and fsync; **no
> plugin scan or full-tree scan.** Gate: M1. Evidence: commit benchmark report.

`specs/prikk-app-requirements-v1.2.md` §6.2 states it as product behaviour: "Commit must not run audit
plugins or scan the full worktree."

**The full-tree clause is violated in production code today, and more severely than a traversal cost.**
`worktree_patch/node_authoring.rs:266` calls `enumerate_worktree_files(layout)`, resolving to `walk_dir` in
`worktree_patch/node_authoring/worktree_files.rs:24`, which recurses `list_directory` over the entire
worktree mutation root on every commit.

**It also reads every file's full contents.** `worktree_files.rs:11-14`:

```rust
pub(super) struct WorktreeFile {
    pub(super) bytes: Vec<u8>,
    pub(super) mode: u32,
}
```

`insert_regular_file` calls `read_file_state_if_exists` and stores `file.bytes` for every regular file, into
a `BTreeMap` that `enumerate_worktree_files` returns whole and `author_inner` holds across authoring. So
commit cost is dominated by **content reads**, not directory walking — and commit memory is O(total worktree
bytes) regardless of change size.

Design review v2 found that earlier drafts of this RFC described only the traversal, having cited
`worktree_files.rs` by line number without reading what it stores. That framing would have permitted an
index that still read every file.

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

### 2. The requirement's meaning — RESOLVED by owner ruling 2026-07-30

**Ruling: NFR-PERF-01 bounds steady-state commit cost, not every commit including the first.**

The question was whether the requirement forbids a full-tree scan *ever*, or only as a recurring per-commit
cost. It arose because NFR-PERF-04, in the same table, requires that "indexes and caches improve performance
but are **rebuildable and never authoritative**," with cache deletion/rebuild tests as its evidence — and a
rebuildable changed-path index must reconstruct itself by reading the worktree.

**Rationale, recorded because the reading is now load-bearing.** Under the strict reading the two
requirements *contradict* each other: NFR-PERF-04 blesses indexes while NFR-PERF-01 would forbid the only
way to build one. Nothing in the specs suggests the conflict was intended. Steady-state is the only reading
under which both requirements are simultaneously satisfiable, which is why it was chosen — not because it is
the cheaper route.

**The ruling is not a licence to scan.** A design that scans whenever the cache happens to be cold, with no
bound on how often that is, satisfies the letter and defeats the requirement. So:

> **DC-56 must specify cache validity — when the index is trusted, what invalidates it, and what bounds how
> often a rebuild occurs.** An unbounded cold path is not compliance under the steady-state reading, and
> this RFC may not be accepted without that specification.

That obligation is the price of the ruling and is treated as a first-class requirement here, not a caveat.

### 3. Two objectives, not one

The ruling settles latency. The content reads add a second objective that the same fix addresses but that
needs its own evidence.

**Objective 1 — latency.** Commit cost must stop tracking repository size at fixed change size. Evidenced
by DC-59's Axis A flattening.

**Objective 2 — memory.** Commit must stop loading the whole worktree into memory. A 1 GB worktree currently
allocates 1 GB whether one byte changed or none.

**Objective 2 is not covered by any requirement.** NFR-PERF-01 bounds cost in a latency sense; nothing names
the memory footprint. It is in scope here because the same index fixes it and excluding it would be
artificial — but it is a **previously untracked scalability defect**, recorded as such in `MILESTONES.md`
rather than absorbed silently into a performance increment.

**Consequence for the index's contents.** To satisfy objective 2 the index must carry enough per-file state
to *skip the read* for an unchanged file — at minimum size, mtime, and content hash. A path-membership index
alone satisfies neither objective: it would still read every file. State this explicitly; it is the
difference between moving the curve and appearing to.

**Consequence for evidence.** DC-59's harness measures wall-clock only. Objective 2 needs a memory axis,
which is a **DC-59 amendment** — DC-56 cannot assert memory improvement against a harness that does not
measure it.

### 4. The design space, now narrowed

| Route | Status after the ruling |
|---|---|
| **A — changed-path index or cache** | **Selected.** Viable under the steady-state reading. Binds NFR-PERF-04's evidence obligation — cache deletion and rebuild must be tested — plus §2's cache-validity specification |
| **B — explicit paths or a staging step** | **Not needed and out of scope.** It was only mandatory under the strict reading. `CommitArgs` (`crates/prikk-cli/src/args.rs:16-23`) accepts no paths today, and adding them is a user-visible workflow change — a product decision, not this RFC's |
| **C — amend the requirement** | **Not needed.** The requirement is satisfiable as written under the ruling |

**Compliance remains the default and deferral carries the burden of argument.** Inherited from DC-42, which
was right about this.

Should implementation nevertheless find route A unachievable, the outcome is to report that rather than
quietly widen scope into route B or reopen the amendment path — either would be a change to what the owner
decided.

### 5. The index must not be able to be silently wrong

DC-56's earlier drafts said caches are "bound by NFR-PERF-04: rebuildable, never authoritative," as though
that settled the question. It does not.

**A changed-path index determines what a commit contains.** If it wrongly reports a file unchanged — mtime
granularity, clock skew, a filesystem that misreports — the commit silently omits a real change and the
patch is wrong.

**Why an index is permissible at all.** `specs/prikk-non-functional-requirements-v1.1.md:48`, the §3
traceability row for "Performance and caching," glosses NFR-PERF-04 as **"Caches are rebuildable and never
roots of trust."** Root of trust in this project means identity and signature authority — the trust store,
signed objects, state roots. A changed-path index is none of those, so NFR-PERF-01 and NFR-PERF-04 do not
conflict here. Cite that gloss; do not rely on the shorter "never authoritative" phrasing, which reads as
prohibiting this and does not.

**Permissible is not safe, and this is the price of using one:**

> **Index/worktree divergence must be detectable.** `verify` — or an equivalent explicit check — must be
> able to report that the index disagrees with the worktree, so a stale or wrong index is a *reported
> condition* rather than a silently incorrect commit. A design that cannot detect its own staleness may not
> be accepted.

Silence is the specific harm. NFR-REL-01 forbids silent data loss on uncertainty; an undetected omission
from a commit is a quieter form of the same thing.

### 6. The plugin clause

**Discharged at design review v2 — no implementation work required.**

The requirement forbids plugin scans as well as full-tree scans. `PluginResultEntry` exists in the data
model (`crates/prikk-object/src/payload/attestation.rs`), so the clause is not vacuous by construction and
had to be checked rather than assumed.

**Method and result:** searched the entire commit path for `plugin`, `attestation`, and `audit` references —
`worktree_patch.rs`, `worktree_patch/node_authoring.rs`, `worktree_patch/node_authoring/worktree_files.rs`,
`active.rs`, `wal.rs`, `refs.rs`. **Zero matches.** `required_attestation_ids` exists on `RefStatePayload`
but is not referenced from the commit path. No plugin execution is reachable from `author_worktree_patch`.

**The plugin clause of NFR-PERF-01 is already satisfied.** Recorded here so the implementer does not
re-derive it, and so a later reader knows it was checked rather than waved through.

### 7. Traversal semantics are a behaviour change

If the chosen design alters traversal semantics, caching, or repository authority, it is a **production
behaviour change** and requires a focused design amendment reviewed before coding — not a performance PR.
**Place the index under `cache_dir()`.** `layout.rs:185` already defines it, it is in
`required_directories()`, and `ObjectType::BlockSummaryCache` maps to `"block-summary-cache-rebuildable"` —
so the layout already has a sanctioned home for rebuildable caches and a naming convention that encodes the
property. Do not invent a new location.

Caches introduced here are bound by **NFR-PERF-04**: rebuildable, never a root of trust — and inherit its
stated evidence obligation, "cache deletion/rebuild tests"
(`specs/prikk-non-functional-requirements-v1.1.md` §4.5). That evidence is required, not optional, and
must not be re-derived at implementation time.

This is the boundary DC-42 drew correctly in advance and it is retained verbatim in intent.

## Non-goals

- **No benchmark construction — that is DC-59.** DC-56 consumes its report.
- **No CLI or workflow change.** Route B (explicit paths or staging) is recorded and handed off, never
  performed here.
- No plugin-scan work — the clause is already satisfied and was verified at design review (§6).
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
2. **§2's cache-validity specification exists**: when the index is trusted, what invalidates it, and what
   bounds rebuild frequency. The owner ruling of 2026-07-30 settled the reading; this criterion is what
   stops that ruling becoming a loophole.
3. **The index carries per-file state sufficient to skip reads** for unchanged files — at minimum size,
   mtime, and content hash. A path-membership-only index fails this criterion.
4. **Objective 1, latency:** commit cost no longer tracks repository size at fixed change count, shown by
   re-running DC-59's harness and reporting Axis A flattened.
5. **Objective 2, memory:** commit no longer loads the whole worktree into memory, shown against a
   **DC-59 memory axis** — which must be added first. DC-56 may not assert memory improvement against a
   harness that measures only wall-clock.
6. **Index/worktree divergence is detectable and reported**, per §5. A design that cannot detect its own
   staleness does not satisfy this.
7. NFR-PERF-04's evidence obligation is discharged — cache deletion and rebuild leave behaviour unchanged,
   tested. The index lives under `cache_dir()`.
8. NFR-PERF-01 ends in exactly one recorded state: **implemented and evidenced**, or **reported as
   unachievable under route A** — the amendment and product-RFC branches are closed by the 2026-07-30
   ruling.
9. `MILESTONES.md`'s missed-gate row is updated to its resolved state, and the memory finding recorded
   there is closed or carried explicitly.
10. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

All ten are verifiable from the repository. Criteria 3, 5, and 6 are the ones added by design review v2 and
are the ones most easily satisfied in appearance only: an index that indexes paths but still reads every
file would pass criterion 4 and fail the increment's purpose.

## Sequencing note

Criterion 5 makes **DC-59 a second-time dependency**: its harness needs a memory axis before DC-56 can
evidence objective 2. That amendment is small and independent — it can be prepared while DC-56's remaining
design work proceeds, but it must land before DC-56's implementation review.
