# Recovery reference pages — format-1 and `0.18.0` currency: implementation handoff

**Base:** current `main` (`828e631`). **Under `003-landing-work-on-main.md`.**
**Origin:** reported independently by **both** of the previous two increments, and confirmed by me from a
third angle.

**Three files**, all published: `docs/src/reference/concurrency-locking.md`,
`docs/src/reference/durability-recovery.md`, `docs/src/reference/integrity-recovery.md`.

**This is not the version-string swap the earlier reports scoped it as.** Investigating it to write this
handoff turned up something larger: **these pages document recovery procedures for repositories that
every command now refuses to open.**

---

## 1. The finding that sets the scope

`durability-recovery.md:146-150`:

> *"**For released format-1 repositories**, one exact already-signed log-ahead transition **may be
> completed by signer-backed seal**... A missing format-1 pointer with log history is diagnosed but is
> not reconstructed by doctor **in 0.18.0**; preserve the repository and restore from backup or **retain
> it for later migration/recovery tooling**."*

**Three defects in five lines:**

1. **The procedure cannot be performed.** `require_current_format` returns
   `Err(UnsupportedFormatVersion(1))` for any layout that is not format 6, and **every command opens
   through it.** A format-1 repository is refused before any of this is reachable. **RFC 114 ruled
   formats 1-5 out of scope**, and criterion 2's row records that their refusals *"say so rather than
   offering a migration the product cannot honour."* **That ruling never reached these pages.**
2. **`in 0.18.0`** — five releases stale, and phrased as if a later release might differ.
3. **`retain it for later migration/recovery tooling`** — **no such tooling exists or is planned.** Same
   defect as the `0.23.0` changelog's original tag advice, and as the `doctor` refusal message just
   corrected in `6a3d591`: **telling a user to do something that does not exist.**

## 2. Scope: adjudicate each site, do not sweep on the string

`grep "format-1\|0\.18\.0"` over the three files returns **14 hits**, and **they are not all the same
thing.** At least two distinct senses are in play:

- **The repository format** — `format-1 repositories`, `format-1 pointer`. Governed by RFC 114's
  formats-1-5 ruling. **This is where the defects are.**
- **Envelope/signature shapes** — e.g. `integrity-recovery.md:37`'s *"format-1 signature-envelope
  diagnosis for malformed Ed25519..."*, `:93`'s *"format-1 log lead"*, `:209`'s *"byte-preserving
  format-1..."*. **These may describe live behaviour and be perfectly current.**

**Adjudicate every one of the 14 against the code. Do not fix on the string.** A sweep that rewrites
`format-1` everywhere would corrupt accurate text — the opposite defect, and harder to detect.

**Confirmed defective already** (verify, do not inherit): `concurrency-locking.md:155,276`;
`durability-recovery.md:149,198`; `integrity-recovery.md:156,200`. **`durability-recovery.md:146-148`
is the passage in §1** and is the most important.

**`release-compatibility.md`'s `0.18.0` mentions are out of scope** — genuine historical references to
that release, correctly excluded by the earlier report. **Do not touch that file.**

## 3. The question to answer, not to assume

**What should a page say about a repository the tool refuses to open?**

There are two honest shapes and I am not ruling between them:

- **Delete the procedures** — they cannot be run; documenting them is documenting nothing.
- **Keep them, explicitly marked historical** — *"format-1 repositories are refused by every command
  since RFC 114; the following describes 0.18.0-era behaviour and is retained for historical
  reference."*

**Pick one, apply it consistently across all three pages, and say in your report which you chose and
why.** What is **not** acceptable is the current state: a procedure written in the present tense for
something that cannot happen.

**Whichever you choose, the "later migration/recovery tooling" promise goes.** Nothing licenses it.

## 4. Out of scope

- **Every file except those three.**
- **`data-model.md` / `data-model-lifecycle.md`** — just corrected in `828e631`; **report** any
  contradiction rather than editing.
- **No code.** If a doc turns out to be right and the code wrong, **report it** — do not fix.
- **`MILESTONES.md`, `ROADMAP.md`, the badge.** Mine or the owner's.

## 5. What to report

1. **All 14 sites adjudicated** — each with a verdict (`DEFECTIVE` / `CURRENT`) and the authority. **The
   `CURRENT` ones are as much of the deliverable as the fixes**; that is what distinguishes this from a
   string sweep.
2. **Your §3 choice**, and why.
3. **Every "remedy" sentence you removed**, quoted, so the record shows what was being promised.
4. Any contradiction found in the two just-corrected pages (§4).
5. The **full gate set against the exact commit, after the last edit**, plus `mdbook build`.
6. Test counts — **expected unchanged**.
7. Anything here that was wrong. **My §1 reading is mine, not verified fact you may inherit** —
   in particular **re-derive that `require_current_format` really makes the §1 procedure unreachable**,
   the same way `6a3d591` re-derived the `doctor` ordering rather than trusting my handoff. It found my
   reading correct that time; it may not this time.

**Stop and escalate, do not guess**, if: a site's correct wording depends on whether format-1 support
might return (it will not, but that is the owner's to say, not mine); the two §3 shapes both look wrong
for some page; or you find a **fourth** page carrying the same framing.
