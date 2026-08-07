# RFC (proposed) - DC-74 Merge Execution

**Status.** **Proposed 2026-08-04.** Awaits owner acceptance.
**Authored by** the architect. **Independence.** Author-reviewed — the standing ceiling.
**Arises from.** The forward roadmap accepted 2026-08-04, item B, ruled next after DC-73.
**Requirement.** Product **M3** ("Block DAG and Checkout"). Merge execution is its unbuilt half.

## 1. What exists, and what does not

`merge-evidence` and `merge-plan` report what a merge *would* involve. `patch_algebra` (DC-16, DC-18)
provides commutation, confluence, and typed conflict witnesses, backed by a soundness oracle.
`IMPLEMENTATION-STATUS.md:302` states the gap plainly: **no automatic merge-base discovery, no merge
execution.**

**Two people cannot converge divergent work.** DC-66 made the two-role workflow expressible; this is what
makes a team possible at all.

## 2. The constraint that decides this increment's shape — verified, not assumed

**Every replay path in the tree is single-parent only.**

- `patch_replay.rs:206` — *"Fails closed on a multi-parent lineage (v1 single-parent only)"*
- `patch_replay.rs:4` — the manifest is built *"by walking a single-parent block chain"*
- `cache_ladder.rs:58` — provenance verification walks *"the actual walked single-parent chain"*
- DC-64's incremental step requires the new block to have **exactly one parent** equal to the cached
  baseline; `rollback_preview.rs:91` assumes the same.

**So a merge that produces a two-parent block does not "add merge" — it invalidates the replay model, the
lifecycle cache, rollback preview, and DC-64's entire trust argument simultaneously.** That is not a
scoping detail; it is the difference between one increment and a program.

## 3. The fork this creates — the owner's, not mine

**Route A — merge produces a two-parent block.** The block DAG becomes a real DAG, matching product M3's
name. Every replay path must learn multi-parent traversal, which means: what does "the" baseline mean when
there are two; how does DC-64's cache key work; what does `verify` walk; what does a horizon mean. **This
is a multi-increment program touching the identity model**, not a merge feature.

**Route B — merge produces an ordinary single-parent patch** whose content is the merged result, carrying
evidence of what it merged. Lineage stays linear. Everything built keeps working unchanged. **But history
then does not record that a merge happened as a structural fact** — only as evidence attached to a patch,
which is weaker than prikk's usual posture of making things true by construction.

**Route B is far smaller and fits everything built. Route A is what "Block DAG" promised.** They are not
orderable by technical merit, and the choice determines whether this is one increment or five.

**I recommend B for this increment and recording A as its own question** — but that recommendation
narrows what product M3 claims, so it is not mine to make alone.

## 4. What must be established before designing — blocking

| Question | Why it blocks |
|---|---|
| **Does `BlockPayload` already carry multiple parents?** Read the field, do not infer from the M3 label | If the format already permits it, Route A is a replay problem only; if not, it is also a format change |
| **What does `merge-plan` currently emit, concretely?** | Execution may be "apply what the plan already computes." Run it against a real divergence rather than reading its types |
| **Is merge-base discovery separable?** `--baseline-block` is explicit today | If discovery can stay manual for v1, the increment shrinks substantially |
| **What must happen on conflict?** Witnesses exist; execution must refuse | A merge that partially applies and then stops is the worst outcome available here |

## 5. Acceptance criteria

1. §4's four questions answered and reported **before** a design is proposed.
2. The owner's Route A/B ruling recorded in this RFC before implementation.
3. Merge execution produces a result that `verify` accepts and that **rebuilds byte-exact from sealed
   history**, tested through the compiled binary — the DC-67 pattern.
4. **Conflicts refuse cleanly, leaving no partial state**, tested against a constructed conflicting pair
   for each `ConflictWitnessKind` reachable today.
5. DC-64's cache, `rollback-preview`, and `verify` continue to work against post-merge history — tested,
   not argued.
6. If Route B: the evidence recording *what was merged* is specified, and its weakness relative to
   structural parentage is stated in the docs rather than left implicit.
7. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after.

## 6. Non-goals

- **Conflict arbitration.** Detection exists; resolution is a separate theme, and a resolution is itself a
  signed patch — a trust question, not an ergonomics one.
- **Patch aggregation.** Separate theme, and not in the requirements at all.
- Merge-base *discovery*, if §4 shows it separable.
- Changing `patch_algebra`'s conservative subset. DC-16's soundness oracle is the foundation; widening what
  is provably safe is its own increment.
