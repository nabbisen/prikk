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

## 6. Open questions a design must answer

1. **Does the index carry one line per item, or the existing backlog table's shape**
   (`ID | Tier | Owner | Status | Trigger / next action | Completion condition`)? The richer shape
   invites staleness; the thinner one carries less.
2. **What marks a milestone row "incomplete"** in a way a gate can read without interpreting prose?
3. **Do accepted-but-unfinished RFCs belong in the set?** `rfcs/accepted/` holds work in progress;
   including it widens the gate meaningfully.
4. **Where do fileless findings go** — a tracked backlog file, or a line in the index itself?
   **This is §4's unresolved half and the one that decides whether the index is genuinely single.**

## 7. What this is not

- **Not a priority mechanism** (§3).
- **Not a claim that the index is true**, only that it is complete with respect to its sources.
- **Not a replacement for review records.** `.git-exclude/reviewed/` stays where reasoning lives; the
  index carries only that an item is open.
