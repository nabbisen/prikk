# DC-75 Merge Block Lineage — Handoff v1

**Cleared to start on §1 only, and §1 may run now, in parallel with DC-74.** Accepted by the project
owner 2026-08-08, at `rfcs/accepted/DC-75-MERGE-BLOCK-LINEAGE.md`. **Authored by** the architect.
**Origin: your own sizing finding** in `../DC-74-merge-execution/sequencing-recommendation-v1.md`. The
RFC's §2 and §3 are your analysis, verified and adopted. **Touches:** `block_state.rs`, both
`single_parent_chain` functions, `merge_evidence.rs`, `cache_ladder.rs`, `verify`.

## 1. Sequencing — read before anything

- **§4's prerequisite investigation: start whenever you like, including now.** It is read-only —
  measurement and design analysis, no production code — so it cannot collide with DC-74.
- **§5 implementation: wait for DC-74 to merge.** Both increments touch the seal path and
  `block_state.rs`. Two concurrent edits there make both harder to review and neither easier to write.

If DC-74 turns out to depend on something §4 discovers, **say so and stop** rather than reconciling the
two yourself.

## 2. What this increment is for

It **discharges DC-74's release condition** — merge execution does not ship until sealed history
structurally records a merge. **Neither DC-74 nor this ships without it.** Criterion 5 requires the same
commit that satisfies the condition to discharge it explicitly in `MILESTONES.md`.

The thing being protected is narrow and worth stating once: a merge under DC-74's adoption model is sound
only if both sides were confluent from a common baseline. **If sealed history does not record what that
baseline was, no later party can re-check the merge** — they can only observe that a maintainer sealed
some patches. That is the gap.

## 3. Answer §3's question first, and answer it with measurements

> When a block has two parents, what does "the state derived from this block" mean, and against which
> parent(s) is it cryptographically verified?

Your two candidates — **mainline-authoritative** (one parent walked and verified, the other retained as
evidence) and **both-parents-verified** — are framed correctly in the RFC. What §4.1 needs is not an
argument between them but **the cost of the stronger one, measured**: what both-parents-verified does to
DC-64's incremental cache, where the price actually lands.

Take the measurement before forming a view. This project's estimates have gone wrong in both directions
this week, including two of mine in DC-74, and the difference every time was whether someone had read or
run the thing before describing it.

**Note the asymmetry when you weigh them.** Mainline-authoritative *names* the second parent without
verifying through it — so the merge record exists but the record's own soundness rests on a path nobody
checks. Whether that satisfies §4.2 is the crux of this increment, and I do not have a settled view.

## 4. Two constraints on the work itself

**Do not open `Repair` or `Import` by accident.** `validate_block_v2_shape` rejects
`Merge | Repair | Import` in one arm. Opening `Merge` must be deliberate and must leave the other two
closed unless §4.3 says otherwise, with the reason recorded.

**The four tests in §2 change, they do not get deleted** — `merge_lineage_fails_closed`, the two
`ParentPolicy::Dc13MergeAware` rejection tests, `multi_parent_candidate_fails_before_report`,
`format2_parent_and_kind_matrix_is_closed`. Each records a deliberate fail-closed decision. Changing one
means replacing that decision with a different one, and criterion 4 requires the reason in the commit.

## 5. What I will check hardest

Criterion 2: **a merge sealed by DC-74's execution path is recorded as `BlockKind::Merge` naming both
parents, and `verify` re-derives its soundness from sealed history alone** — tested through the compiled
binary. I will construct a two-branch merge, seal it, delete everything derived, and check that a party
holding only the sealed objects can establish the merge was sound.

Gate set: `EXECUTION-ORDER.md` §6 rule 9, **verbatim**, including `--locked`, `--no-fetch`, and
`cargo +1.85.0`. Test counts before and after.

## 6. Non-goals

Merge execution (DC-74 owns it). Conflict arbitration and patch aggregation. Populating
`parent_patch_ids` — if §3 shows a patch DAG is the better vehicle, **report it as a finding**; do not
absorb it.
