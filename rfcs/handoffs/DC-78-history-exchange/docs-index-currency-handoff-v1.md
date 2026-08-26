# Docs front page — three false or dead claims in fifteen lines

**Base:** current `main` (`09356c7`). **Under `003-landing-work-on-main.md`.**
**Origin:** reported while adding the install page, correctly not fixed there.

`docs/src/index.md` is the documentation site's front door. **It is fifteen lines long and I found
three problems in it.**

---

## 1. "Intentionally short" is false

> This documentation is intentionally short in the early implementation phase and will grow as
> FDD-approved implementation areas land.

**Counted:** `SUMMARY.md` lists **22 guide pages** and **12 reference pages**; `docs/src` holds
**37 `.md` files**. It is not short, and the sentence's forward-looking framing — *"will grow"* — is
backwards. **It already grew.**

**Do not simply delete the sentence.** The paragraph is the only thing on the page that tells a reader
what state the documentation is in. **Replace it with something true**, or say nothing about
completeness at all — but do not leave a promise about the future where a description of the present
belongs.

## 2. "FDD-approved" is dead jargon on the front page

`grep -rln "FDD-approved" docs/` returns **`docs/src/index.md` and nothing else** (the `docs/book/`
hits are build output of that same line).

**A term used exactly once in the entire documentation, on its front page, undefined, is worse than no
term.** A reader has no way to find out what it means.

**Remove it.** If the concept matters to a documentation reader, it needs a definition somewhere — and
if it does not, the front page is the last place it belongs.

## 3. The two front doors describe prikk differently

- `docs/src/index.md`: *"Prikk is a design-first experimental VCS."*
- `README.md:26`: *"Prikk is a standalone distributed version control system built around
  block-oriented patch theory."*

**The docs front page never says what prikk actually does.** No mention of distributed, of version
control beyond the acronym, or of patch theory.

**Align the description of what prikk is with the README's own sentence** — the same technique used
for the crate descriptions, and for the same reason: the project's own claim about itself cannot
overclaim relative to itself.

**One hard constraint.** **Do not drop, soften, or strengthen "experimental."** That word is a posture
claim tied to the early-implementation badge, which is **the owner's decision, not ours.** Keep it,
and change only the description of what the tool *is*. **If you believe the two cannot both be kept in
one sentence, stop and say so rather than choosing.**

## 4. Out of scope

- **`README.md`.** It is the source here, not the target.
- **The early-implementation badge**, and any claim about project maturity beyond §3's constraint.
- **The name note and the architecture links.** Both are fine.
- **Any other docs page.** If you find the same staleness elsewhere, **report it, do not fix it.**

## 5. Controls

1. **No sentence duplicates the README verbatim** — §3 asks you to align a claim, not copy a
   paragraph. Show it mechanically, as the crate-README increment did.
2. **`mdbook build` clean**, and the page still renders with its links intact.
3. **"FDD-approved" appears nowhere in `docs/src`** afterwards — show it.
4. **Full gate set green**, and the test count **must not move** — this is docs-only.

**If the count moves, something other than documentation changed. Stop and say so.**

## 6. What to report

1. **The page, before and after, in full** — it is fifteen lines; quote both.
2. **Your page counts**, independently derived (I counted 22 guide, 12 reference, 37 files — **check
   me**).
3. **How you kept "experimental" while fixing §3**, or why you could not.
4. All four controls (§5), quoted.
5. **Full gate set against the exact commit, after the last edit**, including `mdbook build`.
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here was wrong.

**Stop and escalate, do not guess**, if: fixing §3 requires a judgment about project maturity (§3's
constraint); or you find that "design-first" is load-bearing terminology elsewhere in the project that
the front page is correctly reflecting — **that would make §3 wrong, and I would rather know.**
