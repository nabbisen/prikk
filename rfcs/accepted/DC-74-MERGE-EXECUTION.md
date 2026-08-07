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

## 3. What patch theory says — asked by the owner, answered from the code

The owner asked which route fits patch theory better. Answering it required reading the operation
grammar rather than reasoning from milestone labels, and **it overturns §3 as first written.**

### 3.1 prikk's patches are context-free, by construction

`crates/prikk-object/src/payload/patch/operations.rs`: every operation carries a stable nonzero
`NodeId`. `EditText` identifies its span by **content anchors** — `left_anchor_hash`,
`right_anchor_hash`, 32 bytes each — and carries `presentation_hint_line` marked, in the source,
**"not part of algebraic identity."**

That is the Pijul-shaped design, not the Darcs-shaped one. An operation names *what* it edits by stable
identity, never *where* by position. So a patch from a divergent branch does not need to be transformed
to apply here: it commutes as-is.

**This is the decisive property, because signatures are bytes.** In a context-dependent model
(Darcs-style) merging *transforms* the incoming patch, its canonical bytes change, its ObjectId moves,
and the original AUTHOR signature no longer covers it — so whoever merges must re-sign content they did
not write. That is DC-35's "automation cannot occupy an accountable approval identity," arriving at the
patch layer. **prikk's design avoids this entirely**: transported patches are bit-identical, so author
signatures survive a merge untouched.

`PatchPayload` confirms it — a patch binds to `parent_patch_ids` and explicit `preconditions`, **not to
a baseline tree**. Its context is a dependency set, not a snapshot.

### 3.2 Therefore a merge authors nothing — and Route B as first written was wrong

In patch theory a merge is the **pushout**: the union of two patch sets over a common context,
well-defined exactly when they commute — which is what DC-16's conservative subset and DC-18's
confluence already decide. Darcs and Pijul, the two production patch-theoretic systems, both have **no
merge commits at all**, for this reason: there is no new content for one to hold.

§3's original Route B — "a patch whose content is the merged result" — **synthesizes new operations**,
which discards the incoming patches' ObjectIds and their author signatures and makes the merger the
apparent author of someone else's work. **That is a squash**, and squashing is the specific thing patch
theory exists to make unnecessary. I withdraw it.

**Route B′ — adoption.** The merge seals the *other side's patches verbatim*: same bytes, same ObjectIds,
same AUTHOR signatures. The maintainer seals; nobody re-authors. This is the mathematically canonical
merge and it is the one prikk's operation grammar was built for.

### 3.3 The fork is much smaller than §3 first claimed — and it is not an algebra question

**`BlockPayload.parent_block_ids` is already `Vec<ObjectId>`** (`payload/block.rs:50`), sorted and
unique, with a source comment anticipating *"a later design adds semantic parent roles."* **Multi-parent
blocks are already legal in the wire format.** §4's first question is answered: Route A was never a
format change. Only replay refuses it — `patch_replay.rs:206` — and that refusal is a v1 implementation
choice, not a constraint of the model.

And under B′ the merge is **structurally recorded either way**: the adopted patches carry
`parent_patch_ids` into the other branch's history, so the merge cannot be verified without reaching
it. **The patch DAG records the merge whether or not the block does.**

So what remains is a narrow bookkeeping question at the *authority* layer, where patch theory has no
opinion: should the sealed block also name both sealed ancestors? It is true, the format already says
it, and it costs multi-parent replay.

**Recommendation, now with a reason rather than a preference:** adopt **B′** for the merge semantics —
this is not a route choice but a correctness requirement, since synthesis breaks authorship. Then take
**single-parent blocks for this increment** and open multi-parent block lineage as its own increment,
because it buys bookkeeping that the patch DAG already provides and costs the replay, cache, and
rollback rework §2 enumerates.

## 4. What must be established before designing — blocking

| Question | Status |
|---|---|
| ~~Does `BlockPayload` carry multiple parents?~~ | **Answered above: yes, already `Vec<ObjectId>`** |
| **Can a block adopt a patch it did not author, with signatures intact?** Read the seal and `verify` paths | **B′ depends entirely on this.** If sealing re-derives or re-signs patch bytes, B′ is unavailable and the fork reopens |
| **What does `merge-plan` emit, concretely?** Run it against a real divergence | Execution may be "seal what the plan already computes" |
| **Is merge-base discovery separable?** `--baseline-block` is explicit today | If discovery stays manual for v1, the increment shrinks |
| **What must happen on conflict?** | A merge that partially applies and then stops is the worst outcome available |

## 5. Acceptance criteria

1. §4's four questions answered and reported **before** a design is proposed.
2. The owner's Route A/B ruling recorded in this RFC before implementation.
3. Adopted patches are **byte-identical** to their originals — same ObjectIds, same author signatures — asserted by test, not by argument.
4. Merge execution produces a result that `verify` accepts and that **rebuilds byte-exact from sealed
   history**, tested through the compiled binary — the DC-67 pattern.
5. **Conflicts refuse cleanly, leaving no partial state**, tested against a constructed conflicting pair
   for each `ConflictWitnessKind` reachable today.
6. DC-64's cache, `rollback-preview`, and `verify` continue to work against post-merge history — tested,
   not argued.
7. If single-parent blocks are taken, the docs state that merge provenance is carried by the patch DAG
   rather than by block parentage, rather than leaving it implicit.
8. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after.

## 6. Non-goals

- **Conflict arbitration.** Detection exists; resolution is a separate theme, and a resolution is itself a
  signed patch — a trust question, not an ergonomics one.
- **Patch aggregation.** Separate theme, and not in the requirements at all.
- Merge-base *discovery*, if §4 shows it separable.
- Changing `patch_algebra`'s conservative subset. DC-16's soundness oracle is the foundation; widening what
  is provably safe is its own increment.
