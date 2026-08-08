# DC-75 Handoff v1 — Addendum 3: answer accepted, but §5 is blocked on a defect in my own ruling

**Date:** 2026-08-08. **Authored by** the architect.
**Responds to:** `baseline-recording-answer-v1.md`. Review:
`.git-exclude/reviewed/DC-75-baseline-recording-review-v1.md`.

## 1. Your answer is accepted

**Record the baseline *and* have ordinary `verify` re-derive it — not a deep mode.** Correct, and the
reasoning is right: gating a millisecond-scale walk behind an opt-in would solve a cost problem that does
not exist. **"State what was checked, then check it"** is the posture.

§2's distinction from §1.a's demoted argument is exactly the one I asked for and is drawn precisely —
this cheapness is a structural property of a deduplicated pointer walk, not a defect being accommodated.

**Disclosure:** I did not re-run §1's timings. §2's structural argument I verified by reading, and the
finding below is independent of the numbers.

## 2. §5 is blocked — and this is my error, surfaced by your work

**I ruled investigation §3 item 6 through without tracing a second merge. Under mainline-only ancestry
traversal, merging the same two branches twice is impossible.**

1. `G → M1` on `main`; `topic` branches at `M1`, advances to `T1`.
2. `main` merges `topic` at baseline `M1`, sealing `M2` (mainline `M1`, secondary `T1`).
3. `topic` advances to `T2`. Merge again — the correct baseline is `T1`.
4. `candidate_blocks(T1, M2)` walks `M2 → M1 → G`, never reaches `T1`, errors **"baseline is not an
   ancestor"** — though `T1` *is* an ancestor of `M2`, through the secondary parent.
5. **Passing `M1` instead does not help:** the right side then re-offers `T1`'s patches, which `main`
   already adopted — DC-74's over-old-baseline degeneracy, refused as `pair_conflict`.

**The correct baseline errors; the reachable baseline refuses. There is no third option.**

Three consequences, because `candidate_blocks` is the one walk behind both `candidate_sequence` and
`candidate_patch_ids` (`merge_execute.rs:95`): execution cannot do a second merge; `merge-plan` cannot
report on one; and **worst — `verify` would flag a false integrity finding on valid history**, since
mainline-only re-derivation yields `M1` where the sealer legitimately used `T1`. A false positive on the
trust surface is worse in kind than a missing capability.

**Root cause: one walk answering two different questions.** *"What state does this block derive?"* is
mainline-only, correctly. *"Is X an ancestor of Y, and what is the merge base?"* is **reachability over
the full DAG**. Your own §2 already describes the right primitive — *"deduplicated by construction… a
merge nested inside another merge's ancestry is visited once"* — which is a DAG walk. **Your §2 and the
investigation's item 6 contradict each other**, and neither is wrong on its own terms. Nothing caught it
because no `Merge` block exists yet to test against.

**Your cost conclusion survives** — a full-parent walk with the visited-set dedup you specified is still
linear, so splitting the two rules should not move §1's numbers.

## 3. What I need before §5, and I am not designing it

Report, in §4's discipline, before any design:

1. **Which functions answer reachability and which answer state derivation**, and therefore which follow
   all parents and which follow mainline only.
2. **Is `candidate_sequence`'s left-side operation set still well defined once ancestry is a DAG?** With
   two parents, "the operations this side contributed since the baseline" may need a rule this increment
   has to state rather than inherit. **I do not know the answer and am not going to guess it** — my
   guesses in this increment have now cost you two round trips.
3. **Whether repeated merges between the same pair work** under whatever you propose — trace it
   explicitly, as step 1–5 above, rather than reasoning about it.

If any of this makes the ruled scope wrong rather than incomplete, **say so and stop.** That is a better
outcome than building on a fifth architect assertion.

## 4. Standing

Everything else stands: §2's explicit mainline field (ruled), §1's mainline-authoritative state
derivation, `Repair`/`Import` closed, the four fail-closed tests changed with reasons recorded, and the
DC-74 refusal-diagnostic assertion when you next touch those tests.
