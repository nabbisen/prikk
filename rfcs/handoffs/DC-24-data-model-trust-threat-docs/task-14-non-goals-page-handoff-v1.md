# TASK-14 — the consolidated non-goals page

**Base:** current `main` (`c4c76d2`, CI green). **Under `003-landing-work-on-main.md`.**
**Owner-authorized.** This is `ROADMAP.md`'s own `TASK-14`, whose trigger has fired.

**Its row states the completion condition, and it is binding:** *"Reviewed non-goals page is committed
and links ROADMAP as the planning authority."* **Durable home: `docs/src/reference/non-goals.md`**,
which does not exist yet.

---

## 1. Why now

The trigger reads *"start when deferred-feature lists begin drifting across README, ROADMAP, mdBook,
and release notes."*

**It has fired.** In the last two days five themes were deleted from `ROADMAP.md` and two more
dissolved. Non-goal statements currently live in **at least ten files** — `ROADMAP.md` plus nine
`docs/src/` pages — with no page that collects them and no way to tell whether the set is complete.

## 2. The distinction this page exists to draw

**A non-goal is not a deferred feature, and conflating them is the drift.**

- **Deferred** — *"eventually built, not yet."*
- **Refused** — *"will not be built at all."*

**This project already drew that line once, this session**, when `patch-algebra.md`'s deferred list was
corrected: *"'Deferred' means eventually built; automatic conflict resolution is refused by the
architecture and will not be built at all."* **`merge.md:102` now carries the same correction.**

**That is the page's whole job**: make the refused set findable and distinguishable, so nobody has to
discover the difference one page at a time.

## 3. What belongs on it — refused, not deferred

**Re-derive the set yourself**; these are the ones I know, from decisions recorded this session:

- **Automatic conflict resolution** — refused by the architecture (DC-35 applied at the patch layer by
  DC-74). A resolution is a patch; automation cannot author signed content on a person's behalf.
- **Repository identity** — repositories are anonymous. No `project_id`, no peer identity, no origin
  field, *because none exists*. `0x0A` is retired and must never be reassigned.
- **Networked transport** — prikk stays off the network by RFC 116's accepted ruling. Moving a sync
  artifact is the operator's own channel, by design, not a gap.
- **Library API stability before 1.0** — only the `prikk` CLI is a supported surface; the seven library
  crates are published implementation detail.

**Some of these are already stated authoritatively elsewhere.** **Link, do not restate** — a sentence
byte-identical to `trust-threat-model.md`'s or `patch-algebra.md`'s belongs in one place, and this page
is not it. **The page's value is the collection and the distinction, not the prose.**

## 4. What must not happen

- **Do not restate deferred work as refused.** If something is merely unscheduled, it belongs in
  `ROADMAP.md`, and the page must **link ROADMAP as the planning authority** — the row's own completion
  condition.
- **Do not invent non-goals.** Every entry needs a decision behind it — an RFC, a ruling, or a
  documented design constraint. **If you cannot cite one, it is not a non-goal, it is an opinion.**
- **Do not absorb the ten existing statements.** They stay where they are; this page points at them.

## 5. Wire it in

`docs/src/SUMMARY.md`, in the Reference section. **Report where you placed it and why.**

## 6. Out of scope

- **`ROADMAP.md`'s own themes and backlog tables** — including `TASK-14`'s own row. **I will mark it
  done when this lands**; do not edit the table.
- **`README.md`.** Its own limits paragraph is current and separately maintained.
- **Rewriting any of the ten existing statements.**
- **Any code change.**

## 7. Controls

1. **Every entry cites a decision** — quote the citation for each, and say plainly if any lacks one.
2. **No sentence is byte-identical to its source page** — show it mechanically, as the crate-README
   increment did.
3. **The page links ROADMAP as the planning authority** — the row's binding completion condition.
4. **`mdbook build` clean**, the page reachable from `SUMMARY.md`, every link resolving.
5. **Full gate set green, test count unmoved** — documentation only.

## 8. What to report

1. **The page, in full.**
2. **Your re-derived refused set** (§3), including anything I missed and anything of mine you rejected.
3. All five controls (§7), quoted.
4. **Full gate set against the exact commit, after the last edit**, including `mdbook build`.
5. **Every numbered requirement's disposition, including ones that went without incident.**
6. Anything here was wrong.

**Stop and escalate, do not guess**, if: something looks refused but has no decision behind it — **that
is a design question for me, not a documentation call**; or a decision I cite in §3 turns out narrower
than I have stated it — **I recorded four rulings in two days and may have over-generalised one.**
