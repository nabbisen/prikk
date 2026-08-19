# DC-53 Stage 2 follow-up — the `PBNDL002` bump broke every repository-format migration path

**Raised by the architect 2026-08-19**, while establishing the state for badge criterion 2.
**This is a live defect on `main` (`89036bf`), and it is mine as much as anyone's** — I reviewed and
accepted the bundle-format bump, ruled it "fail-closed and acceptable", and never asked who was
instructed to *produce* `PBNDL001` bundles.

## 1. The defect

`layout.rs` refuses every retired repository format with an instruction to migrate by bundle:

> *"this repository uses format N, which prikk no longer supports (this version requires format 6). to
> migrate: use a prikk version that supports format N to `prikk bundle export`, then `prikk bundle
> import` here"*

**All five retired formats say this** — grep-confirmed, five occurrences of `bundle export` in
`layout.rs`.

`bundle.rs` now refuses the bundles those older versions produce:

> *"this bundle uses format PBNDL001, which prikk no longer supports (this version requires PBNDL002).
> re-export with a current prikk build."*

**The two instructions cannot both be satisfied.** A format-1..5 repository:

1. cannot be opened by a current build — rejected at open, so
2. must be exported by an older build, which emits `PBNDL001`, which
3. a current build refuses, telling the user to re-export with a current build — **which returns to
   step 1.**

**Every prikk repository not already at format 6 is currently unmigratable**, and the only documented
path out is a loop.

## 2. Why the review did not catch it

I checked that `decode_bundle` rejects trailing bytes, concluded an additive section was impossible,
and ruled the magic bump fail-closed and therefore acceptable. **That reasoning was about the decoder in
isolation.** I never asked what else in the product tells a user to produce a `PBNDL001` bundle — and
the answer was five messages in a file I had read earlier the same day for a different reason.

**"Fail-closed" is a property of a check, not of a system.** A refusal that is locally correct can still
sever a path the product depends on, and the question that would have caught this is *who is instructed
to produce the thing I am now refusing?*

## 3. The fix

**Accept `PBNDL001` on import; keep emitting `PBNDL002` on export.**

A `PBNDL001` bundle is a `PBNDL002` bundle without the author-key section. Decoding it means parsing the
existing header and object list and treating the author-key set as **empty** — which is not a special
case, because DC-53 already defines that outcome: **no recorded key material means the imported Patches
read `Unverifiable`**, exactly as vector 7 specifies. **The vocabulary for "imported history whose
authorship this repository cannot check" already exists and is already tested.**

**Do not restore `PBNDL001` export.** The bump was right; only the refusal on the read side was wrong.
Import is where compatibility belongs, and this is the same asymmetry every format transition in this
project already has: read what the past wrote, write only the present.

**Then correct the two messages.** The `PBNDL001` refusal message disappears with the refusal. The five
`layout.rs` messages should be re-read against what a user can actually do once import accepts old
bundles — they will then be true, but check rather than assume.

## 4. What to report

- **A test that walks the real path**: a `PBNDL001` bundle imports into a current repository, its
  Patches read `Unverifiable`, and `verify` passes and says so. **Construct the old bundle by encoding
  it, not by hand-editing bytes** — a hand-built fixture proves the parser accepts a shape, not that the
  path works.
- Whether any *other* product surface instructs a user to produce something a current build refuses.
  **This defect is one instance of a class**, and the class is worth one grep: refusal messages that
  name a producer.
- The usual gate set, against the fixed commit.

## 5. Not in scope

- **No repository-format change.** Format 6 stays; this is about reading old *bundles*, not old
  repositories.
- **No new compatibility promise.** What prikk guarantees across versions is badge criterion 2's
  subject, now in progress, and it must not be settled implicitly by this fix.
