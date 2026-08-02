# Owner Decision Record — Multi-Commit Queuing

**Decided by the project owner, 2026-08-02.** Recorded by the architect.
**Decision: Option A — multi-commit queuing is a scheduled capability of prikk.**
**Options paper:** `.git-exclude/reviewed/prikk-multi-patch-queuing-options-v1.md`.

This closes the **one outstanding owner decision** in the development lane, open since 2026-07-29.

## What was decided

The active session will hold **N** unsealed patches rather than exactly one. `commit` will no longer refuse
on a non-empty WAL; `seal` becomes the publish boundary that batches queued patches into a block.

## Why — the reasons that actually carried

**1. The author/maintainer separation is currently unusable, not merely inconvenient.** `commit` is
author-signed (`main.rs:130`), `seal` is maintainer-signed (`main.rs:153`), and `commit` refuses on a
non-empty WAL. So **an author cannot make two commits in a row without a maintainer signing in between.**
Either the author holds the maintainer key — defeating the separation the signature roles exist to create —
or every second commit blocks on another person. For any repository where those are different people,
today's model does not work. That asymmetry decided it: **broken for teams, against marginally more to
explain for solo operators.**

**2. This is unfinished original design, not new scope.** Six independent places already assume it —
NFR-PERF-02, NFR-PERF-03, `prikk-app-requirements-v1.2.md` §6.3 ("unsealed **patch count**"), §6.4
("convert active WAL **patches** … into a block object"), §7.4 (thresholds in patches), and `layout.rs`'s
`active/**default**`, a directory layout already shaped for named sessions.

**3. The predictability cost is smaller than the architect's own options paper represented.** The paper
argued queuing "splits" committing from publishing and adds state users must track. **That state already
exists**: after `commit`, a patch sits unsealed until `seal` runs. Users already understand sealing,
already have unsealed work, already run two commands. Queuing **raises a cap from 1 to N** — it adds no
step and no concept, and it *removes* an error (`active WAL already contains patches; run prikk seal before
committing again`). Solo users may continue committing and sealing one-for-one; nothing forces
accumulation. Recorded because the paper understated this and the correction was material to the decision.

## What this does not decide

- **The threshold values.** NFR-PERF-02's 800/1000 become reachable, but whether they are the right numbers
  is DC-57's question, not this one.
- **Whether `seal` should remain maintainer-only.** Queuing makes the two-role split workable; it does not
  settle whether a solo operator should need a maintainer key at all. Separate question if it arises.
- **Ordering, drop, and reorder semantics** for queued patches. Design questions for the increment.

## Consequences

**NFR-PERF-02 and NFR-PERF-03 are no longer unmeetable**, and need no amendment. **DC-57 stays held** — its
premise is now scheduled rather than architecturally unreachable, so it is blocked on the queuing increment
rather than on a decision. Its withdrawn handoff stays withdrawn until then.

**The cost is the crash-recovery surface, not the append path.** A one-slot WAL has a small state space that
DC-38's recovery work covers. An ordered, partially-sealed queue does not, and `WalReplay`,
`trailing_partial_bytes`, and `doctor --repair-wal-tail` are built for the small case. **Any estimate that
scopes this as "let the WAL hold N records" is wrong** — the increment is mostly recovery, `verify`, and
ordering semantics.

## Sequencing — measurement first

The next step is **not** the queuing increment.

**DC-64's residual O(live node count) cost must be re-measured under batching first.** The one-record cap is
what forced every commit to present an unseen baseline; batched commits share one, so that cost may
amortize across a batch. If it largely does, the unowned residual-cost finding closes without separate work
and the queuing increment's performance obligations shrink. If it does not, that is worth knowing before
scoping.

One measurement, before designing — the pattern that caught DC-56's misattribution, DC-64's unhittable
cache key, and DC-65's invariant question. The architect discharges it at design review, as with DC-64.
