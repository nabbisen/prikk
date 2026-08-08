# DC-75 Handoff v1 — Addendum 4: criterion 5 ruled and amended

**Date:** 2026-08-08. **Authored by** the architect.
**Responds to:** `.git-exclude/review-request/prikk-dc-75-milestones-authority-question-v1.md`.
**Ruling:** `.git-exclude/reviewed/DC-75-milestones-authority-question-ruling-v1.md`.

## 1. Reading B governs — you do not write that line

**The conflict was real and it was mine.** Criterion 5 predates `MILESTONES.md`'s Authority section; when
I wrote that section I did not check which existing criteria it collided with. Same failure mode as the
item-6 ruling.

**But not for the reason you found.** "The developer does not edit it at all" is a *file-authorship*
rule, and that is the weaker argument — it would evaporate the next time documents are reorganized, which
is exactly what happened today.

**The durable reason: discharging a release condition is an attestation about release readiness, and it is
the object of implementation review rather than an output of implementation.** Whether §5 actually makes
sealed history structurally record a merge is precisely what I test at review. A commit that both
implements it and declares the condition satisfied self-certifies the claim under examination. That holds
whatever file the condition sits in.

## 2. What this means concretely for your delivery

- **You** implement §5 and **report** in the delivery that the condition's technical content is met —
  sealed history structurally records a merge, and a verifier re-derives its soundness — **with
  evidence**. Same shape as every other criterion you report.
- **I** verify at review and write the `MILESTONES.md` line.
- **Discharge means the condition no longer blocks a release**, not that one is authorized. Activation
  stays the owner's three-authority commit.

**Criterion 5 is amended in place** (`rfcs/accepted/DC-75-MERGE-BLOCK-LINEAGE.md`) to say this, with the
amendment and its reason in the text rather than silently corrected. **Nothing in your commit should
touch `MILESTONES.md`.**

## 3. Your routing was right

You took a genuine open question to `.git-exclude/review-request/` instead of guessing, folding it into an
implementation report, or leaving it in conversation — and raised it **before** reaching the affected
step, with the cost of guessing wrong stated. That is the standing rule working as intended, and it is the
second time this session that asking instead of inferring has caught an error of mine.

## 4. Nothing else changes

Addendum 3's block still stands and is the only thing gating §5: **separate reachability from state
derivation**, and report the three items in its §3 before designing — which functions follow all parents
versus mainline only, whether `candidate_sequence`'s left-side operation set is still well defined over a
DAG, and an explicit trace showing repeated merges between the same pair work.

Everything else stands: the explicit mainline field, mainline-authoritative state derivation, record the
baseline *and* re-derive it in ordinary `verify`, `Repair`/`Import` closed, the four fail-closed tests
changed with reasons recorded, and the DC-74 refusal-diagnostic assertion.
