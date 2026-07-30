# DC-56 Commit Scan and Memory Compliance - Handoff v2

**Cleared to start.** Accepted by the project owner on 2026-07-30, at
`rfcs/accepted/DC-56-COMMIT-FULL-TREE-SCAN-COMPLIANCE.md`.
**Authored by** the architect. Design review v2 returned two blocking findings, both resolved at `38803f0`.
**Size:** the largest increment currently open. A new cache with validity rules on the commit hot path.
**Touches:** `crates/prikk-store/src/worktree_patch/node_authoring/`, the commit path's traversal **and
content-read** model, and a new changed-path index under `cache_dir()`.

**This supersedes handoff v1**, which was edited twice in place and may exist in three different states. Work
from this file.

## What changed from v1: the memory baseline is not your job

v1 told you to measure a memory baseline yourself, on the parent commit or with the index forced cold. **That
was my error and it is withdrawn.** A review finding against DC-62 is repaired under DC-62, not pushed into
the next increment — which is what the developers did, correctly, at `07b1fc8`.

So **DC-62 is complete at `07b1fc8`**, its harness now measures a floor (a real `commit` against a 1-file
repository) and publishes each point's excess over it. Because that landed before this increment exists, the
baseline is necessarily on the unmodified commit path already. Nothing about baselines is in your scope.

**What criterion 5 needs from you:** re-run the memory axis after your change and compare the **"Above
floor"** column — not absolute `VmHWM` — against `07b1fc8`'s published figures:

| Repository size | Above floor (KB), before your change |
|---:|---:|
| 10 files | 20 |
| 100 files | 152 |
| 1,000 files | 1,336 |
| 10,000 files | 13,256 |

Absolute `VmHWM` hides the effect you are claiming: across that range it grows only 2.58x where above-floor
grows 9.92x. **Quote the above-floor column or the claim is not evidenced.**

## What this closes

**NFR-PERF-01** — a **missed product-M1 gate**, not scheduled work. Product M1 shipped long ago and the
project released through 0.17.7 without meeting it.

> Commit cost is bounded by patch construction, signature, WAL append, and fsync; no plugin scan or
> full-tree scan.

DC-59 measured the violation. Axis A, change set fixed at exactly one file:

| Repository size | Median commit |
|---:|---:|
| 10 files | 4.22 ms |
| 100 files | 8.86 ms |
| 1,000 files | 58.20 ms |
| 10,000 files | 516.46 ms |

At 10,000 files roughly **99% of commit cost is attributable to something other than the change being
committed.**

## Two objectives, and the second is not in any requirement

**Objective 1 — latency.** Commit cost must stop tracking repository size at fixed change size.

**Objective 2 — memory.** `worktree_files.rs:11-14`:

```rust
pub(super) struct WorktreeFile {
    pub(super) bytes: Vec<u8>,
    pub(super) mode: u32,
}
```

`insert_regular_file` stores `file.bytes` for **every regular file**, into a `BTreeMap` that
`enumerate_worktree_files` returns whole and `author_inner` holds across authoring. So commit memory is
O(total worktree bytes) — a 1 GB worktree allocates 1 GB whether one byte changed or none.

**No requirement names this.** NFR-PERF-01 bounds cost in a latency sense. It is in scope because the same
index fixes it, and it is recorded in `MILESTONES.md` as an untracked scalability defect rather than absorbed
silently.

**Consequence for what you build:** the index must carry enough per-file state to **skip the read** — at
minimum size, mtime, and content hash. **A path-membership index alone satisfies neither objective**, because
it would still read every file. That is the difference between moving the curve and appearing to.

## The owner ruling that makes an index permissible

Ruled 2026-07-30: **NFR-PERF-01 bounds steady-state commit cost, not every commit including the first.**

Why the question arose: NFR-PERF-04 requires that "indexes and caches improve performance but are rebuildable
and never authoritative," and a rebuildable index must reconstruct itself by reading the worktree. Under the
strict reading the two requirements contradict each other — NFR-PERF-04 blesses indexes while NFR-PERF-01
would forbid building one. Steady-state is the only reading where both are satisfiable.

**The ruling is not a licence to scan.** A design that scans whenever the cache happens to be cold, with no
bound on how often that is, satisfies the letter and defeats the requirement.

