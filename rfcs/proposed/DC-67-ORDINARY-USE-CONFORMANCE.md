# RFC (proposed) - DC-67 Ordinary-Use Conformance Suite

**Status.** **Proposed 2026-08-02.** Awaits owner acceptance.
**Authored by** the architect.
**Independence.** Authored and reviewed by the architect — the standing ceiling. Compensated by §5's
criteria being reproducible from the repository, and by §2's prediction being falsifiable.
**Arises from.** Three consecutive increments in which the defect that mattered was found by running
*sequences*, not by inspection or by any existing gate.
**Requirement.** None names this. That is the finding.

## 1. The problem, stated as evidence rather than opinion

**Until 2026-07-31, prikk could not edit the same text file twice.** Create, commit, seal, edit, commit,
seal, edit → `integrity error: baseline content Blob … is missing`. Long-standing. Reproduced
independently by the architect on a pre-fix commit.

What passed over it: **563 prikk-store tests, 80 object tests, a crash matrix, a property/fuzz campaign,
and the entirety of DC-41 — an increment whose sole purpose was integrity evidence.**

The bug is closed. **The reason it survived is not**, and it has since recurred twice more:

| # | Where | Found by |
|---|---|---|
| 1 | `plan_edit_text` reading an unstored `blob_id` (DC-65) | a benchmark axis failing for an unrelated reason |
| 2 | DC-64's incremental step, per-block `TextCache` | DC-65's own five-generation test, at generation 3 |
| 3 | DC-66's queue fold, empty `TextCache` | DC-66's queue test |

**Three for three: none was found by inspection, and none by an existing gate.** All three are the same
shape — a path that only misbehaves on the *second or later* time it runs against a given thing.

## 2. The diagnosis, and a falsifiable prediction

This project's assurance is aimed hard at one axis — **adversarial and structural failure**: malformed
input, torn writes, crash points, wrong types, hostile encodings. That axis is genuinely well covered, and
DC-41 covered it deliberately.

The orthogonal axis is **ordinary sequential use**, and it is close to uncovered. Almost every test
exercises the *first* time a path runs against a given node, ref, or repository. Defects that depend on
history are invisible to that test shape **by construction** — DC-65's own coverage finding said exactly
this, and DC-66's repeated it one level up.

> **Prediction, recorded so this RFC can be judged: a suite of the shape in §3 will find at least one
> further defect of this class.**

If it finds none, that is real evidence the coverage is better than I believe, obtained cheaply, and this
increment closes as a permanent regression guard. **Either outcome is a good one**; only not looking is bad.

## 3. What is built

A conformance suite of **ordinary user sequences, run through the compiled binary**, each at **N ≥ 3
generations** where a generation is a mutate → commit → seal cycle. Not unit tests, not adversarial cases:
sentences a user would say.

Minimum set, each of which is one plain-language workflow:

1. Edit the same text file across N generations *(now covered, by accident of DC-65 — keep it)*
2. Edit the same **binary** file across N generations
3. Create → delete → recreate the same path, repeatedly
4. Create a file, edit it, delete it, then create a different file at the same path
5. Change a file's mode across generations, with and without content changes
6. Branch, commit on both branches, close one, verify, keep committing on the other
7. Tag, then keep committing past the tag; tag again
8. Queue N commits, seal as one block, then queue N more and seal again *(DC-66)*
9. Delete the caches (`commit-index`, `lifecycle-state`) mid-sequence and continue *(NFR-PERF-04)*
10. Every sequence above ends by **deleting the worktree and rebuilding it from sealed history**, asserting
    byte-exact content

**Criterion 10 is the point.** `verify` passing proves history is structurally valid; rebuilding content
proves it is **semantically correct**. The architect's DC-66 verification used exactly this and it is the
strongest cheap check available.

## 4. What this requires that does not exist yet

| Prerequisite | State |
|---|---|
| A store-level multi-generation helper | **Exists** — `seal_active_patch`, added by DC-65 and extended to N by DC-66 |
| A CLI-level equivalent | Partially — each of DC-61/65/66's test files rolls its own `commit`/`seal`/key setup. **Consolidate before writing ten more** |
| An agreed value of N | **Open.** DC-65 used 5, DC-66 used 4, the architect used 6. §5 says ≥ 3; pick one and justify it |

**No blocking measurement.** Unusually for this program, everything needed already exists — which is itself
evidence this suite should have been written earlier.

## 5. Acceptance criteria

1. All ten sequences in §3 implemented, each at the chosen N, through the compiled binary.
2. Every sequence ends with a delete-and-rebuild content assertion (§3.10).
3. The shared CLI harness is consolidated, not copy-pasted an eleventh time.
4. **Every defect found is reported, not fixed in place.** A correctness fix folded into a test increment
   is the amendment-of-convenience this program refuses. Each becomes its own finding.
5. If **no** defect is found, that is stated plainly as the result — not padded.
6. The suite runs in the ordinary `cargo test --workspace --locked` gate, or its exclusion is justified
   with a runtime measurement.
7. A statement of what ordinary-use shapes remain **uncovered** after this increment. The list will not be
   empty and pretending otherwise repeats the original error.
8. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after.

## 6. Non-goals

- **Fixing what it finds.** See criterion 4.
- **Adversarial or fuzz cases.** That axis is covered; this is the other one.
- **Performance.** DC-59/62's harnesses own that.
- **Replacing existing tests.** This is an added axis, not a substitution.
