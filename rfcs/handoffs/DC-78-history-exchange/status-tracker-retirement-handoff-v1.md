# Retire `rfcs/IMPLEMENTATION-STATUS.md` and `ROADMAP.md`'s status sections

**Base:** current `main` (`9694f41`). **Under `003-landing-work-on-main.md`.**
**Owner-authorized 2026-08-24** ("retire both"). **Take after
`roadmap-false-theme-currency-handoff-v1.md` lands** — both touch `ROADMAP.md`.

**A retirement, not a refresh.** Both documents drifted to a 0.17.7-era snapshot while ~20 increments and
six releases landed. **Refreshing an unmaintained tracker produces a fourth thing to drift**; `CHANGELOG.md`
and `MILESTONES.md` stay current because every increment touches them.

---

## 1. `rfcs/IMPLEMENTATION-STATUS.md` — banner, do not delete

**Follow the project's own precedent:** `.git-exclude/tasks/dev-team/002-merge-branches.md` was retired
with a banner at the top — *"RETIRED 2026-08-23... Kept rather than deleted, because the incidents
recorded in it are the reason it was retired."* **Read it before writing yours.**

The banner must say: **retired**, **what replaced each of its jobs** (release history → `CHANGELOG.md`;
criterion and schedule state → `MILESTONES.md`; per-increment design state → `rfcs/accepted/` and
`rfcs/done/`), and **why it is kept** — its record of which increments were accepted when is real
history, and its drift is the evidence for retiring it.

**Leave the body untouched below the banner.** Do not correct its stale figures — **a retired document
that has been half-corrected is worse than one plainly marked as a snapshot of its time.**

## 2. `ROADMAP.md` — remove the four status sections

`## Current Increment`, `## Release Candidate Increment`, `## Last Released Increment`,
`## Next Increments`. **`ROADMAP.md` becomes purely forward-looking themes** — the half that stayed
honest.

**Two things to check rather than assume:**

- **`Next Increments`** carries M0-M4 milestone framing. **If any of it is forward-looking rather than
  status, keep that part** — possibly under `Future Themes`. **Adjudicate; do not delete wholesale.**
- **`ROADMAP.md:181,193-194`** hold **release-lane fields** (`parked` / `none`). Those belong to the
  official-release regime, which `MILESTONES.md` §"Durable release-lane transition" now scopes
  explicitly (`9694f41`). **Do not silently drop them** — either keep them where a future activation
  would find them, or say in your report where they should live. **This is the one place a careless
  deletion loses something load-bearing.**

## 3. Inbound references — the part most likely to break

**`IMPLEMENTATION-STATUS.md` is referenced from at least ten files**, including four published reference
pages (`patch-algebra.md`, `trust-threat-model.md`, `data-model.md`, `release-compatibility.md`),
`CHANGELOG.md`, `ROADMAP.md`, and several `rfcs/done/` records.

**Adjudicate each. Do not sweep.** Three different cases, and they need different treatment:

- **Anchor-table citations** in the reference pages — the *claim* may still hold with a retired source.
  **Precedent from `f69779c`: a citation to partly-retired work is fine when the cited claim is still
  true.** Verify, then leave.
- **Historical records** (`rfcs/done/*`) — **leave entirely.** They record what was true then.
- **Live prose asserting it is current** — that is the defect class. **`MILESTONES.md:5` was one and I
  have already fixed it** (`9694f41`). **Find any others.**

**Report every reference you found and what you did**, including the ones you left alone.

## 4. Out of scope

- **`MILESTONES.md`** — mine; already corrected.
- **The reference pages' own claims** — adjudicated at `93c0b53`; touch only a citation if §3 requires it.
- **No code.**

## 5. What to report

1. **The banner text** (§1).
2. **Each of the four ROADMAP sections**, and your `Next Increments` / release-lane adjudications (§2).
3. **Every inbound reference**, with its verdict (§3) — the ones you left alone included.
4. **Full gate set against the exact commit, after the last edit**, plus `mdbook build` if any
   `docs/` file changed.
5. Test counts — **expected unchanged**.
6. Anything here that was wrong, **including my "at least ten files"**.

**Stop and escalate, do not guess**, if: a reference asserts something about `IMPLEMENTATION-STATUS.md`
being authoritative that a retirement would falsify and you cannot tell what should replace it; the
release-lane fields (§2) turn out to be load-bearing somewhere I have not seen; or removing a status
section would take genuinely forward-looking content with it — **retaining too much is the recoverable
error here, deleting too much is not.**