> **You must specify cache validity: when the index is trusted, what invalidates it, and what bounds how often
> a rebuild occurs.** This RFC cannot be accepted complete without it. It is acceptance criterion 2.

## The index must not be able to be silently wrong

A changed-path index **determines what a commit contains.** If it wrongly reports a file unchanged — mtime
granularity, clock skew, a filesystem that misreports — the commit silently omits a real change and the patch
is wrong.

**Why an index is permissible at all:** `specs/prikk-non-functional-requirements-v1.1.md:48`, the §3
traceability row for "Performance and caching," glosses NFR-PERF-04 as **"Caches are rebuildable and never
roots of trust."** Root of trust here means identity and signature authority. An index is not that. Cite that
gloss — not the shorter "never authoritative" phrasing, which reads as prohibiting this and does not.

**Permissible is not safe, so:**

> **Index/worktree divergence must be detectable.** `verify` — or an equivalent explicit check — must be able
> to report that the index disagrees with the worktree, so a stale or wrong index is a *reported condition*
> rather than a silently incorrect commit.

Silence is the harm. NFR-REL-01 forbids silent data loss on uncertainty; an undetected omission from a commit
is a quieter form of the same thing. Acceptance criterion 6.

## Where the index lives

**Under `cache_dir()`.** `layout.rs:185` already defines it, it is in `required_directories()`, and
`ObjectType::BlockSummaryCache` maps to `"block-summary-cache-rebuildable"` — the layout already has a
sanctioned home for rebuildable caches and a naming convention that encodes the property. **Do not invent a
new location.**

NFR-PERF-04's own evidence obligation applies: **cache deletion and rebuild must leave behaviour unchanged,
tested.** Criterion 7.

## Already done for you — do not redo it

**The plugin clause is satisfied and verified.** NFR-PERF-01 forbids plugin scans as well as full-tree scans.
I searched the entire commit path for `plugin`, `attestation`, and `audit` — `worktree_patch.rs`,
`node_authoring.rs`, `worktree_files.rs`, `active.rs`, `wal.rs`, `refs.rs`. **Zero matches.**
`required_attestation_ids` exists on `RefStatePayload` but is not referenced from the commit path. No
verification work is needed here.

**The memory baseline.** Per §"What changed from v1".

## Traps

- **Building a path-only index.** Satisfies the RFC's word "index" and neither objective.
- **An unbounded cold path.** "Steady-state" is not a loophole; criterion 2 is what closes it.
- **An index that cannot be detected as wrong.** Criterion 6.
- **Putting the index anywhere but `cache_dir()`.**
- **Changing traversal semantics, caching, or repository authority beyond the index itself** — that is a
  production behaviour change needing its own reviewed amendment, not a performance PR.
- **Quoting absolute `VmHWM` as your memory evidence.** It understates the effect by roughly 4x. Use the
  above-floor column.
- **Treating the tmpfs caveat as absolute cost.** DC-59's run is on tmpfs, which is fine for Axis A's *shape*
  but means absolute figures need a journaling-filesystem run before anything depends on them.

## Definition of done

An index under `cache_dir()` carrying per-file state sufficient to skip reads;
a stated cache-validity design bounding rebuild frequency; index/worktree divergence detectable and reported;
Axis A flattened, shown by re-running DC-59's harness; memory improvement shown against DC-62's **above-floor**
column; NFR-PERF-04's deletion-and-rebuild evidence; `MILESTONES.md`'s missed-gate row and the memory finding
updated.

## Submit with

The diff; the cache-validity specification as a document, not only as code; DC-59's harness re-run showing
Axis A flattened; DC-62's memory axis showing the above-floor footprint no longer tracking worktree size; the
deletion-and-rebuild test; the divergence-detection test with a deliberately stale index; test counts per
touched crate before and after; an explicit statement of what did not change; and the full gate set from
`rfcs/EXECUTION-ORDER.md` §6 rule 9 run on a **clean checkout of the commit**, stated as such.

## Standing request

Three increments in this program were redesigned or scoped down because implementation found something design
review missed — DC-57, DC-60, DC-61. This RFC's own review missed, at two prior passes, that the commit path
reads file contents rather than only traversing, and v1 of this handoff assigned you work that belonged to
DC-62. If something here contradicts what the code actually does, stop and report it; that has been worth
more than the code every time.
