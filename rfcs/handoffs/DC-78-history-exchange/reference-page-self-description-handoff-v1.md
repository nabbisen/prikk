# Reference pages — stale self-descriptions and self-narration: implementation handoff

**Base:** current `main` (`8516df8`, CI + Docs green). **Under `003-landing-work-on-main.md`.**
**Origin:** owner asked what remained on docs, 2026-08-24; found by scanning all 11 reference pages.

**Scope: all 11 pages in `docs/src/reference/`.** Three carry a confirmed defect; the other eight are to
be **adjudicated**, not assumed clean.

---

## 1. The trap — read this before touching anything

Two of the three defects are a page **pinning itself to an old version**:

- `patch-algebra.md:4` — *"describes the current implementation **through 0.17.1**"* — six releases back
- `path-safety.md:4` — *"describes the current implementation **through 0.17.6**"* — six releases back

**Deleting the pin is not the fix on its own.** *"Describes the current implementation through 0.17.1"*
is a **narrow, honest claim**. *"Describes the current implementation"* is a **much wider one.** Striking
six words silently upgrades what the page asserts about itself — **from "current as of six releases ago"
to "current now"** — and if the body has drifted since 0.17.1, that makes the page **more** wrong, not
less, while looking tidier.

**So the pin comes off only after the body is checked.** If you cannot verify a page's content against
the code, **say so and leave the pin** — a stale-but-honest claim beats a fresh-but-false one. **This is
the single most important instruction here.**

## 2. The three confirmed defects

### 2.1 / 2.2 — the version pins above

**The target shape already exists in this repo.** `durability-recovery.md:4` and
`repository-layout.md:4` both say *"describes the current implementation"* with **no version at all**.
That is the pattern: a reference page is current by definition or it is broken, and a version pin just
becomes a second thing to keep in sync. **Match those two pages** — subject to §1.

### 2.3 — `trust-threat-model.md` narrates its own edit history

`trust-threat-model.md:6`:

> *"**Refreshed 2026-08-18 after DC-53 completed**; before that refresh this page still described
> 0.16.0..."*

**Identical to the defect removed from `data-model.md:48` at `8516df8`** — a reference reporting on its
own edits. A reader learns something *was once* wrong, about a claim they cannot see, with no way to
tell whether the note still applies.

**Its line 4 is a different case, and better than the other two.** It says *"the implementation on
`main` as of 2026-08-18 (released through 0.22.1)"* — **dated rather than implying timelessness**, which
is honest. It is now stale (`0.23.0` shipped) but **the shape is defensible**. Decide whether to
re-date it or move it to the §2.1 no-pin shape, **apply the same choice to all three pages, and say
which you chose.**

## 3. Adjudicate the other eight

`architecture.md`, `concurrency-locking.md`, `data-model.md`, `data-model-lifecycle.md`,
`integrity-recovery.md`, `platform-support.md`, `release-compatibility.md`, plus the two §2.1 model
pages. Most have **no self-description line at all.**

**Two questions, reported per page:**

1. **Does it carry a version pin or self-narration I missed?** My scan matched a specific set of
   phrasings; **a page could say the same thing in words my grep never saw.**
2. **Should a page with no self-description line get one?** **My view is no** — silence claims nothing,
   and the two model pages show the useful shape when one is wanted. **But adjudicate it; do not treat
   my view as settled.**

**Do not add a self-description line to eight pages for consistency's sake.** That would be eight new
things to keep current, which is the defect this increment removes.

**`release-compatibility.md:19`** — *"Tags through 0.17.7 predate this policy"* — is a **legitimate
historical statement about specific tags**, not a self-pin. **Verify that reading, then leave it.**

## 4. Out of scope

- **The 21 guide pages.** Their failure mode is *instructions that no longer work*, which needs a
  different method — closer to running them than reading them. **Report anything you notice; do not fix.**
- **`README.md`, `MILESTONES.md`, `ROADMAP.md`, the badge.**
- **No code.** If a page is right and the code wrong, **report it.**

## 5. What to report

1. **All 11 pages adjudicated** — `DEFECTIVE` / `CURRENT`, with what you checked. **The `CURRENT`
   verdicts are half the deliverable.**
2. **For every pin you removed: what you verified in the body, and against what.** §1 is the whole
   increment; a removed pin with no verification behind it is a regression. **If you left a pin in
   place, say which and why** — that is a good outcome, not a failure.
3. **Your §2.3 choice** (re-date vs no-pin), applied consistently.
4. **Your §3 answer** on whether unpinned pages should gain a line.
5. **Full gate set against the exact commit, after the last edit**, plus `mdbook build`.
6. Test counts — **expected unchanged**.
7. Anything here that was wrong. **My scan is a grep over phrasings I guessed at** — §3 item 1 exists
   because I expect it to have missed something. Across this arc my scope claims have been wrong three
   times, **twice in the under-fixing direction.**

**Stop and escalate, do not guess**, if: verifying a page's body would take longer than the rest of the
increment combined (**say so — that page becomes its own increment, and leaving its pin is the correct
interim state**); or you find a page whose body is materially wrong rather than merely mis-dated,
**which is a content defect and a different increment.**
