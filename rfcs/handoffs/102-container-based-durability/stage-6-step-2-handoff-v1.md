# RFC 102, Stage 6 Step 2 — Implementation Handoff v1

**Authorized by the project owner 2026-08-15.** Design: `design-v1.md` **§15**, and **§15.7** in
particular — read that first, it carries four owner decisions and two hazards the owner named.

Step 1 merged at `4ad0021` with green three-platform CI. This is the last unit of RFC 102.

## 1. Before anything: what this stage is actually worth

**Measured, not estimated** (`FINDINGS.md`, taken at `4ad0021` over 20 signed commits): the compactor
reclaims **~6.5%** of per-commit growth. The pointer index is the only container that accumulates pure
garbage; object containers (73%) and the ref log (21%) are not reclaimable under current rulings, and
fixing that needs two features outside RFC 102 which the owner has **deliberately deferred**.

Say so if a design choice starts trading complexity for reclamation beyond that ceiling. **Finishing the
stage is still right** — format 6 already carries the slots and generation logs, and stopping would leave
allocated machinery with no purpose.

## 2. The four owner decisions (§15.7)

1. **Writers participate in a lock.** Optimistic publication was considered and rejected: it leaves a
   window after the generation record lands, and leaves the standing concurrency gap open with no other
   occasion to fix it.
2. **The lock is container-scoped**, not repository-wide `ActiveLock`. A container is one **logical**
   record stream — **slot pair plus generation log** — never a single file, because compaction writes
   `-b` while writers write `-a`.
3. **Stale-lock recovery is a prerequisite**, not a follow-up. See §4 below.
4. **The growth programme is deferred** with the measurement recorded. Do not widen into it.

## 3. Deadlock — declare a total lock order

Multi-container operations already exist: `trust.rs:111`/`:129` write the key *and* policy containers;
publication writes the ref log *and* the pointer index. Per-container locks make order inversion
possible.

**Declare one total order and enforce it structurally** — a single acquisition helper that takes the set
and orders it, not discipline at each call site. A comment saying "always take X before Y" is not an
enforcement mechanism.

## 4. The wedge — read this before adding any lock

`lock.rs:108-112`'s own lock body says it: *"note=PR-007 lock has no stale-lock stealing yet"*.
`acquire_lock_file` returns `LockConflict` on `AlreadyExists` and nothing else, so **a lock file
surviving a crash wedges that lock permanently**. And **`doctor.rs:405` acquires `ActiveLock` itself** —
the repair tool is inside the failure mode.

**Adding three container locks without recovery triples an already-unrecovered wedge.** Recovery lands
before or with them.

Design it, do not assume a shape. PID-based staleness is unreliable across reboots and containers; an
explicit `prikk unlock` with an operator confirmation is cruder and honest. **Report your reasoning
before building it** — this is a safety mechanism whose failure mode is "repository permanently
unusable."

## 5. Two obligations that are easy to lose

- **When is a retired slot safe to reuse?** Compaction *n* writes `-b`; compaction *n+1* reuses `-a`,
  which must be truncated first. A reader that resolved `-a` before publication and is still reading sees
  a truncated file — fail-closed, but a spurious failure. Candidate worth evaluating: the reader re-reads
  the generation log afterwards and retries if it moved.
- **`refs/verify/scan.rs:72-79`'s `pointer_locator` hardcodes slot `A`** and documents that as
  Step-1-correct. Once `-b` can be live, `verify`/`doctor` would name the wrong file for a damaged
  record — degrading the diagnostic an operator needs precisely when compaction has gone wrong.

## 6. Step 0 first — report before production code

1. **How wide the exclusion goes.** The tearing exposure is **not** limited to the three compaction
   targets: the shared ref log takes concurrent appends from the same unserialized commands
   (`branch create`, `tag create`, `merge`, `bundle import` — §15.7's traced inventory). **Put the width
   back to the owner**; do not settle it in implementation.
2. **The stale-lock recovery design** (§4).
3. **The total lock order** (§3).
4. **Retired-slot reuse** (§5).
5. **DC-41-grade recoverability** at the new state count — §9 criterion 5, deferred from Step 0.

## 7. What must not change

- **Compaction refuses on any known-corrupt record** (§15.3). No compacting around damage, no repair, no
  partial progress. A refusal is recoverable; a deletion is not.
- **The ref log and trust key containers are never compacted** — DC-38/DC-69 audit trail, and
  `trust.rs:77`'s TOFU history.
- **Criterion 2 stays closed for the repository.** No new `atomic_replace`, no name created outside
  `init`.
- **DC-95's classification**, on every path this touches.

## 8. Acceptance criteria

1. **The compactor publishes by appending a generation record**, after the new slot's bytes are durable —
   §5's ordering, one layer up.
2. **A crash between writing the slot and appending the record leaves the old generation authoritative** —
   shown, not argued.
3. **Compaction refuses on a corrupt container** — damage a record, observe the refusal, restore.
4. **Concurrent writer and compactor cannot interleave** — demonstrated, not asserted.
5. **A stale lock is recoverable**, including when `doctor` itself needs the lock.
6. **No deadlock under the declared order** — with a test that would fail if a new call site took locks
   out of order.
7. **Ref log and trust key containers untouched**, with a test that fails if a later change compacts them.
8. **`prikk compact`**, standalone, never folded into `doctor`.
9. **DC-41-grade recoverability re-earned** at the new state count.
10. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus **green three-platform CI**.
11. **`docs/src/reference/` reflects what this stage ships.**

## 9. Standing

- **Work on a branch.** `rfc102-stage6` is kept, merged and idle — continue on it or branch fresh.
- **Report counts** per rule 10. Baseline at `4ad0021`: `prikk-store` **709**, `prikk-object` 80,
  `prikk-replay` 44, `prikk-hash` 14, `prikk-crypto` 7, `prikk-release-policy` 83; **179 locked
  packages**. **Report the figure; the architect updates the line** — `rfcs/` is architect-only, which is
  why that obligation moved (rule 10's own note).
- A stop-and-report remains a complete outcome. Stage 5 produced four; Stage 6 Step 0 produced the
  finding that reshaped this stage.
