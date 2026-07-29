# RFC (proposed) - DC-59 Commit Benchmark Harness

**Status.** Proposed. Requires design review before implementation may begin.
**Split from.** DC-56, per design review v1 finding B2 — the benchmark was one paragraph inside an RFC
whose decision depends entirely on it.
**Requirement.** Produces the evidence artifact **NFR-PERF-01** names:
`specs/prikk-non-functional-requirements-v1.1.md` §4.5, Evidence column — "Commit benchmark report."
**Touches.** A new benchmark harness and its committed report. **No production code.**

## Problem

`prikk` has **no benchmark infrastructure at all** — no `benches/` directory in any crate, no `[[bench]]`
target, no `criterion` or comparable harness in any manifest, no `examples/`. Verified across the
workspace on 2026-07-29.

Every performance number this project has produced was measured ad hoc and is **not reproducible from the
repository**. DC-50's ~5.8× hashing figure and DC-55's confirming table
(`implementation-evidence-v1.md` §"Step 6") are both honest and both unverifiable by a reviewer — DC-55's
implementation review accepted its performance criterion explicitly as "not independently verifiable,"
which was the right call given what existed but is not a standard worth keeping.

DC-56 must decide whether commit complies with NFR-PERF-01's no-full-tree-scan rule. That decision rests
on a measured cost curve. Producing that curve is a self-contained piece of work with its own design
questions, and DC-56 gave it a sentence.

**This increment produces evidence and decides nothing.** Whether commit complies, and what to do if it
does not, is DC-56's.

## Design

### 1. Harness form — no new dependency

The measurement needed is wall-clock cost of a whole `commit` across repositories of controlled size. That
is not statistical micro-benchmarking, so `criterion`'s value is low relative to adding a dependency tree
to a workspace that has deliberately kept three third-party production crates and a short dev list.

**Preferred form:** an integration test under `crates/prikk-cli/tests/`, `#[ignore]`d by default so it
never slows the normal suite, driving the real binary through `CARGO_BIN_EXE_prikk`. This follows the
DC-55 end-to-end test's existing pattern
(`crates/prikk-cli/tests/dc55_sha256_identity_end_to_end.rs`), which already generates a repository,
drives the binary, and asserts on results — proven in this codebase.

Design review may select a different form. What it may **not** do is leave the choice to the implementer,
since the dependency question is a placement-gate-adjacent decision under DC-51.

### 2. Deterministic repository generation

The generator must be **reproducible**: given the same parameters it must produce byte-identical
repositories, so a reviewer rebuilding the curve measures the same thing.

Parameters: file count, directory breadth and depth, and file size. Content derived from a fixed seed —
the `SplitMix64` used by `crates/prikk-hash/src/tests/hash_differential.rs` is already in the tree and
already reviewed, and reusing it avoids introducing a second notion of determinism.

Record the generator's parameters in the report. A curve whose inputs cannot be reconstructed is not
evidence.

### 3. Two axes, because one cannot distinguish the hypotheses

**Axis A — cost against repository size, at fixed change count.** Vary total file count across at least
four points spanning two orders of magnitude (e.g. 10 / 100 / 1,000 / 10,000 files), changing exactly one
file each time. **This is the axis that demonstrates or refutes the full-tree scan.** If commit cost grows
with repository size while the change set is constant, the scan is doing the work.

**Axis B — cost against change count, at fixed repository size.** Vary changed-file count at a constant
total size. This establishes the cost that NFR-PERF-01 explicitly *permits* — patch construction scales
with the change set, and that is the requirement's own bound.

Axis A alone is insufficient: without B there is no baseline for what proportionate cost looks like, and
a reviewer cannot tell a scan from expensive per-change work. Reporting a single latency number, or Axis A
alone, is the failure mode this increment exists to prevent.

### 4. Committed, reviewable report

Results land in a **committed report file** with: generator parameters, both curves as tables, machine and
filesystem context, and the exact command that reproduces them. Not pasted into a review note — the point
is that the next increment, and the next reviewer, can re-run it.

State the filesystem, since commit cost includes fsync and NFR-PERF-01 names fsync in its bound. A number
measured on tmpfs and a number measured on a journaling filesystem are different claims.

### 5. Scope discipline

The harness measures the commit path as it exists. It does not modify it, optimise it, or prepare it for
modification.

## Non-goals

- **No decision about NFR-PERF-01 compliance.** That is DC-56's, and it needs this evidence to make it.
- No optimisation, caching, index, or traversal change of any kind.
- No production code change.
- No hashing benchmark — DC-50 and DC-55 covered that, and re-deriving it is out of scope. (Retrofitting
  those figures into this harness is a reasonable follow-up, not part of this increment.)
- No CI integration. Adding a `run:` command to `.github/workflows/ci.yml` requires a reviewed classifier
  amendment under standing rule 8, and a long-running benchmark does not belong in the normal gate set.

## Risks

**Measuring the wrong thing.** If generation places all files in one directory, the curve may reflect
directory-entry scaling rather than tree traversal. Vary breadth and depth, and record both.

**Filesystem noise swamping the signal.** Commit includes fsync. Warm-up runs and repeated trials with a
recorded median are needed, or run-to-run variance will exceed the effect being measured. Specify the
trial count in the report.

**The harness becoming a maintenance burden.** It is `#[ignore]`d and not in the gate set, so it can rot
undetected. Accept this deliberately: it is a measurement instrument, run when a performance question is
open. Say so in its module documentation so a future reader does not mistake dormancy for neglect.

## Acceptance criteria

1. A committed benchmark harness exists, adds **no new production dependency**, and is excluded from the
   default test run.
2. Repository generation is deterministic and parameterised; the same parameters reproduce byte-identical
   repositories.
3. Axis A measured across at least four repository sizes spanning two orders of magnitude, at fixed change
   count.
4. Axis B measured across varying change counts at fixed repository size.
5. A committed report records both curves, generator parameters, trial count, machine and **filesystem**
   context, and the exact reproduction command.
6. No production code changed — evidenced by the diff.
7. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

Criteria 1–4 and 6 are verifiable from the repository. Criterion 5's *numbers* are hardware-dependent and
will differ per reviewer — but the **shape** of Axis A is the claim that matters, and that must reproduce.
Say so in the report: a reviewer confirming the curve's shape has confirmed the finding.
