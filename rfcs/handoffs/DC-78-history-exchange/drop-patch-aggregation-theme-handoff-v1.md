# Drop the patch aggregation theme from `ROADMAP.md`

**Base:** current `main` (`aaca89c`). **Under `003-landing-work-on-main.md`.**
**Owner-authorized 2026-08-24.** **Documentation only. One deletion.**

---

## 1. What to do

**Delete `ROADMAP.md:141-166`** — the section headed *"### Patch aggregation — an original concept that
is NOT in the requirements"*, through to (not including) `### Structured output for tooling` at `:167`.

**No banner, no retirement note, no replacement.** This is a deletion, not a retirement — see §3.

## 2. Why — the ruling, so the deletion is not mysterious later

The theme has **two possible readings, and neither leaves a user better off:**

- **A unit of history coarser than a patch — already delivered.** Sealing N patches produces one
  immutable Block, and **the Block is what a user already sees**: `prikk log` iterates blocks, printing
  `block {id}`, kind, ref-state and update-seq (`crates/prikk-cli/src/output/worktree.rs:55-59`). There is
  nothing to build.
- **Squashing patches into one patch — forbidden by design.** prikk has no amend, no rebase, no
  force-push. **Squashing is rewriting.** Most of its value elsewhere comes from tidying messy history
  after the fact, which is the specific thing this project promises is impossible.

**So: exists already, or must never exist.** The theme's own text already recorded that it *"appears
nowhere in `specs/`"* — an original concept that never reached the requirements.

## 3. Why no banner, when `IMPLEMENTATION-STATUS.md` got one

**A banner preserves a record someone might need.** `IMPLEMENTATION-STATUS.md` kept one because its
history of which increments were accepted when is real and unreproducible.

**Here the record is the reasoning, and the reasoning is already recorded** — in this handoff, in
`.git-exclude/reviewed/cluster-a-dag-or-chain-investigation-v1.md`, and in git history. **A banner whose
only content is "we once considered this" is what git history is for.**

**And keeping it would repeat the failure being corrected.** The theme survived twenty days *because* it
was recorded rather than resolved: **"recorded, not rejected" reads as diligence and functions as
deferral.** The same shape kept `parent_patch_ids` alive as a field that might mean something, and keeps
`ProjectGenesis` alive as a type code reserved for a decision never made. **Do not leave a fourth.**

## 4. Do not touch the inbound references

Three records cite the theme as an out-of-scope non-goal:

- `rfcs/done/DC-74-MERGE-EXECUTION.md:148`
- `rfcs/done/DC-75-MERGE-BLOCK-LINEAGE.md:101`
- `rfcs/done/DC-78-HISTORY-EXCHANGE.md:277`

**Leave all three.** They are historical records, and **their claim — that it was out of scope for that
increment — remains true.** This is the `f69779c` precedent: a citation to retired work is fine when the
cited claim still holds. **Do not add "(theme dropped)" annotations to done RFCs.**

**Two of my own handoffs also name it as out-of-scope.** Same treatment — issued handoffs are records.

## 5. What to report

1. **Confirmation of the exact line range deleted**, and that `### Structured output for tooling`
   survives intact immediately after.
2. **Any inbound reference beyond the five I listed**, and what you did.
3. **Anything that turns out to *depend* on the theme** rather than merely mention it — §6.
4. **Full gate set against the exact commit, after the last edit.**
5. Test counts — **expected unchanged**.
6. Anything here that was wrong, **including my line numbers and my reading of `prikk log`**.

**Stop and escalate, do not guess**, if: something depends on the theme rather than mentioning it; the
deletion would take neighbouring content with it; or **`prikk log` turns out not to present blocks the
way §2 claims** — that would undercut the ruling's first half, and it is the finding I would most want to
hear.
