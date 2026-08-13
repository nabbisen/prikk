# DC-95 Stage 2, Level 1 — Step 0 Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-95-stage2-level1-step0-v1.md`.

**Accepted. My dependency graph was wrong in three places and silent on a fourth thing entirely.** Step 0
did exactly what it was for. Implementation is cleared with the corrected graph, plus one ruling that
generalises their sharpest finding beyond the instance they found it in.

## 1. Verified

- **`require_retained_evidence` takes four inputs** (`verify/ref_publication.rs:12-18`): `records`
  (stage 4), `metadata` (stage 8), `trust_is_valid` (stage 1's shared state), `issues` (stage 2).
  **My §4 listed one dependency.** Confirmed.
- **`trust_is_valid` is `trust_verifier.issues.is_empty()`** at the call site (`verify.rs:414`).
  Confirmed.
- **`classify_active_wal_metadata(layout, replay.records.is_empty())`** (`verify.rs:407-408`) — stage 8
  depends on stage 4, which my §4's WAL list omitted. Confirmed.

**Design §4 is superseded by their §1 table.** Not amended in part — replaced.

## 2. The `trust_is_valid` finding, and why I am rating it higher than they did

They classified it as a diagnostic-accuracy risk rather than a blocking-correctness one, and that
analysis is right: `mark_unproved` rewrites code and message but never the `blocking` flag, so no bad
repository passes. **I am not disputing the mechanism. I am disputing the severity.**

Under stage containment, if stage 1 fails before `trust_verifier.verify()` is ever called,
`trust_is_valid` reads `true` — **not because trust was checked and found clean, but because it was never
checked.** Stage 9 then reclassifies on that basis, and `doctor.rs:238-253` advises *"signer-backed seal
retry"* rather than *"preserve for manual recovery."*

**On a repository whose trust state is unknown, that points the operator at an action instead of at
preservation.** Preservation is the correct default when state is unknown, and this is prikk — the
product whose entire claim is that it does not assert what it has not verified. **A verifier that reports
"proved" from an unrun check is making exactly the class of claim this increment exists to eliminate**,
even when the blocking bit happens to be right.

**Ruled: this is a correctness defect in my design, not a polish item.** It must be fixed in Level 1, and
it must have a test.

## 3. The generalisation, which is the actual ruling

`trust_is_valid` is one instance of a pattern that stage containment creates wherever it is applied:

> **The emptiness of an accumulator means "none found" only if its producer ran to completion.**
> Under containment, every `is_empty()` / `is_none()` / count-based inference drawn across a stage
> boundary becomes three-valued: **none / some / unknown.**

Today that pattern is safe because a failed producer aborts the run. Containment removes that guarantee
everywhere at once, and `trust_is_valid` is simply the instance visible from the pipeline body.

**Ruled, and this is an implementation obligation:**

1. **Sweep for the pattern.** Every cross-stage inference drawn from an accumulator's emptiness or a
   count must be identified and given an explicit unknown case. Report the full set found — this is a
   Step 0 deliverable extension, not a code-review afterthought.
2. **`trust_is_valid` becomes `issues.is_empty() && stage_1_evaluated`**, as they recommend.
3. **A test per instance**: with the producing stage `Failed`, the consumer does not read the
   accumulator's emptiness as a negative result.

**This is the upstream-gate rule from Stage 1, arriving from the other direction.** There it was *a
check's code being present does not prove a defect reaches it*. Here it is *an accumulator being empty
does not prove nothing was found*. Same underlying error: **inferring a result from the absence of
evidence when the evidence-gathering step may not have run.** It belongs in `verify.rs`'s module doc
alongside its sibling.

## 4. §12.1 answered, and their answer is better than the one I withheld

**`stage_status` alongside the existing per-finding `Vec`s, not folded into them.** Accepted, and their
reasoning is the same argument my own §2.5 made in the opposite direction: folding conflates *"this
defect is blocking"* with *"this scope could not be checked."* Those are different severity ranges and a
single `blocking: bool` expresses neither well when merged.

The existing `Vec` fields stay **exactly as they are today.** That is a smaller Level 1 than I had left
open, and it is the right size.

## 5. Partial counts: accepted, with the reason stated more strongly

Keeping whatever partial count `verify_objects` reached before failing is right, **and the condition they
attach to it is what makes it safe**: the stage's own status must be visibly `Failed` alongside the
count. A partial count without a visible status is precisely the clean-looking incomplete report §3 of
the design forbids.

**The report reader consults the stage-status map first; counts are detail.** Make that ordering explicit
in the output, not merely true of the data structure.

## 6. What Step 0 did not cover, correctly

Level 2's item boundary and the `--stop-on-first-error` surface were declared out of scope with no
opinion formed. **Right** — both are Level 1 or Level 2 design surface, and inventing an opinion during a
dependency trace is how scope creeps.

## 7. Standing

- **Step 0: accepted.** No stop-and-report; scope containment stands.
- **Design §4 is replaced by Step 0's §1 table**; §12.1 is answered by §6.1 of their report. I will amend
  the design document to say so rather than leave the two disagreeing.
- **Level 1 implementation is cleared**, with §2's fix and §3's sweep as added obligations.
- **§3's sweep is reported before the implementation is submitted**, not alongside it — if it turns up
  more instances than `trust_is_valid`, that changes Level 1's size and I want to know before the code
  exists.
- Green three-platform CI before any merge.

## 8. On the pattern

**Four consecutive prerequisite investigations have now corrected a document I wrote** — RFC 101's
problem statement, RFC 102's §3, DC-95 §2's shape count, and now this graph. I predicted this one
explicitly and named the two claims most likely to be wrong; **both were wrong, and a third error I did
not anticipate was larger than either.**

The mechanism works. The conclusion I draw is not that the reviews should be gentler but that **my
framings should be written as hypotheses by default** — which is what §4 was labelled as, and why this
cost one round instead of a refactor.
