# `ROADMAP.md` — two false themes and two fired triggers: implementation handoff

**Base:** current `main` (`0b11934`, CI + Docs green). **Under `003-landing-work-on-main.md`.**
**Origin:** owner asked what themes remain, 2026-08-24; scoping the answer found the roadmap describing
shipped features as unbuilt.

**Deliberately narrower than the staleness I found.** §5 explains what is being held back and why —
**do not widen into it.**

---

## 1. Two themes state that shipped features do not exist

**`ROADMAP.md:466`, "Merge execution — CONFIRMED as the next accepted increment":**

> *"`merge-evidence` and `merge-plan` exist; **nothing applies a merge**... **RFC not yet written**."*

**DC-74 shipped `prikk merge` in 0.19.0.** `CHANGELOG.md`'s 0.19.0 entry: *"**`prikk merge` executes a
merge.**"*

**`ROADMAP.md:533`, "Cross-platform mutation — open question, not scheduled":**

> *"**mutation is Linux-only**, so prikk cannot be *used* off Linux, only inspected."*

**Shipped in 0.21.0. Criterion 6 is MET** — Linux, macOS and Windows all mutate, CI-gated on all three.

**This is the README false-claim class, in the file whose entire purpose is directing effort.** As it
stands the roadmap would send a contributor to build merge execution, and tell them prikk cannot mutate
off Linux.

**Correct both. Do not delete the sections** — the same treatment the sync bullet already received in
this arc: state what shipped, when, and keep whatever residual is genuinely still open. **Merge
execution has real residuals** (semantic merge, rename detection, automatic merge-base discovery);
**cross-platform mutation has the two documented Windows narrower guarantees.** Derive both from
`MILESTONES.md` and `CHANGELOG.md`, not from my summary.

## 2. Two "later" entries whose trigger has fired

- **`:430` "Repository layout when sync arrives — decided 2026-08-04, applied later."** **Sync arrived**
  (0.23.0). The decision was *nested directories under one workspace*, with today's crates moving to
  `crates/shared/`. **Adjudicate: is it now due, superseded, or does it need re-ruling?** **Do not
  perform any layout change** — report your reading.
- **`:543` "MSRV policy — to write before packaging is attempted."** **Packaging happened** — crates.io
  and prebuilt binaries across four targets. The policy is **overdue, not pending**. `rust-version =
  "1.85"` is declared; what is missing is the stated rule for when it may rise. **Correct the framing;
  writing the policy itself is a separate increment.**

## 3. Everything else in Future Themes — adjudicate, do not assume

Remaining: multi-parent block lineage, conflict arbitration, peer trust, quarantine policy, patch
aggregation, structured output, editor/IDE integration. **I read all seven as still genuinely open**,
but that is a reading. **Check each against the code and say so** — a `CURRENT` verdict is as much of
the deliverable as a correction.

**Peer trust deserves particular attention:** RFC 116/117 shipped sync, which makes *"what a remote is
permitted to assert"* more live, not less. **If its text now understates the question, say so.**

## 4. Out of scope

- **`MILESTONES.md`, `CHANGELOG.md`, `README.md`, the badge.**
- **No code.** Including §2's layout decision — **report only.**
- **The four status sections** (`Current Increment`, `Release Candidate Increment`,
  `Last Released Increment`, `Next Increments`) — **see §5.**

## 5. What is deliberately held back, and why

**`ROADMAP.md`'s four status sections are stale by up to six releases** — `Last Released Increment` says
**0.17.7**; the tag list says **0.23.0**. **And `rfcs/IMPLEMENTATION-STATUS.md` is stale by the same
amount**: *"Latest released version: 0.17.7"*, zero mentions of RFC 115/116/117, accepted increments
stopping at DC-63.

**Two documents drifting six releases in the same way is evidence they are not maintained — which makes
"what should they say?" the wrong first question.** The right one is whether they should exist at all,
given `CHANGELOG.md` and `MILESTONES.md` are both current and already carry release history and
criterion state.

**Refreshing an unmaintained tracker produces a fourth thing to drift.** That question is the
architect's to put to the owner, and it is being put. **Do not touch either document's status sections
in this increment**, and **do not touch `rfcs/IMPLEMENTATION-STATUS.md` at all.**

**If you form a view while working, report it** — you will have read more of both files than anyone.

## 6. What to report

1. **Both §1 corrections**, with the authority for each and what residual you kept.
2. **Your §2 adjudications** — due, superseded, or needs re-ruling, with reasoning.
3. **All seven §3 themes**, each `OPEN` or `STALE`, with what you checked.
4. **Any view on §5**, report only.
5. **Full gate set against the exact commit, after the last edit.**
6. Test counts — **expected unchanged**.
7. Anything here that was wrong, **including my line numbers**.

**Stop and escalate, do not guess**, if: a §3 theme turns out to be shipped and its correction is larger
than a paragraph; §2's layout decision appears to have been silently superseded by work already done; or
you find a **third** theme stating a shipped feature does not exist — **that would make this a pattern
rather than two corrections.**
