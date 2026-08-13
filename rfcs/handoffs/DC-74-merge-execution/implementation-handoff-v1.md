# DC-74 Merge Execution - Handoff v1

**Cleared to start — on §1, not on §3.** Accepted by the project owner 2026-08-08, at
`rfcs/done/DC-74-MERGE-EXECUTION.md`. **Authored by** the architect.
**Size:** the largest increment on the roadmap. §1 may shrink it substantially, or return it to the owner.
**Touches:** `crates/prikk-store/src/` merge surfaces, the seal path, and `prikk-cli`.

## 1. Start here. Do not design anything yet.

**Four questions, answered from the code and reported before a line of design.** This pattern has now
found a wider gap than the record described in DC-65, DC-72, and DC-73 — three increments running.
Write `prerequisite-questions-v1.md` beside this file.

**Q1 — the one that can end this increment. Can a block seal a patch it did not author, with the
signature intact?**

The whole design rests on it. Read the seal path and `verify`. The question is not "is there a code path"
but: **if patch P is authored and signed by author A on branch X, can a block on branch Y seal P with
byte-identical canonical encoding, the same ObjectId, and A's original signature still validating?**

Answer it by *doing it*, not by reading types — construct it in a test and observe. If anything on the
seal path re-derives, re-encodes, or re-signs patch bytes, **stop and report.** B′ is then unavailable and
the RFC returns to the owner. Do not work around it; do not synthesize a replacement patch. That is the
squash the RFC withdrew, and it would make the merger the apparent author of someone else's work.

**Q2 — what does `merge-plan` actually emit?** Run it against a real two-branch divergence you build
yourself. Execution may turn out to be "seal what the plan already computes," which would shrink this
increment dramatically. Report the actual output, not the type signature.

**Q3 — is merge-base discovery separable?** `--baseline-block` is explicit today. If discovery can stay
manual for v1, say so and scope it out.

**Q4 — what does `patch_algebra` return on conflict, and what is reachable?** Enumerate the
`ConflictWitnessKind` variants that can actually be produced today, as opposed to defined.

## 2. What a merge is in prikk — read this before designing

**A merge authors nothing.** It is the union of two patch sets over a common context, well-defined
exactly when they commute — which DC-16's conservative subset and DC-18's confluence already decide.

This works because **prikk's operations are context-free**: every operation carries a stable nonzero
`NodeId`, and `EditText` identifies its span by `left_anchor_hash`/`right_anchor_hash` content anchors,
with `presentation_hint_line` marked in the source as *"not part of algebraic identity"*
(`crates/prikk-object/src/payload/patch/operations.rs`). An operation names *what* it edits, never
*where*. So an incoming patch needs no transformation, its bytes do not move, and **its author's
signature keeps covering it.**

That is the property the whole increment protects. **If your design ever produces new operations to
represent merged content, it is wrong.**

## 3. Scope, once §1 is reported and accepted

- Merge execution that adopts the incoming patches verbatim and seals them.
- **Single-parent blocks.** Multi-parent lineage is deferred, not rejected — `parent_block_ids` is already
  `Vec<ObjectId>` and legal, but replay fails closed on it (`patch_replay.rs:206`) and teaching it
  multi-parent traversal would reopen DC-64's cache, `rollback_preview`, and what a horizon means. Not
  here.
- Conflict refusal that leaves **no partial state**.

## 4. Acceptance criteria

`rfcs/done/DC-74-MERGE-EXECUTION.md` §5 governs. The two I will check hardest:

- **Adopted patches are byte-identical to their originals** — same ObjectIds, same author signatures.
  Assert it in a test; do not argue it in prose.
- **Rebuild byte-exact from sealed history** through the compiled binary, the DC-67 pattern. I will
  independently rebuild a two-branch merge from sealed history and diff the result.

**Conflicts:** construct a conflicting pair for each witness kind Q4 finds reachable, and assert the
repository is unchanged afterward.

**Do not report the gate set paraphrased.** `rfcs/EXECUTION-ORDER.md` §6 rule 9, verbatim, including
`--locked`, `--no-fetch`, and `cargo +1.85.0`. Report test counts before and after.

## 5. Two things that are not yours here

- **Conflict arbitration.** Detection only. A resolution is itself a signed patch, which is a trust
  question, not an ergonomics one.
- **Widening `patch_algebra`'s conservative subset.** DC-16's soundness oracle is the foundation. If you
  find the subset too narrow to merge anything useful, **report that as a finding** — it is its own
  increment, and a valuable one to have found.
