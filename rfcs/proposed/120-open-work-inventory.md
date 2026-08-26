# RFC 120 — Open-work inventory: one place the candidate set is complete

**Status.** **Proposed** — design recorded by the architect at the owner's instruction, 2026-08-26,
after answering *"what are our remaining themes?"* with an incomplete list. **No implementation
authority; the gate in §5 must not be built before this is accepted.**

**Tracks.** A correction to how this project records open work, not a product capability.

---

## 1. What went wrong, precisely

Asked what remained, the architect read `ROADMAP.md`'s **Future Themes** section and answered from it.
That section names four items. The real open surface is spread across at least four places, and the
answer omitted most of it.

**Measured, not asserted:**

```
ROADMAP.md mentions of `proposed/`:                     0
RFC 108 (Workspace)          referenced in ROADMAP/MILESTONES: 0
RFC 109 (agent-native interface)                              0
RFC 110 (agent safety and provenance)                         0
RFC 113 (history import foundations)                          0
DC-43 / DC-44 / DC-49                                    7 / 1 / 4
```

**The four newest proposed RFCs — all authored or originated by the project owner — are unreachable
from either document a reader consults to ask what remains.** The older `DC-*` items are referenced
only because they predate the RFC-100 numbering and were woven in at the time.

**No amount of care fixes this.** The index does not point at them; finding them requires already
knowing they exist.

## 2. The defect class is the project's own

This is the pattern RFC 118 exists to remove — **an inventory that lives in more than one place with
nothing asserting the set is complete.** It is `parent_patch_ids` documented at six sites, or the
object-taxonomy table drifting from `ObjectType`.

**One difference makes it worse.** Those were facts *transcribed* into several places, so any copy
could be checked against the source. This is a fact **partitioned** across places, where **no source
claims to be whole**. Nothing was stale. Nothing was wrong. The answer was simply incomplete, and
nothing could have said so.

## 3. What a mechanism can and cannot guarantee

**Stating this plainly, because the value of the proposal depends on it.**

- **Enumeration completeness — mechanisable.** Every file in `rfcs/proposed/` and every incomplete
  milestone row appears in one index, or a gate fails.
- **Description accuracy — not mechanisable.** A gate binds *existence*, not truth. A one-line summary
  can go stale exactly as `ROADMAP.md`'s Sync heading did — accurate body, false framing. This needs
  the same currency discipline as every other document.
- **Priority — not mechanisable, and should not be.** Cost, risk, dependency and owner intent are
  judgment. **No gate produces a ranking, and one that appeared to would be lying.**

**What this buys is narrow and worth having: the candidate set is complete.** A priority decision is
then made over everything rather than over what someone remembered. **An omission is worse than a
mis-ranking** — a ranking can be disagreed with; an omission cannot be seen.

## 4. The gap a gate alone does not close

**Open work does not reliably have a file.**

This session produced at least three open items recorded in **no tracked file**: the `sync build`
*"already in sync"* wording question (RFC 116 §4 behaviour that misleads a user who committed without
sealing), the missing local reproduction path for the cross-host CI jobs, and `MILESTONES.md:334`'s
stale `M5` row. They live in `.git-exclude/reviewed/` — **which is git-excluded, and therefore not in
the repository at all** — and in the architect's session memory.

**A gate over two directories would enumerate them faithfully and omit these silently.**

**So the proposal is two things, and the second is a habit:**

1. **Gate the enumeration** (§5).
2. **Record open work found in review as a tracked line** when it is not fixed immediately.

**A habit is exactly what the owner asked not to depend on.** The gate cannot enforce that a line was
*written*, but it can enforce that **every source it knows about is represented** — so the habit's
absence is visible for anything with a file, and only genuinely fileless findings can escape. **That is
a reduction in exposure, not elimination, and it should not be described as more.**

## 5. The gate, if this is accepted

**Bind two derivable sets to one authored index:**

- every `rfcs/proposed/*.md`;
- every milestone row in `MILESTONES.md` not marked complete.

**Fail when a member of either set is not named in the index.** Do not check ordering, wording, or
priority — only presence.

