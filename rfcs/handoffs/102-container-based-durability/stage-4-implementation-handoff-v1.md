# RFC 102, Stage 4 — Implementation Handoff v1

**Authorized by the project owner 2026-08-14.** Design: `design-v1.md` §7 (Stage 4), and §2's container
rules.
**Stages 5–6 are not authorized.** Trust and compaction stay where they are.

## 1. What Stage 4 is

Move **ref publication state** — today's `refs/by-id/*.ref` pointers and `refs/logs/*.log` — into
containers, on the shape Stage 3 established: fixed names at `init`, append-only, checksum-framed,
read through the shared resync primitive.

## 2. The one thing Stage 3 does *not* hand you

**Stage 3's ordering ruling stops at objects, and this was drawn deliberately.**

`design-v1.md` §12's §10.1 ruling — *"no ordering constraint is required within a container"* — rests on
objects being content-addressed, so write order carries no meaning. **Ref logs are the opposite.**
`refs/verify/scan.rs:300-308` validates `update_seq != expected_seq` against the record's own index in
the log, and `:343` cross-checks it against the RefState. **Order is load-bearing, and a container that
loses it loses DC-38's audit trail.**

**Report, before any production code:**

1. **What exactly must be ordered** — the whole log, or per-ref sequences within a shared container?
   Derive it from `scan.rs`'s own checks, not from this paragraph.
2. **Can one container hold multiple refs' logs** while preserving each ref's own sequence? If it can,
   say how the reader reconstructs per-ref order; if it cannot, say so and propose the alternative.
3. **What happens to `refs/tmp/`'s candidate mechanism?** DC-91 established that no per-ref file shape
   avoids first-appearance at ref creation — containers are supposed to remove that. Confirm they
   actually do, and what becomes of `PRIKK-VERIFY-REF-CANDIDATE-DEBRIS`.

## 3. What must not change

- **DC-38's invariant.** *Format publication never permits an ahead log.* This is the guarantee the
  whole RFC 102 arc exists to make platform-independent — Stage 4 is where it is most at risk.
- **The `classify_ref_state` → `require_retained_evidence` ordering**, ruled load-bearing during DC-95.
- **The legacy-timestamp whole-set veto** stays whole-set (DC-95 Stage 2 Level 2, Q4). It is a
  migration-integrity claim about the repository, not a per-ref defect.
- **`ensure_no_incomplete_publication`'s chokepoint** and its six callers. If containerizing refs
  changes what that gate can observe, that is a finding, not an adjustment.
- **No `atomic_replace` on any container path.** Note `refs/pointer.rs:51` uses it today — that is one
  of the seven sites the RFC's §3 correction names, and it is in your scope this time.

## 4. Acceptance criteria

1. **Every new container name created at `init`** — enumeration, as Stage 3 proved it.
2. **No durability-bearing write uses `atomic_replace`.**
3. **Per-ref sequence order survives** — a log whose records are correct but reordered must still be
   rejected.
4. **DC-38's invariant holds**, proven the way DC-95 proved things: disable the check, observe the
   specific failure, restore.
5. **`ensure_no_incomplete_publication` still refuses** on a damaged ref container.
6. **DC-95's classification survives** — the sixteen rows of its own §2 `refs/verify.rs`/`scan.rs` cluster sit on these paths.
7. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus **green three-platform CI**.

## 5. Standing

- **Step 0 first**, reported and ruled before production code. Three stages running have each found
  something in Step 0 that would have been expensive later.
- A stop-and-report remains a complete outcome.
- Stage 4 merges before Stage 5 is scoped.
