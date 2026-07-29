# RFC (proposed) - DC-58 Source-Structure Audit

**Status.** Proposed. Requires design review before implementation may begin.
**Supersedes.** Item 2 of DC-42 (`rfcs/archive/DC-42-PERFORMANCE-MAINTAINABILITY-GATES.md`).
**Requirement.** **NFR-MAINT-02** (module boundaries) and the project Rust development and testing rules.
No milestone gate is missed here — this is corrective-M2 maintainability work, not a requirement gap.
**Touches.** Module layout across the workspace. **No behaviour, no public API, no persisted byte.**

## Problem

Architect review N5 found no gate on source or test structure. Two consequences, both measured on
2026-07-29:

- **23 oversized implementation files** — 7 over 500 lines, 16 between 300 and 500. Largest:
  `prikk-store/src/lifecycle_cache.rs` (974), `prikk-store/src/patch_replay/decode.rs` (733),
  `prikk-object/src/payload/patch.rs` (652).
- **3 inline `mod tests` blocks** remaining under `src/`, against the project testing rule that tests live
  in sibling modules.

The inline-test figure is worth stating plainly because DC-42 implied a much larger job: **it is three
files, not a campaign.** Whoever scopes the work should re-measure rather than inherit an estimate.

## Design

### 1. Report before splitting

Produce a source-structure report listing every implementation file with its ELOC, flagged against the
thresholds. The report is the deliverable that makes the rest reviewable; splitting without it produces a
large diff no one can evaluate for completeness.

### 2. Thresholds

- **Over 300 ELOC** — requires a **recorded split decision**. "Leave as is, because X" is a valid decision;
  silence is not.
- **Over 500 ELOC** — split, unless design review accepts a stated cohesion exception.

### 3. Scope the audit to production files, and name the exclusions

**This is the part most likely to cause damage if left implicit.**
`crates/prikk-object/src/vectors/hard.rs` is 624 lines and would trip the over-500 rule, but it is
`#[cfg(test)]`-gated (`crates/prikk-object/src/lib.rs:16`) and is DC-41 and DC-55 **identity evidence**.
Splitting it would fragment frozen golden vectors for a line-count target.

The same reasoning protects `crates/prikk-hash/src/tests/frozen_outgoing.rs`, whose module documentation
states plainly that it must never be edited — it is DC-55's differential reference and is immutable by
design.

The audit must therefore:

- cover **implementation** files only;
- enumerate test-support exclusions explicitly, with the reason for each;
- treat any `#[cfg(test)]`-gated evidence file as out of scope by default.

### 4. Inline test modules

Move the 3 remaining inline `mod tests` blocks under `src/` to sibling test modules per the project
testing guidelines.

### 5. Mechanical extraction only

Extraction must preserve public module paths and observable behaviour. This is a **pure refactor**: if a
split changes what any caller can see, it has exceeded scope and needs its own design.

## Non-goals

- No arbitrary crate split or public API redesign.
- **No weakening of tests to satisfy line-count targets.** Inherited from DC-42, which named the obvious
  way to game this gate.
- No performance work — DC-56 and DC-57 own the two performance requirements.
- No behaviour change of any kind.

## Risks

**Splitting frozen evidence.** Covered by design item 3; it is the one way this increment could do real
harm, because the damage would be to the evidence base that DC-41 and DC-55 established rather than to
code that tests would catch.

**Line count as a proxy for cohesion.** A 900-line file with one clear responsibility may be better than
three 300-line files with a tangled dependency between them. The over-500 rule permits a stated cohesion
exception for exactly this reason, and design review should expect a few rather than treat every exception
as a failure.

**Diff size defeating review.** 23 files is too large for one review unit. Stage it: the report first, then
splits in reviewable batches. Each batch is independently verifiable because behaviour must not change.

## Acceptance criteria

1. A source-structure report exists and is committed, covering every implementation file with ELOC against
   thresholds.
2. Test-support exclusions are enumerated with a reason for each; `vectors/hard.rs` and
   `frozen_outgoing.rs` are among them.
3. Every file over 300 ELOC has a recorded split decision; every file over 500 is split or carries an
   accepted cohesion exception.
4. The 3 inline `mod tests` blocks under `src/` are relocated.
5. **Public module paths and observable behaviour are unchanged** — evidenced by the full test suite
   passing with unchanged per-crate counts, and by no diff in any identity artifact.
6. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

Criterion 5 is the one that matters: this increment's correctness claim is that *nothing changed*, and an
unchanged test count across a large refactor is what evidences it. All criteria are verifiable from the
repository.
