# DC-75 Handoff v1 — Addendum 2: §2 ruled, one new question before §5

**Date:** 2026-08-08. **Authored by** the architect.
**Responds to:** `prerequisite-investigation-v1.md`. Full review:
`.git-exclude/reviewed/DC-75-prerequisite-investigation-review-v1.md`.

**Your investigation is accepted.** It measured instead of asserting, found a format-level blocker
neither handoff anticipated, re-ran the scenario it was asked to, and declined two forks that were not
yours. §0's O(N³) finding is now tracked in **`FINDINGS.md`** — note the register moved out of
`MILESTONES.md` today; that is where findings live now, and the owner authorized your row.

## 1. §2 ruled: **option 2 — the explicit mainline field.** Not option 1.

**The format already does exactly this.** `BlockPayload.snapshot_blob_ref` is `Option<ObjectId>` at
field tag 5, encoded **only when `Some`** (`block.rs:233`). So a new optional tag for the mainline
parent, present only on `Merge` blocks, leaves **every existing `Root`/`Normal` block's ObjectId
untouched**. Option 1 would instead change what valid canonical encoding *means* for one kind.

**A correction to your §2 sizing, which makes option 2 the smaller change, not the larger one:** it does
**not** need to touch every existing construction site "to decide what to put there for `Root`/`Normal`."
`None` is the default, exactly as `snapshot_blob_ref` already is at all of them.

Also: **keep uniqueness enforced.** Today the sort invariant is what rejects a block naming the same
parent twice; a `Merge` block must still reject that.

## 2. §1 accepted — but restate the reasoning, and this matters

**Mainline-authoritative stands.** §1.b and §1.c are the strong arguments and I endorse both.

**§1.a should not be load-bearing.** You offered cost as decisive "given §0" — but §0 is a **defect**,
not a property of the design. Choosing the weaker verification model to accommodate a fixable bug is
backwards; if `verify` becomes O(N), doubling it is cheap. **Restate the recommendation on (b), (c), and
(d), which hold regardless of cost, and demote (a) to corroboration.** I am not reversing you — had the
case rested on (a) alone, the right answer would have been to fix the cubic cost first and re-decide.

## 3. One new blocking question, before §5

**Your §3 list does not discharge DC-74's release condition, and I did not spot this when I wrote it
either.** The condition requires *"a later verifier must be able to re-check **the baseline** and both
sides."* §3 records both **sides**. **Nothing records the baseline confluence was proven against**, and
§3 item 4 says not to re-run the analysis.

Two readings, needing different designs:

- **(a) The verifier computes a merge base itself** from recorded parentage. No new field. **Trustless**
  — nothing taken on assertion — but it never reveals *which* baseline the sealer used, and your own §5
  showed the baseline choice is consequential.
- **(b) The block records the baseline.** One more optional field, same pattern as §1. History states
  what it was checked against, and a verifier can detect a merge sealed against a baseline that is not
  the true merge base. **But a recorded baseline is a claim, and a claim can be false.**

**Answer this in §4's discipline before designing §5.** My lean is **both** — record it *and* have a deep
verifier re-derive rather than trust it — but that is a lean, and my assertions in this increment have
been wrong four times. **Measure it; do not take my lean as the answer.**

## 4. Accepted without change

§4.3 (`Repair`/`Import` stay closed). §5's re-run, including the precise finding that DC-74's refusal
there is a **false positive at the operation-classification level**, correctly reported rather than
absorbed. §0's measurement.

## 5. Then proceed

Once §3 above is answered and reported, **§5 implementation is cleared** — nothing else is outstanding.
Acceptance criteria and gate discipline unchanged, verbatim per `EXECUTION-ORDER.md` §6 rule 9.
