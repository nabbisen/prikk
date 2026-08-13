# DC-95 Stage 2, Level 2 — Handoff v1

**Authorized by the project owner 2026-08-13.** **Blocked until Level 1 merges** —
`stage-2-design-v1.md` §9: *"Level 1 must merge before Level 2 begins."* That is a real dependency, not
a formality: Level 2 changes code Level 1 just restructured, and stacking on an unmerged branch is how a
review loses track of which half a failure came from.

**Do not start until `dc-95-stage2-level1-scope-containment` is merged to `main`.**

## 1. What Level 2 is

**Item containment inside the two iterating stages.** Level 1 contained failures at the twelve stage
boundaries; a malformed object still aborts the whole `Objects` stage, hiding every other object's
verdict. Level 2 pushes containment one level in:

- **`verify_objects`** — per object.
- **`verify_refs`** — per ref.

One bad item yields a finding for that item; the rest of the stage still evaluates.

## 2. Step 0 again, and for a better reason than last time

**Before any production code: establish what an "item" is in each of the two stages, from the code.**

Level 1's Step 0 was worth a round because my dependency graph was wrong. Level 2's is worth more,
because **the item boundary is not obviously where it looks.** `verify_objects`
(`verify/objects.rs:82-93`) loops over object *types*, accumulating `pending_v2_blocks`, and **then runs
`verify_blocks_topological` across everything it collected.** That trailing whole-store check is not
per-item and cannot be made per-item — it is a property of the set.

So `verify_objects` has **at least two** natural units, not one: the per-object pass and the
whole-store topological pass. Report:

1. What the real item boundaries are in each stage, and whether the trailing whole-store work should
   become its own contained sub-stage rather than being folded into either.
2. **What a per-item failure means for the counts.** Level 1 ruled a failed stage's counts absent rather
   than partial, precisely because a partial count asserted something untrue. Under item containment, a
   count of successfully-verified items may become genuinely meaningful — **or may not**, if the trailing
   set-level check still hasn't run. Do not assume this reverses the Level 1 ruling; derive it.
3. Whether `verify_refs` has the same shape or a genuinely per-ref one.

**A stop-and-report is available**, as always. If item containment turns out to mean restructuring
`verify_objects`'s return shape — which Level 1 explicitly ruled out of its own scope — say so and stop;
that is a scope question for me, not a decision to absorb.

## 3. Constraints carried forward, unchanged

- **No check is rewritten, moved, or deleted.** Same rule as Level 1. If one appears to need conversion,
  that is a finding.
- **The three provably unreachable checks stay** — topological cycle, duplicate pointer identity,
  duplicate ref-log identity. Round 6's ruling survives Level 2 as it survived Level 1. **Note that the
  topological-cycle check lives in exactly the trailing whole-store pass this round is restructuring** —
  that proximity is the risk.
- **`classify_ref_state` → `require_retained_evidence` ordering is load-bearing.** Do not reorder.
- **`repair_repository` must refuse for every defect it refuses today.** Per-item findings must be
  blocking on the same footing.
- **No result derived from a step that may not have run** — the module-doc rule Level 1 landed. Item
  containment creates new instances of exactly this: a per-item "clean" verdict is not clean if a
  set-level check never ran.

## 4. Keep the whole-map assertion pattern

Level 1's tests assert over the **entire** stage map rather than per stage, and that caught two
regressions a per-stage assertion would have missed — the uncontained `CommitIndex`, and the fabricated
`blocked_by`. **Do the same for items:** assert the whole per-item outcome set, not the presence of one
expected entry.

## 5. Acceptance criteria

1. **Two independent bad items in the same stage are both reported.** This is Level 2's whole point and
   the criterion Level 1 could not satisfy.
2. **A per-item failure does not suppress the stage's remaining items, nor the trailing set-level work
   where that work is still valid.**
3. **Counts mean what they claim** — whatever §2.2 concludes, the report must not assert a completeness
   it does not have.
4. **`repair_repository` still refuses**, per item class.
5. Stage 1's classification survives: all 41 rows still hold, and the 647 tests still pass with
   assertion shapes changed where needed but no coverage lost.
6. Green three-platform CI.

## 6. Standing

- **Blocked on Level 1's merge.** Nothing begins before that.
- **Step 0 first**, reported and ruled before implementation — same as Level 1, which cost one round and
  saved a refactor.
- Volatile review results now live in `.git-exclude/reviewed/` only; binding rulings go into the RFC or
  the code. Handoffs like this one stay here.

---

## Appendix — an unrelated two-line fix, bundled only for delivery

**Not part of Level 2. Not reviewed with it. Commit separately.**

The `accepted/`→`done/` migration (`677d121`) moved DC-78, leaving two `crates/` doc comments citing a
path that no longer exists. `crates/` is not architect-writable, so they are yours:

- `crates/prikk-cli/tests/dc78_seal_provenance.rs:3`
- `crates/prikk-store/src/trust.rs:4`

Both read `rfcs/accepted/DC-78-HISTORY-EXCHANGE.md`; both should read `rfcs/done/…`.

**Do not sweep for others.** The nearby DC-72 citations in `path.rs`, `trust.rs:180` and
`refs/publication.rs:141` are correct — DC-72 did not move. Exactly these two lines are wrong.
