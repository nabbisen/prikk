# DC-58 Source-Structure Audit - Handoff

**Cleared to start**, with one file deferred — read §"Deferred" first.
Accepted by the project owner on 2026-07-29, at `rfcs/accepted/DC-58-SOURCE-STRUCTURE-AUDIT.md`. Architect
design review returned no blocking findings.
**Authored by** the architect.
**Size:** large in file count, zero in behaviour. **No behaviour, no public API, no persisted byte.**

## What this is

A structure gate the project has never had, plus the splits it turns up.

**The correctness claim of this entire increment is that nothing changed.** That is unusual and it shapes
everything below: an unchanged test count across a large refactor is the evidence, and a changed one is a
defect regardless of whether tests still pass.

## Deferred — do not touch this file

**`crates/prikk-store/src/worktree_patch/node_authoring.rs` is out of scope for now**, despite being 601
lines and therefore over the split threshold.

Three increments want it at once: DC-58 would split it, **DC-56** may restructure its traversal to close
NFR-PERF-01, and **DC-59** benchmarks the commit path running through it. Splitting it while DC-56's design
is unsettled produces conflicting edits and invalidates DC-59's "measures the path as it exists" premise.

It returns to scope once DC-56 records an outcome. Note it in your report as **deferred with a reason**,
not as an oversight — a later reader must be able to tell the difference.

Everything else oversized is independent of the performance work.

## Step 1 — report first

Produce the source-structure report before splitting anything. It lists every **implementation** file with
its ELOC, flagged against the thresholds.

The report is what makes the rest reviewable. Splitting first produces a large diff nobody can evaluate
for completeness.

Current measurements, taken 2026-07-29 — re-measure rather than inheriting these:

- **7** implementation files over 500 lines
- **16** between 300 and 500
- Largest: `prikk-store/src/lifecycle_cache.rs` (974), `prikk-store/src/patch_replay/decode.rs` (733),
  `prikk-object/src/payload/patch.rs` (652)

## Step 2 — scope the audit to production, and name every exclusion

**This is the step where the increment could do real harm.**

`crates/prikk-object/src/vectors/hard.rs` is 624 lines and trips the over-500 rule. It is
`#[cfg(test)]`-gated (`crates/prikk-object/src/lib.rs:16`) and is **DC-41 and DC-55 identity evidence** —
frozen golden vectors. Splitting it fragments the evidence base for a line-count target.

`crates/prikk-hash/src/tests/frozen_outgoing.rs` is the same class: its own module documentation says
plainly that it must never be edited. It is DC-55's differential reference and immutable by design.

So:

- Cover **implementation** files only.
- Enumerate test-support exclusions **explicitly, with a reason for each**.
- Treat any `#[cfg(test)]`-gated evidence file as out of scope by default.

A blanket line-count sweep that catches these is a failed audit even if every test still passes.

## Step 3 — thresholds

| ELOC | Rule |
|---|---|
| over 300 | **Recorded split decision required.** "Leave as is, because X" is a valid decision; silence is not |
| over 500 | Split, unless design review accepts a stated cohesion exception |

Expect to propose a few cohesion exceptions. A 900-line file with one clear responsibility can be better
than three 300-line files with tangled dependencies between them, and the rule permits that — an exception
with a reason is a normal outcome, not a failure.

## Step 4 — inline test modules

Move the remaining inline `mod tests` blocks under `src/` to sibling test modules per the project testing
guidelines.

**There are 3.** Measured 2026-07-29. The RFC's framing implied a larger campaign; it is not one.
Re-measure and report the count you find.

## Step 5 — mechanical extraction only

Extraction preserves **public module paths and observable behaviour**. This is a pure refactor. If a split
changes what any caller can see, it has exceeded scope and needs its own design.

Stage the work: report first, then splits in reviewable batches. 23 files is too large for one review
unit, and each batch is independently verifiable precisely because behaviour must not change.

## Traps

- **Splitting frozen evidence.** Covered in step 2; the one way this increment causes lasting damage.
- **Touching `node_authoring.rs`.** Covered above.
- **Weakening or deleting tests to satisfy a line-count target.** The obvious way to game this gate, and
  explicitly prohibited.
- **Treating line count as a proxy for cohesion** rather than a prompt to think about it.
- **Changing behaviour "while you're in there."** Any behaviour change belongs in its own increment.

## Definition of done

A committed source-structure report covering every implementation file with ELOC against thresholds;
test-support exclusions enumerated with reasons, including `vectors/hard.rs` and `frozen_outgoing.rs`;
`node_authoring.rs` recorded as deferred with its reason; every file over 300 carrying a recorded split
decision; every file over 500 split or carrying an accepted cohesion exception; inline `mod tests` blocks
relocated; **public module paths and observable behaviour unchanged.**

## Submit with

The diff; the report; test counts per touched crate before and after — **these must be identical**, and a
difference is a finding, not a detail; confirmation that no identity artifact changed
(`vectors/snapshot.txt`, `vectors/hard.rs`, `state_root/tests/vectors.rs`, `text_span/vectors.rs`); an
explicit statement of what did not change; and the full gate set from `rfcs/EXECUTION-ORDER.md` §6 rule 9
including release-policy `check`, `boundary-check`, and `reference-check`.
