# RFC (proposed) - DC-66 Multi-Commit Queuing

**Status.** **Proposed 2026-08-02.** Awaits owner acceptance.
**Authored by** the architect.
**Independence.** Authored and reviewed by the architect — the standing ceiling
([[prikk-author-review-independence-ceiling]]). Compensated as usual by acceptance criteria written to be
reproducible from the repository, and by §4's prerequisites being discharged at design review rather than
assumed.
**Arises from.** The owner decision of 2026-08-02
(`handoffs/DC-57-active-patch-thresholds/queuing-decision-record-v1.md`).
**Unblocks.** **DC-57** (NFR-PERF-02's 800/1000 thresholds), held since 2026-07-29.
**Requirement.** `specs/prikk-app-requirements-v1.2.md` §6.3, §6.4, §7.4; **NFR-PERF-02**, **NFR-PERF-03**.
**Gate status.** Product **M3** — "active block limit behavior", missed and carried.
**Touches.** The active-session model, all three append guards, `seal`, `verify`, `doctor`, and crash
recovery.

## 1. What is being built

The active session holds **N** unsealed patches instead of exactly one. `commit` stops refusing on a
non-empty WAL. `seal` becomes the publish boundary, batching the queue into one block.

**Explicitly not a new step or concept for users.** The unsealed state already exists — after `commit`, a
patch sits in the active WAL until `seal` runs. This raises a cap from 1 to N, *removes* an error message,
and leaves the one-for-one workflow available to anyone who prefers it.

## 2. Most of this already exists — which is the trap, not the good news

Verified at `81382b1`:

| Surface | State |
|---|---|
| `BlockPayload.patch_ids` | **already `Vec<ObjectId>`** (`payload/block.rs:55`), decoder already pushes repeatedly |
| `seal`'s patch collection | **already loops** — `Vec::with_capacity(records.len())`, `for record in records` (`seal/support.rs`) |
| WAL append | already appends records to a log |
| The cap | **three `!replay.records.is_empty()` guards** — `active.rs:67`, `node_authoring.rs`, `rollback_draft.rs:136` |

**So the feature appears to be three deleted `if` statements. It is not.** Anyone scoping this from the
table above will underestimate it by an order of magnitude, and this section exists to stop that.

The block format and seal already tolerate N because they were designed for it. **Nothing downstream of the
queue was ever exercised with N > 1**, which is precisely the coverage shape DC-65 proved this project is
blind to: single-instance tests only ever exercise the first time a path runs.

## 3. Where the increment actually is

**Crash recovery.** A one-slot WAL has a small state space, and DC-38's recovery work plus
`doctor --repair-wal-tail` cover it. An ordered, partially-written queue does not:

- `WalReplay.trailing_partial_bytes` currently means "one record, torn tail." With N it means "k complete
  records plus a torn k+1th" — recoverable by truncation, but the *meaning* of the repair changes: it now
  discards one author's work while retaining others'.
- What does `doctor` offer when the queue is intact but the ref metadata is missing or malformed? Today
  that state fails closed for a single patch (DC-61's N1). With N it fails closed for a batch.
- Crash **during** seal, with a block partially written against a queue of N.

**`verify` and reporting.** What is a healthy queue? Ordering, ref ownership, and per-patch signature
validity all become plural. `status` must say how many patches are queued and against which ref.

**Ordering and identity.** Are patches sealed strictly in append order? Node identity depends on it —
`node_authoring.rs` mints fresh ids in canonical path order and inserts into `working_state` immediately so
the next mint sees them. Across queued patches this must hold **transitively**, or two queued patches can
mint the same id.

**Interaction with what has just been built.** DC-64's `apply_one_block` must handle a block carrying N
patch ids, not one. DC-65's text-edit materialization must work across a queue where an earlier *unsealed*
patch is the baseline for a later one. **Both are on the critical path and neither has ever run that way.**

## 4. What must be established before designing — blocking

DC-56, DC-59, DC-60, and DC-64's first drafts were each scoped against an unchecked premise. This section
is the standing countermeasure.

| Question | Why it blocks |
|---|---|
| Does the second queued patch's authoring see the first as baseline, or the last *sealed* state? | This decides whether queuing is "N independent patches" or "a chain." Node identity, DC-65's text materialization, and conflict behaviour all follow from it. **The single most important question in this RFC** |
| Does `require_active_ref_for_non_empty_wal` still express the right invariant? | All queued patches must belong to one ref. The guard exists; whether its semantics survive N is unchecked |
| What does `doctor --repair-wal-tail` mean when truncation discards one of N? | Today it discards "the" patch. With N it makes a selective-loss decision no one has authorized |
| Can `apply_one_block` (DC-64) handle a block with N patch ids today? | If not, DC-64's cache silently falls back on every sealed batch, and the increment ships a performance regression it did not measure |

**All four are answerable by reading and one probe. Answer them before proposing a design.**

## 5. Acceptance criteria

1. §4's four questions answered and reported **before** a design is proposed.
2. The **baseline-for-the-next-queued-patch** rule is stated explicitly and both authoring and replay
   conform to it.
3. Node identity is safe across a queue: no two queued patches can mint the same `node_id`, tested against
   constructed state.
4. `seal` batches N patches into one block, and the resulting block is byte-identical to what N separate
   seals would have produced **only if** that is the stated intent — otherwise the difference is stated and
   justified.
5. Crash recovery covers a torn queue: k complete records plus a partial k+1th, and a crash during seal.
   **`doctor`'s repair must not silently discard complete queued patches.**
6. `verify` reports queue health — ordering, single-ref ownership, per-patch signature validity.
7. `status` reports the queued patch count and its target ref.
8. **DC-64's cache and DC-65's text materialization both work across a queue**, tested — not assumed from
   the fact that they work at N = 1.
9. The one-for-one workflow still works unchanged; existing tests pass without modification, or every
   modification is justified.
10. A coverage statement in DC-65's spirit: what class of N > 1 behaviour was previously untested, and what
    was added.
11. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

**Criteria 5 and 8 are load-bearing.** 5 because selective data loss during repair is the worst outcome
available here; 8 because both are freshly built, on the commit path, and have never run against N > 1.

## 6. Non-goals

- **NFR-PERF-02's threshold values.** DC-57 owns those and becomes implementable once this lands.
- **Performance.** Batching does not amortize DC-64's residual cost — measured 2026-08-02, see the decision
  record §Sequencing. Do not scope this as a performance increment; it is a capability increment.
- **Whether `seal` should remain maintainer-only.** Named as undecided in the decision record.
- **Named/parallel active sessions.** `layout.rs` already has `active/default`, which suggests the shape,
  but one queue on one ref is this increment's scope. Multiple concurrent sessions is a separate question.
