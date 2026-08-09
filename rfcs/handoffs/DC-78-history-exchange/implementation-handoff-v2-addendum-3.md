# DC-78 Handoff v2 — Addendum 3: doc repair accepted, one more instance, sequencing ruled

**Date:** 2026-08-09. **Authored by** the architect. **Review:**
`.git-exclude/reviewed/DC-78-stage1-doc-repair-review-v1.md`.

## 1. Accepted — and your TOFU judgment is better argued than my lean was

I said the sentence was probably false and declined to rule. **You ruled it false and replaced rather
than narrowed it**, on the reasoning that TOFU *is* "trust what you first observe, refuse silent
replacement," and that the original made a flat claim about the **local** mechanism. That is the correct
reading — a narrowed "no *remote* TOFU" would have been technically defensible and quietly misleading,
because the paragraph is about the local store.

**You also found a third instance I never flagged** (`security-setup.md:17`'s Core Caveats bullet, twelve
lines above one I did). Fixing a caveat that would have contradicted the paragraph below it is right.

**And one phrase deserves credit:** *"permanently, until an operator removes it out-of-band."* I checked —
there is only `trust maintainer add`, no removal command. **"Out-of-band" is exactly accurate** where
"until removed" would have implied a command that does not exist.

## 2. A fourth instance, which my sweep found and yours did not

`docs/src/reference/repository-layout.md:177`:

> "The trust policy **currently supports one trusted MAINTAINER key** with `required = 1`."

**Still false.** I found it by grepping all of `docs/src/` for every phrasing of the claim, rather than
revisiting only the files I had flagged.

**You found one I missed; I found one you missed.** The lesson runs the same way in both directions: **a
claim repeated in four places is not repaired by fixing the instances someone pointed at.** Make a
tree-wide sweep the default whenever a factual claim about behaviour changes — I should have asked for
one in addendum-2 instead of naming two line numbers.

## 3. Sequencing — approved, with one refinement

**D3 first: agreed.** Presentation over data Stage 1 already produces, and it discharges §D3's reporting
gap, which is the part of provenance actually missing rather than merely unsurfaced.

**Refinement: ruling 4 lands *with* D4/D6, not after it.** Your reasoning holds in the abstract, but each
stage merges to `main`. **If D4 lands alone there is a window where received refs appear in
`branch list` indistinguishable from local ones** — precisely the confusion the namespace exists to
prevent. Ruling 4 is three call sites; carrying it in the same stage keeps `main` honest at every point,
not just at the end.

So: **D3 → (D4/D6 + ruling 4).**

## 4. Then merge

**Fix `repository-layout.md:177` and Stage 1 merges.** No re-review for a one-sentence correction —
report it in the next stage's package.
