# DC-75 Handoff v1 — Addendum 5: patch-identity rule scoped INTO §5, with an escape hatch

**Date:** 2026-08-08. **Authored by** the architect.
**Responds to:** `reachability-vs-state-derivation-answer-v1.md` (`2fb326a`).
**Review:** `.git-exclude/reviewed/DC-75-reachability-answer-review-v1.md`.

**First, my error:** I issued addendum 4 without checking that addendum 3 had already been answered.
`2fb326a` predated it. You were right to say so.

## 1. Accepted — and you corrected my trace

Both code claims verified independently: `DecodedPatchOperation` (`decode.rs:29`) carries only `op_seq`
and `kind`; `analyze_merge_evidence` (`analysis.rs:25`) receives only those slices. **Your root cause is
exactly right** — no layer in that pipeline could recognise an already-adopted patch.

**Your correction to my addendum-3 trace stands.** Today's code errors the instant it *reads* `M2`
(*"requires single-parent candidate chains… has 2 parents"*), not on reaching genesis; I described
post-fix behaviour as current. Outcome right, mechanism wrong.

**And the `PairReplayFailed` demonstration is the more serious finding** — distinguishing a designed
classification from the proof engine breaking on unbuildable input is exactly the distinction that
matters, and you drew it by construction rather than argument.

## 2. Ruling: scope it into §5. Not a fourth handoff item.

**Your own §2 root cause is what decided this, against where I first leaned.**

**Patch identity is lost at *decode*, not at the walk.** `candidate_blocks` walks `BlockPayload`s and
**every block carries its `patch_ids`**. So the membership test your §2 asks for — is this operation's
source patch already reachable from the baseline by any parent path — is expressible **where identity
still exists**: as a set of patch ids reachable from the baseline, applied during candidate-set
construction. **No change to `DecodedPatchOperation`, no change to `analyze_merge_evidence`'s
signature.**

Why that makes scoping-in right:

1. **§5 is rewriting that walk anyway.** Splitting means a second increment re-touching the same
   functions for a rule belonging to the same rewrite.
2. **It stays out of `patch_algebra`.** DC-74's non-goal — widening the conservative subset is its own
   increment — is **not** triggered if the rule lives at candidate-set construction. The classifier's
   contract is untouched; it simply stops receiving input it cannot judge.
3. **Deferring would ship a `PairReplayFailed` escape**, or force a conservative reachability guard that
   over-refuses legitimate merges — a worse artifact than the rule.

## 3. The escape hatch is the operative part of this ruling

**If evaluating it shows the rule cannot live at the walk layer** — that it genuinely needs identity
inside `analyze_merge_evidence` — **stop and report.** It then splits, and §5 narrows to refusing the
repeated-merge shape cleanly instead of solving it.

**I am ruling the path I believe is cheaper, from your root-cause analysis. I have not implemented it,
and five of my assertions in this increment have not survived contact with the code.** Treat this as a
direction to evaluate first, not a design to build to. If it is wrong, saying so is the deliverable.

## 4. What §5 now owes

Everything previously ruled, plus this rule:

- Reachability walks follow **all** parents; state derivation follows **mainline only** — your §1
  catalog's assignments accepted as written, including `lineage_horizon` under state derivation.
- **Candidate-set construction excludes patches already reachable from the baseline**, per §2 above.
- **Repeated merges between the same two branches work** — trace it explicitly, as you did in §2, and
  show the `PairReplayFailed` case is gone rather than moved.
- Explicit mainline field; recorded baseline; ordinary `verify` re-derives and reports disagreement.
- `Repair`/`Import` closed. Four fail-closed tests changed with reasons recorded. DC-74's
  refusal-diagnostic assertion.
- Criterion 5: **report** the condition's technical content with evidence; **do not** touch
  `MILESTONES.md`.

## 5. Owner notified of one consequence

If §3's escape hatch fires and the rule splits, 0.19.0 would ship merge that cannot merge the same pair
twice. Whether that is acceptable is a release-scope decision and it is with the owner now, not
something for you to design around either way.
