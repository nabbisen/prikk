# DC-74 Handoff v1 — Addendum 2: sequencing ruled

**Date:** 2026-08-08. **Authored by** the architect.
**Responds to:** `sequencing-recommendation-v1.md`.

## Ruling: accepted in full. Proceed with §3's original scope.

**Multi-parent lineage is not a prerequisite.** Your recommendation is correct, your sizing is correct,
and **my addendum-1 §3 estimate was wrong.** Review at
`.git-exclude/reviewed/DC-74-sequencing-recommendation-review-v1.md`.

I re-derived your three load-bearing claims rather than accept them, and all three hold:
`block_state.rs:13-26` rejects `BlockKind::Merge` as *"not authorized"* **before parent count is
considered**; `Merge = 3` exists in the wire format (`payload/block.rs:18`); `ParentPolicy::Dc13MergeAware`
is reserved and fail-closed (`cache_ladder.rs:31-36`).

**My "confined to derived-state machinery" framing was the error you say it was.** `verify.rs:327` and
`checkout.rs:186` do traverse all parents, but that is irrelevant while the shape gate rejects the kind
outright, and the `.first()` sites I listed are downstream of that gate rather than independent. The
estimate was offered to help you sequence and instead handed you a wrong prior to argue against. Standing
correction to my practice, recorded in the review: **no sizing estimate from me without reading the gate
that actually blocks the work.**

## Your sizing finding now has a permanent home

Rather than leave it in a handoff file, it is `rfcs/proposed/DC-75-MERGE-BLOCK-LINEAGE.md`, credited to
you and marked verified by me. It carries your central design question as its §3 — *when a block has two
parents, what does the derived state mean, and against which parent(s) is it verified?* — with your
mainline-authoritative / both-parents-verified framing intact, and your four affected tests named in §2 as
tests to be **changed deliberately with the reason recorded**, not deleted. DC-75 is **proposed**, so it
is not authorized and is nobody's current work.

## One thing to know while you build, which changes nothing you do

Under §3's scope your merge blocks will be `BlockKind::Normal`, because the shape gate authorizes nothing
else. So a merge will be **actively labelled an ordinary commit** while the format carries an unused
`Merge` kind. That is slightly worse than silence, and it is a reason the release condition must not be
quietly relaxed later — **not** a reason to change what you build now. Nothing persists pre-release.

**No action needed on it.** Do not try to work around the shape gate, and do not widen it here: that is
DC-75's scope, and opening `Merge` also touches `Repair` and `Import`, which must not be opened by
accident.

## Unchanged

Handoff v1 and addendum 1 stand except for addendum 1 §3's estimate, corrected above. Proceed to §3:
single-parent block adoption, seal, clean conflict refusal. §4's acceptance criteria are unchanged — the
two I will check hardest remain byte-identical adopted patches and a byte-exact rebuild from sealed
history through the compiled binary.
