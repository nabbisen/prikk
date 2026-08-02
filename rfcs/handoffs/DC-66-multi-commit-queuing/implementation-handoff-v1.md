# DC-66 Multi-Commit Queuing - Handoff

**Cleared to start.** Accepted by the project owner on 2026-08-02, at
`rfcs/accepted/DC-66-MULTI-COMMIT-QUEUING.md`.
**Authored by** the architect. **Arises from** the owner decision at
`handoffs/DC-57-active-patch-thresholds/queuing-decision-record-v1.md` — read it, especially why the
decision was made (the author/maintainer separation, not performance).
**Unblocks** DC-57, held since 2026-07-29.
**Touches:** the active-session model, three append guards, `seal`, `verify`, `doctor`, `status`, and crash
recovery.

## 1. Read this before you estimate the work

**The feature will look like three deleted `if` statements. It is not.**

Verified at `81382b1`: `BlockPayload.patch_ids` is already `Vec<ObjectId>`; `seal`'s patch collection
already loops (`Vec::with_capacity(records.len())`, `for record in records`); the WAL already appends
records to a log. The cap is three `!replay.records.is_empty()` guards — `active.rs:67`,
`node_authoring.rs`, `rollback_draft.rs:136`.

The block format and `seal` tolerate N because they were **designed** for it. **Nothing downstream of the
queue has ever run with N > 1.** That is exactly the coverage blindness DC-65 proved this project has:
single-instance tests only ever exercise the first time a path runs against a given thing.

**The increment is crash recovery, `verify`, and ordering.** Budget accordingly, and if your estimate is
dominated by the append path, you have mis-scoped it.

## 2. Answer these four before designing. All blocking

Per the RFC §4. DC-56, DC-59, DC-60, and DC-64's first drafts were each scoped against an unchecked premise;
this is the standing countermeasure.

**The first one decides the shape of everything else:**

> **Does the second queued patch author against the first, or against the last *sealed* state?**

If against the first, queuing is a **chain** and node identity, DC-65's text materialization, and conflict
behaviour all follow from that. If against the last sealed state, it is **N independent patches** and they
may conflict with each other. These are different increments. Do not assume; establish it and say which.

The other three:

- Does `require_active_ref_for_non_empty_wal` still express the right invariant with N? (All queued patches
  must belong to one ref — the guard exists, but its semantics under N are unchecked.)
- What does `doctor --repair-wal-tail` **mean** when truncation discards one of N?
- Can DC-64's `apply_one_block` handle a block with N patch ids **today**? If not, DC-64's cache silently
  falls back on every sealed batch and you would ship an unmeasured performance regression.

## 3. The two places this will actually bite

**Selective data loss during repair.** Today `doctor --repair-wal-tail` discards *the* torn patch. With a
queue it discards one author's work while retaining others'. **Criterion 5 forbids silently discarding
complete queued patches** — a repair that truncates a torn k+1th record must not touch the k complete ones,
and must say what it did.

**Two freshly-built things that have never seen a queue.** DC-64's incremental baseline cache and DC-65's
text-edit materialization are both on the commit path, both landed within the last two days, and both have
only ever run at N = 1. DC-65's own fifth site was exactly this shape of interaction — a path that could not
be reached until an earlier fix made it reachable. **Criterion 8 requires testing both against a queue, not
inferring from N = 1.**

## 4. Traps

- **Estimating from the diff size of §1.** The guards are the smallest part.
- **Assuming DC-64 and DC-65 still work** because their tests pass. Their tests are all N = 1.
- **A repair that silently drops complete queued patches.** The worst outcome available here.
- **Folding in NFR-PERF-02's thresholds.** DC-57 owns those and becomes implementable once you land.
- **Scoping this as performance work.** Batching does **not** amortize DC-64's residual cost — measured
  2026-08-02, decision record §Sequencing. This is a capability increment.
- **Adding named or parallel active sessions.** `layout.rs`'s `active/default` suggests the shape, but one
  queue on one ref is this increment's scope.

## 5. Definition of done

§2's four questions answered and reported before a design; the baseline-for-the-next-queued-patch rule
stated with authoring and replay conforming; node identity safe across a queue (no two queued patches can
mint the same id), tested; `seal` batching N into one block; crash recovery covering a torn queue and a
crash during seal, with no silent loss of complete patches; `verify` reporting queue health; `status`
reporting queued count and target ref; **DC-64's cache and DC-65's materialization tested across a queue**;
the one-for-one workflow unchanged with existing tests passing unmodified or every change justified; a
coverage statement in DC-65's spirit; full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9 with test counts
before and after.

## 6. Submit with

The diff; §2's four answers as a document; the ordering/identity rule; the crash-recovery evidence
including the torn-queue and crash-during-seal cases; the DC-64 and DC-65 queue-interaction tests; the
coverage statement; test counts per touched crate before and after; an explicit statement of what did not
change; and the full gate set run on a **clean checkout of the commit**, stated as such.

**Report the gate commands verbatim** — `--locked`, `--no-fetch`, `+1.85.0`. DC-65's review carried a
non-blocking note for reporting paraphrased equivalents; the results were fine, but a substituted command
cannot be checked against the standard.

## 7. Standing request

Five increments in this program were redesigned or rescoped because implementation found what design review
missed — DC-57, DC-60, DC-61, DC-56, and DC-64's own first draft. **DC-65 found a severe long-standing
correctness bug by noticing that a benchmark failed for an unrelated reason and tracing it instead of
routing around it.** That has been worth more than the code every time. If something here contradicts what
the code actually does, stop and report it.
