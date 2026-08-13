# DC-95 Stage 2, Level 1 — Sweep Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-95-stage2-level1-sweep-v1.md`.

**Sweep accepted: one instance, and it is the one already ruled. Level 1's size is unchanged by it.**
Their §3 scope question, however, **reverses my own §5 ruling** — and for a stronger reason than the one
they give.

## 1. The sweep's discriminator is sound, and it is the reusable part

> A value produced **atomically** by one stage is safe by construction — a consumer that depends on it is
> itself `NotEvaluated`. The risk is specific to values that are **shared, mutable, and written across
> more than one stage's execution.**

That is the right test, and it is why the answer is one rather than many: `trust_verifier` is the only
value in the pipeline with that shape. `object_summary`, `ref_verification`, `replay` and
`active_wal_metadata_status` are each returned whole or not at all.

**Deriving the discriminator rather than enumerating cases is what makes "exactly one" a claim I can
accept** — an enumeration would only tell me what they happened to look at.

## 2. My §5 ruling on partial counts is withdrawn — it was wrong, not merely expensive

They flag that keeping partial counts requires restructuring three functions' return shapes, and ask
whether that is in Level 1's scope. **The scope question is real, but it is not the deciding one.**

`verify_objects` (`verify/objects.rs:82-93`) loops over object types accumulating into `summary`, and
**then, after the loop, calls `verify_blocks_topological` on the accumulated `pending_v2_blocks`** before
returning. That is a whole-store cross-object check.

**So a partial count from a mid-loop failure does not mean what my §5 said it meant.** It means *N
objects were scanned by per-type verification* — with the whole-store topological check **not run at all,
for any of them.** Reporting `checked_objects: 40000` alongside `Failed` invites precisely the reading
*"forty thousand objects were checked and were fine,"* which is not established for a single one of them.

**That is the same defect as `trust_is_valid`, in its third form.** A count from an incomplete stage is an
inference drawn from partial evidence, exactly as an empty accumulator from an unrun producer is. I ruled
§5 the other way after accepting their Step 0 §2 recommendation, and neither of us checked what the count
would actually be asserting.

## 3. Ruled

1. **A `Failed` stage's counts are absent, not partial — and not zero.** Zero is itself a claim
   (*"we looked and found none"*). The field must be unknown.
2. **No restructuring of `verify_objects`, `verify_refs` or `Wal::replay()`'s return shapes in Level 1.**
   The implementation handoff §4's scope stands as written, and this ruling settles the ambiguity they
   correctly identified in it: changing a stage function's return shape **is** out of Level 1's scope,
   as much as changing a check is.
3. **The central invariant is satisfied either way**, as they say — but only option (b) satisfies §3 of
   the design, which forbids a report that reads as complete when it is not. A partial count is a
   completeness claim in miniature.

**Level 1 gets smaller, not larger.** That is the right direction for a refactor whose failure mode is
losing a check.

## 4. The rule, in its general form, for `verify.rs`'s module doc

Three instances now, arrived at from three directions:

> **Do not report a result derived from a step that may not have run.** An empty accumulator is not
> "none found"; a partial count is not "this many verified"; a check's presence in the source is not
> proof a defect reaches it.

The third is Stage 1's upstream-gate rule, already documented. **The first two are the same rule seen
from the accumulation side, and they belong beside it** — written once, generally, rather than as three
anecdotes.

## 5. Standing

- **Sweep: accepted.** One instance, already ruled and unchanged.
- **§5 of the Step 0 ruling: withdrawn and replaced by §3 above.**
- **Level 1 implementation is cleared to begin.** No further prerequisite.
- The module-doc rule in §4 lands with Level 1, not as a separate change.
- Green three-platform CI before merge.

## 6. Note

They asked a yes/no scope question and did not assume an answer, on a point where assuming would have
been easy and would have doubled the increment. **That is the second time this round that stopping to
ask has been worth more than proceeding well** — Step 0 was the first.
