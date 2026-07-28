# DC-52 Python and Oracle Decommissioning - Implementation Handoff

**Prepared in advance.** Implementation may **not** begin until `rfcs/proposed/DC-52-…` moves to
`rfcs/accepted/` through design review, **and** the later-commit stability evidence DC-45 requires has
been accepted.
**Authored by** the architect (function-designer role). Implementation review remains independent.
**Size:** medium, but strictly ordered — four steps, each separately reviewable, no bundling.
**Touches:** `release/` Python files, frozen oracle material, `tools/release-policy` responsibility map.
No product code.

## Why this exists

DC-45 made the Rust policy command authoritative but deliberately retained the Python implementation and
the frozen oracle as a rollback path, on a staged retirement schedule. Four obligations from that schedule
are still open and live only in prose across `MILESTONES.md`, `rfcs/IMPLEMENTATION-STATUS.md`, and
architect review records. Left there, the retained Python is either removed prematurely — losing rollback
— or retained forever, defeating the consolidation DC-45 paid eleven review rounds to achieve.

## Step 1 — Bind the responsibility map (precondition for step 3)

The 50-entry responsibility map is not mechanically bound to an executed Rust check registry. Make it
fail closed in both directions:

- a map entry with no corresponding executed check → error;
- an executed check with no map entry → error.

A one-directional check is not sufficient; the failure mode DC-45 worried about is drift, which shows up
in whichever direction is unguarded.

## Step 2 — Make the `defaults.run` invariant explicit (precondition for step 3)

The governed-procedure YAML extractor skips an empty `run` value whose nearest shallower key is
`defaults`, relying on the GitHub Actions schema forbidding an executable scalar there. That reasoning is
correct today but *assumed*. I accepted it at review v11 on condition it be made explicit before Python
retirement.

Validate that the block nested under `defaults.run` contains only known configuration keys — `shell`,
`working-directory` — and error otherwise. This converts a schema assumption into an enforced rule and
protects the exception against future extractor changes.

**Trap:** `command_scan/procedure.rs` is a review-gated policy artifact. This is a policy change, not a
refactor, and it must not alter any other extraction or classification behaviour. Prove the existing
governed-file corpus still scans identically.

## Step 3 — Decommission the five Python files

Exhaustive, one file at a time. For each: **either** remove it with equivalence evidence, **or** record an
owner-approved, event-bound exception naming the event that would allow removal. No file may be silently
retained, and "we might need it" is not an event.

Do not start before steps 1 and 2 are accepted.

## Step 4 — Rule on the eight frozen contract/evidence files

For each: replacement, consolidation, or final retirement — with the **rollback consequence** of the
choice stated. This step may proceed independently of step 3.

## Standing constraints

- Rust remains the authoritative policy command throughout. This increment does not re-open cutover.
- No change to the public policy schema, the product publication graph, or differential oracle semantics.
- **No removal of rollback capability before its replacement evidence is accepted** — this is the whole
  point of the staging.
- No signer, release-lane, or publication action.

## Definition of done

- Responsibility map fails closed on divergence in both directions.
- `defaults.run` accepts only `shell` and `working-directory`; all other nested keys error.
- Each of the five Python files: removed with evidence, or carrying an explicit, event-bound,
  owner-approved exception.
- Each of the eight frozen files has a recorded disposition with its rollback consequence.
- At **every** step: release-policy `check` passes all 154 oracle cases, `boundary-check` and
  `reference-check` remain `valid: true`.
- Full gate set green (`rfcs/EXECUTION-ORDER.md` §6.8).

## Submit with

Per step: diff; evidence note (what was bound/validated/removed, and the equivalence or exception basis);
the 154-case oracle result; gate output; explicit statement that Rust remains authoritative and rollback
capability is intact or explicitly and knowingly reduced.
