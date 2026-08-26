# The three live DC-era proposals — establish status, resolve the dangling predecessor

**Base:** current `main` (`c588abe`). **Under `003-landing-work-on-main.md`.**
**Owner-authorized.** Origin: the DC-* naming was raised as a source of future confusion; the sweep
found the confusion is **status, not naming**.

---

## 1. What this is not

**Do not rename anything.** There are **90 DC-* files** — 3 proposed, 34 accepted, 50 done, 3 archived
— and DC numbers are cited **from code**: `ci.yml` cites DC-71 B2 for the tar-not-zip ruling,
`prikk-ffi` cites DC-96, the trust docs cite DC-35. Those citations point at **decisions**, not files.
**Renaming breaks the link between a code comment and the ruling that justifies it, for cosmetic
gain.** `rfc_naming.rs`'s frozen legacy allowlist already makes these names legal by explicit decision.

**Only the three in `rfcs/proposed/` are in scope**, because only those appear in the RFC 120 open-work
index beside owner-originated concepts, where a reader cannot tell they are a different kind of thing.

## 2. DC-43 — its stated predecessor was superseded a month before

DC-43 positions itself as *"third remaining post-M1 increment, **after DC-42**."*

**DC-42 does not exist as live work.** It is in `rfcs/archive/`, **superseded 2026-07-29**, split into
**DC-56, DC-57, DC-58** — all three now in `rfcs/accepted/` — and, per `EXECUTION-ORDER.md:187`,
**never implemented.**

**So DC-43's schedule position cites a predecessor that was dissolved into three others before this
position was written or has gone stale since.** Establish what it should now say:

- does DC-43 sit after **DC-56/57/58**, or after some subset;
- or is the ordering simply obsolete and should be stated as such?

**Do not invent an ordering.** If the record does not support one, **say the position is unknown** —
that is a truer statement than a plausible guess, and this project has ruled that way before.

**DC-43 is not closable.** Its own text makes it *"a prerequisite for any public-preview
reconsideration."* **Whatever else changes, that claim must survive or be explicitly retired by the
owner — not by this increment.**

## 3. DC-44 — establish whether it is deferred or abandoned

**M3, beyond M2, target release not assigned.** It tracks NFR-REL-03, format-migration exercises, and
backup/restore evidence *"missing from the 0.17.7 architecture review."*

**Determine whether that gap still exists.** Much has shipped since 0.17.7 — the format-stability
contract (RFC 114), the release-compatibility gate (RFC 119 G1), migration coverage in
`format_stability_gate.rs`. **If parts of DC-44's scope have been overtaken, say which**; if the
remainder is real, say that.

**This is the one of the three most likely to be closable, and the one where I am least confident.**
Report evidence, not a verdict you are unsure of.

## 4. DC-49 — confirm the blocker still blocks

Its own text: *"Blocked. Not startable until the M1 portability-claim correction ships."*
`MILESTONES.md:463` repeats it.

**Establish whether that correction has shipped.** If it has, DC-49 is unblocked and its own text is
stale. If it has not, **name what remains**, so the block is checkable rather than inherited.

## 5. What to produce

**Not a rewrite of the three RFCs.** For each, establish the **current, evidenced status** and record it
where a reader meets it — **the RFC 120 open-work index entry is the natural place**, since that is
where the confusion arises.

**Keep the index thin** (RFC 120 §6 Q1). One clause per entry is enough: blocked-on-X, prerequisite-for-Y,
superseded-in-part. **If a status needs a paragraph, it belongs in the RFC itself, not the index.**

**The index is gated** — `boundary-check`'s `open-work-index` category checks both directions. **Do not
break it**; the markers scope the read.

## 6. Out of scope

- **Renaming, or any change to the legacy allowlist** (§1).
- **The 87 DC files outside `rfcs/proposed/`.**
- **Closing DC-43** (§2) — the owner's, if anyone's.
- **Deciding whether any of the three gets worked.** This establishes status, not priority.
- **Any code change**, unless the index gate needs one to keep passing.

## 7. Controls

1. **Every status claim cites its evidence** — a file, a line, a commit. **A status without a citation
   is the defect this increment exists to remove.**
2. **The open-work index gate still passes**, both directions.
3. **`mdbook build`** if `docs/src/` is touched (I expect none), and **`boundary-check` clean**.
4. **Full gate set green**, count unmoved unless a gate changed — say which and why.

## 8. What to report

1. **The three statuses**, each with citations.
2. **What DC-43's schedule position should now say**, or that it is unknown (§2).
3. **Whether DC-44's gap still exists**, with evidence (§3).
4. **Whether DC-49's blocker still blocks** (§4).
5. All four controls (§7), quoted.
6. **Full gate set against the exact commit, after the last edit.**
7. **Every numbered requirement's disposition, including ones that went without incident.**
8. Anything here was wrong.

**Stop and escalate, do not guess**, if: DC-43's prerequisite claim conflicts with something shipped
since — **that is a claim about public-preview readiness and is the owner's**; or a DC file outside
`rfcs/proposed/` turns out to be live work rather than history — **that would widen this increment and
I want to decide that, not have it absorbed.**
