# DC-78 Handoff v2 — Addendum 2: Stage 1 accepted in substance, one blocking doc repair

**Date:** 2026-08-09. **Authored by** the architect. **Review:**
`.git-exclude/reviewed/DC-78-stage1-implementation-review-v1.md`.

## 1. Staging confirmed — and the rule prescribes it, it does not merely permit it

`EXECUTION-ORDER.md` §6 rule 2, verbatim: *"One increment per candidate. No bundling. **Multi-stage
increments land one stage per review.**"*

**You read it right.** Landing D2–D6 in one commit would have been the bundling that rule forbids.
Stating the boundary up front, with what is deferred and why, is the shape the rule asks for. **Continue
staging.**

## 2. Both negative controls discriminate precisely — I ran my own versions

Disabling the TOFU refusal fails **only** the refusal test. Degrading the policy to single-key fails
**only** the second-key test. **Each pins exactly its own guarantee and nothing else** — materially
better than DC-74's refusal tests, where four of five survived removing the gate they existed to pin.

Your note that a first draft asserted `!stdout.contains("issue")` and false-positived against the
report's own `"…issues: 0"` labels is worth recording: **you caught your own vacuous assertion by running
it.** 901 tests, zero failures; clippy clean on all three targets; all policy gates valid.

## 3. §4's deferral is confirmed — and safe for a reason you did not give

Designing the TOFU record's block-id/ref-name shape against no producer would be guessing. Agreed.

**And it is specifically safe because the trust store is repository-local and not content-addressed.**
No object id, state root, or signature preimage depends on its shape, so extending
`AdoptedMaintainerKey` later invalidates no sealed history. **Had it been part of object identity,
deferring would have been the wrong call** — worth knowing as the test for future deferrals of this kind.

## 4. BLOCKING — two documentation paragraphs are now false

Your §7 weighed the **TOFU** sentence, and that reasoning is defensible. **But the same two paragraphs
make two further claims this stage just falsified:**

- **`docs/src/guide/security-setup.md:66-68`** — *"a fixed-shape policy **equivalent to one trusted
  MAINTAINER key**… **Broader policy shapes are rejected**."*
- **`docs/src/reference/trust-threat-model.md:59-61`** — *"supports **a single repository-local trusted
  MAINTAINER key**… The parser **deliberately rejects broader policy shapes**."*

**Both are now wrong**, and your own
`policy_accepts_two_keys_and_rejects_malformed_two_key_syntax` proves it. A reader is told the parser
rejects what it now accepts — **about a security control.**

**Correct the shape claims in both.** Small repair, not a re-review.

**The TOFU sentence is yours to judge.** I lean toward it already being false — refusing a changed key
for a known key id *is* trust-on-first-use enforcement — but your scoping to "no *remote* TOFU yet" is
arguable and **I am not ruling it.** Say which way you land and why.

## 5. Then merge, and sequence the next stage yourself

Nothing else outstanding. Next stage is yours to order among D3's provenance reporting, D4/D6's import
and bundle, and ruling 4's namespace awareness.

**One request on mechanics:** keep committing to the branch. It worked — and when my documentation
commit landed on your branch by mistake, **asking rather than acting was right twice over**, because I
was mid-rebase of that same branch when the owner stopped me. Two people fixing it at once would have
been worse than the mistake.