**Precedents, all in-repo and proven:** `trust_gated_operations_binding_gate.rs` (enum bound to a
markdown section), `object_type_table_binding_gate.rs` (enum bound to a markdown table, both
directions), the MSRV transcription gate (`Cargo.toml` bound to six sites), and `reference-check`'s
inventory-plus-scanner. **This is a fifth application of a shape that already works here — not a new
mechanism.**

**Scope the index to `ROADMAP.md`**, which is where the question naturally goes. **Binding
`MILESTONES.md` means editing that file, which requires the owner's explicit instruction** — so the
milestone half of §5 is separable and may be deferred without weakening the RFC-half.

## 5a. Findings from the §6 research, 2026-08-27

**The dev team researched §6 and built nothing, as the status line requires.** Two of its evidence
claims were wrong in ways that changed the rulings, and one produced a new finding.

**`ROADMAP.md`'s backlog tables are not historical.** The research reported them "fully populated with
`Done`/`Released` rows — nothing new has landed." They hold **three `Open` rows**:

| TASK-14 | consolidated non-goals / deferred features | *start when deferred-feature lists begin drifting* |
| TASK-15 | roles & user-classes orientation | *start when the docs need a clearer audience map* |
| TASK-16 | error taxonomy & diagnostics | *start with TASK-07 or when diagnostics need user-facing intent* |

**Both the researcher and the architect read that section as historical and skipped it.** It is a third
instance of §1's own defect — open work sitting where a reader stops looking — and this one is *inside
`ROADMAP.md`*, the document §5 proposes to make authoritative. **TASK-14's own trigger arguably fired
this week**, given how many deferred-feature entries were deleted.

**`rfcs/accepted/` holds thirteen RFCs, not five.** The research cited 115–119. It also contains 100,
102, 103, 105, 106, 107, 112, and 114 — long finished. **That difference decides Q3.**

## 6. Rulings

**Q1 — thin lines, and do not absorb the existing tables.** The gate checks presence only, so any
column it does not read is unverified transcription that can drift. **But the backlog tables are live
(§5a) and their `Trigger / next action` column is what makes TASK-14/15/16 actionable rather than a
nag.** So: **a thin gated index, which *references* those tables rather than replacing them.** Do not
flatten a working structure to fit a new one.

**Q2 — defer the milestone half.** `MILESTONES.md`'s "State today" column is free prose (`MET,
2026-08-22`; `OPEN, and deliberately so — not a blocker`), and a regex over it is fragile in both
directions. **Adopting the researcher's reasoning verbatim: encoding a heuristic over prose the gate
does not control is a worse failure than not gating yet.** Revisit only if a structured status
convention is adopted, which needs the owner's instruction independently.

**Q3 — `rfcs/proposed/` only; exclude `rfcs/accepted/`.** Thirteen accepted RFCs are dominated by
finished work, and distinguishing "accepted and still open" from "accepted and shipped" needs the same
prose-marker that Q2 just refused. **An index that lists eight finished RFCs teaches readers to ignore
it** — the opposite failure from the one this RFC exists to fix, and no less damaging.

**Q4 — a named section in the same document.** Adopting the researcher's reasoning: a separate tracked
file recreates a second place a reader must know to check. **The gate still cannot enforce that a
fileless finding was ever written down** (§4), and that limit stays stated.

**Net scope: gate `rfcs/proposed/*.md` against one thin index section in `ROADMAP.md`, with a
"findings without a file" section beside it.** Substantially smaller than §5 first implied, and every
reduction came from evidence rather than from wanting less work.

## 7. Open questions a design must answer

1. **Does the index carry one line per item, or the existing backlog table's shape**
   (`ID | Tier | Owner | Status | Trigger / next action | Completion condition`)? The richer shape
   invites staleness; the thinner one carries less.
2. **What marks a milestone row "incomplete"** in a way a gate can read without interpreting prose?
3. **Do accepted-but-unfinished RFCs belong in the set?** `rfcs/accepted/` holds work in progress;
   including it widens the gate meaningfully.
4. **Where do fileless findings go** — a tracked backlog file, or a line in the index itself?
   **This is §4's unresolved half and the one that decides whether the index is genuinely single.**

## 8. What this is not

- **Not a priority mechanism** (§3).
- **Not a claim that the index is true**, only that it is complete with respect to its sources.
- **Not a replacement for review records.** `.git-exclude/reviewed/` stays where reasoning lives; the
  index carries only that an item is open.
