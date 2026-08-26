# RFC 120 — build the open-work index and its completeness gate

**Base:** current `main` (`89812cd`). **Under `003-landing-work-on-main.md`.**
**RFC 120 is ACCEPTED** (`rfcs/accepted/120-open-work-inventory.md`) at the reduced scope its §6 rules.
**Build only that scope.**

---

## 1. What is in scope, and what the RFC deliberately excluded

**In:** every `rfcs/proposed/*.md` — **seven files today** — bound to one thin index section in
`ROADMAP.md`, plus a *"findings without a file"* section beside it.

**Out, by ruling, not oversight:**

- **`MILESTONES.md`'s milestone rows** (§6 Q2). Its status column is free prose; a regex over it is
  fragile both ways. **Do not gate it, and do not edit that file.**
- **`rfcs/accepted/`** (§6 Q3). Thirteen files, dominated by finished work. **An index listing eight
  finished RFCs teaches readers to ignore it.**
- **The existing backlog tables.** §6 Q1: the index **references** them; it does not absorb or replace
  them. **They are live** — `TASK-14`, `TASK-15`, `TASK-16` are `Open` with real triggers, which both
  the architect and the previous researcher misread as historical.

## 2. The index

**One thin line per proposed RFC**: its number, its title, and a link to the file. **Nothing else.**

**The gate checks presence only**, so any column it cannot read is unverified transcription that will
drift silently — that is why §6 ruled the shape thin. **Do not add owner, status, or priority
columns.** If a reader wants the current state of a theme, `ROADMAP.md`'s own theme prose is where that
lives.

**Seed it with all seven**, which are currently referenced **nowhere** in `ROADMAP.md`:

```
108-workspace-concurrent-sessions   109-agent-native-interface
110-agent-safety-and-provenance     113-history-import-foundations
DC-43-RELEASE-SECURITY-CONTROLS     DC-44-MIGRATION-BACKUP-RESTORE-EVIDENCE
DC-49-PORTABLE-LOGIC-PLATFORM-MATRIX
```

**Say plainly in the section's own text that it is an inventory, not a priority order**, and that
presence there is not a claim the item is current — only that it exists and is open. **RFC 120 §3 and
§8 already state this; the section must not overclaim beyond them.**

## 3. The "findings without a file" section

**Seed it with the three this session produced**, which live only in git-excluded review notes:

1. **`sync build` reports *"already in sync"* when the sender's ref is unsealed** — RFC 116 §4's
   deliberate behaviour (a test pins it), but it names a state the user is not in. **A question about
   wording on a ruled surface, not a defect.**
2. **No local reproduction path for the cross-host CI jobs** — the dev team had to hand-build a
   role-by-role proxy; it exists in no tracked file.
3. **`MILESTONES.md:334`'s `M5` row** still reads `| M5 | Sync and Quarantine | no | — |` while
   criterion 1 records sync **MET** and quarantine was dissolved.

**The gate cannot enforce that anything is written here** (RFC 120 §4). **Say that in the section
itself** so no reader mistakes its emptiness for absence of findings.

## 4. The gate

**Bind `rfcs/proposed/*.md` to the index section, both directions:**

- every file in `rfcs/proposed/` is named in the index;
- every entry in the index names a file that exists.

**Both directions matter.** The object-taxonomy gate proved it — a reviewer's control caught a wrong
code paired with a correct name precisely because it checked both ways.

**Scope the read with HTML comment markers**, as `trust_gated_operations_binding_gate.rs` and
`object_type_table_binding_gate.rs` both do, so the gate reads only the intended block and the rest of
`ROADMAP.md` stays free prose.

**Where it lives is yours to choose** — `tools/release-policy`'s `boundary-check` reads manifests and
`.md` already; a `#[cfg(test)]` binding gate in a crate is the other precedent. **Say which and why.**
**Do not build a third mechanism.**

## 5. Out of scope

- **`MILESTONES.md`** — any edit (§1).
- **`rfcs/accepted/`** in the gated set (§1).
- **Rewriting the backlog tables** (§1).
- **Priority, ordering, or status** in the index (§2).
- **Any product code.**

## 6. Controls

1. **A new proposed RFC that is not indexed fails** — add a throwaway file to `rfcs/proposed/`, quote
   the failure, remove it.
2. **An index entry naming a non-existent file fails** — the other direction. Quote it.
3. **The real tree passes** with all seven seeded.
4. **`mdbook build`** if any `docs/src/` file is touched (I expect none), and **`boundary-check`
   passes** — the index is new prose in a file it already scans.
5. **Full gate set green**, and say how the test count moved and why.

**Quote every failure.** After any control that deliberately fails a property test, check
`proptest-regressions/`.

## 7. What to report

1. **The index section and the findings section**, in full.
2. **Where you put the gate**, and why (§4).
3. All five controls (§6), quoted.
4. **Full gate set against the exact commit, after the last edit.**
5. **Every numbered requirement's disposition, including ones that went without incident.**
6. Anything here was wrong.

**Stop and escalate, do not guess**, if: the naming grammar rejects `DC-43`-style filenames in a way
that makes them unindexable — **those three predate the RFC-100 numbering and must still appear**; or
you find open work in a source neither RFC 120 nor this handoff names — **that is the RFC's own thesis
recurring, and I want it before the gate is finished.**
