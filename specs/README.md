# Prikk Specification Authorities

These are the project's **requirements authorities**. They are tracked here so that requirement
citations, gate labels, and amendments are versioned, diffable, and reviewable like any other change.

Brought under version control on 2026-07-29 by DC-42 design review v2 finding B2. Before that they lived
in an untracked directory, which meant an amendment produced no commit, no diff, and no history — and a
reviewer could not verify that a requirement said what an implementation claimed it said.

| Document | Authority for |
|---|---|
| `prikk-app-requirements-v1.2.md` | Functional product requirements. §6.2 carries the commit rule NFR-PERF-01 restates |
| `prikk-non-functional-requirements-v1.1.md` | The NFR matrix — every `NFR-*` ID, its target rule, its **milestone gate**, and its required evidence |
| `prikk-roadmap-milestones-v1.1.md` | The original product milestone scheme (M0–M7) that the NFR gate labels refer to |

## Amending a requirement

An amendment is an ordinary reviewed commit against these files. It requires:

- an accepted RFC that states the amendment and its rationale;
- the project owner's approval, since requirement changes are a reserved decision;
- a corresponding update to `MILESTONES.md` where the requirement is gate-tracked.

Do not amend a requirement to match an implementation that failed to meet it. Record the gap instead, and
let the RFC decide whether to close it or to change the requirement deliberately.

## Two milestone schemes — read this before resolving any gate label

**Gate labels in the NFR matrix (`M0`–`M7`, `Beta`, `Public Preview`) belong to the original product
scheme defined in `prikk-roadmap-milestones-v1.1.md`. They do not refer to `MILESTONES.md`.**

`MILESTONES.md` defines a *separate*, later corrective scheme that reuses the labels `M0`–`M3` with
different meanings. Resolving an NFR gate against it produces the wrong answer about whether a requirement
is met, overdue, or not yet due. The mapping table is maintained in `MILESTONES.md`.

## Not tracked here

`.git-exclude/specs/` retains the external design document, the FDD package, the kickoff plan, the
dependency map, and historical handoff bundles. Those are working material rather than requirement
authorities. If one of them becomes normative for an increment, track it here first.

## Note on `prikk-app-requirements-v1.2.md`

Its filename says v1.2; its own title line says "Prikk Stable App Requirements v1.1" with status
"supporting refresh v1.2". The filename is treated as authoritative for citation purposes. Worth
reconciling at the next amendment.
