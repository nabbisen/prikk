# RFC (proposed) - DC-62 Commit Benchmark Memory Axis

**Status.** **Accepted by the project owner on 2026-07-30.** Implementation may begin.

**Independence.** Authored and reviewed by the architect; one architect, so design review here is author
re-examination. Acceptance criteria are written to be reproducible from the repository, so the
implementation review carries the independent weight.
**Size:** small. One axis added to an existing harness, plus a report section.
**Arises from.** DC-56 design review v2, which found that commit reads every file's contents into memory
(`worktree_files.rs:11-14`, `bytes: Vec<u8>` per file) — a scalability defect no requirement names. DC-56's
acceptance criterion 5 requires evidencing its removal, and DC-59's harness measures wall-clock only.
**Touches.** `crates/prikk-cli/tests/dc59_commit_benchmark.rs` and its committed report.
**No production code.**

## Why a new increment rather than amending DC-59

DC-59 is complete and its implementation review was accepted with no findings. Its acceptance criteria were
all discharged against the scope it had, which named two axes and wall-clock timing.

Adding a criterion to it now would retroactively make a finished increment unfinished — the "rewrite the
requirements after implementation" pattern the operating instructions prohibit. The memory axis is **new
scope arising from a later review**, not a DC-59 defect. So DC-59 stays closed and correct, and this is its
own increment.

## Problem

Commit memory is O(total worktree bytes) regardless of change size: a 1 GB worktree allocates 1 GB whether
one byte changed or none. DC-56 will fix that with a changed-path index that skips reads for unchanged
files.

**The direction of the fix is verifiable from a diff; the magnitude and the absence of a remaining unbounded
path are not.** The specific risk is not "did memory improve" but "is there still a path that loads
everything" — an index that itself materialises all content, or a fallback that does. Reading the diff can
miss that. Measurement catches it. That is why DC-56 criterion 5 requires evidence rather than argument.

## What this requires that does not exist yet

*Mandatory section. Checked before specifying, per the pattern established across DC-56, DC-59, DC-60, and
DC-61.*

| Needed | Exists? |
|---|---|
| A timed child-process invocation to attach measurement to | **Yes** — `dc59_commit_benchmark.rs:275-284` times `Command::output()` |
| Peak-memory reading of the child **without a new dependency** | **Constrained.** `.output()` waits for the child and returns after it exits, so nothing can be read from `/proc/<pid>` afterward. `rustix` is workspace-declared with `features = ["fs"]` only — no `getrusage`/`wait4`. `std` exposes neither. **The only dependency-free route is to `spawn()` and sample `/proc/<pid>/status` `VmHWM` while the child runs** |
| Precedent for `/proc` reading in this workspace | **None.** This introduces it |

DC-59 criterion 1 forbade a new dev-dependency, and that constraint carries here. Adding `libc` or widening
`rustix`'s features for a benchmark would be a poor trade, and widening a workspace dependency's feature set
touches DC-51's placement surface.

## Design

### 1. Sample `VmHWM` while the child runs

Replace `.output()` with `.spawn()` in the timed path only. While the child runs, poll
`/proc/<child_pid>/status` for `VmHWM` at a fixed interval, keep the maximum, then `wait()` and collect
output as before.

`VmHWM` is the kernel's own peak-resident-set high-water mark, so a single successful read late in the run
captures the peak — sampling is to ensure *at least one* read lands while the process is alive, not to
reconstruct a curve.

**Timing must not regress.** The elapsed measurement stays exactly as DC-59 defined it, so Axis A and Axis B
remain comparable to the existing report. If sampling perturbs timing measurably, the two must be run as
separate passes rather than one — state which was done.

### 2. State the limitation honestly

Sampling cannot capture the peak of a run shorter than the interval. At 10 files the commit takes ~4 ms, so
a sample may not land at all.

**That is acceptable and must be stated rather than hidden.** Memory is a question about large repositories;
at 10,000 files the run is ~516 ms and gives ample sampling room. The report must record the sampling
interval, the sample count actually obtained per point, and mark any point where no sample landed as **not
measured** — never as zero, and never omitted silently.

### 3. Linux-only, deliberately

`/proc` is Linux. DC-37 already establishes Linux-only mutation support, and this harness is an `#[ignore]`d
local instrument, not a gate. Record the constraint in the module docs so a future reader on another
platform gets a clear skip rather than a confusing failure.

### 4. Report section

Add a memory axis to `rfcs/handoffs/DC-59-commit-benchmark-harness/benchmark-report-v1.md` — or a v2
alongside it, whichever the design review prefers — covering the same repository sizes as Axis A, at fixed
change count. The claim under test is that peak memory tracks total worktree size, which is what DC-56 must
later flatten.

## Non-goals

- No new dev-dependency, and no widening of an existing dependency's features.
- No change to Axis A or Axis B, or to how elapsed time is measured.
- No production code change.
- No CI integration — same reasoning as DC-59.
- No conclusion about whether the memory footprint is acceptable. That is DC-56's, as latency was.

## Risks

**Sampling perturbs the timing figures**, making the new report incomparable with DC-59's. Mitigated by §1's
separate-pass fallback; the design review should decide whether to require separate passes outright.

**A missed sample read as zero.** The one way this increment produces a false result. §2 requires
*not measured* as a distinct outcome, and the implementation must not default a missing sample to any number.

**`/proc` parsing fragility.** `VmHWM` is a stable field, but the parser should fail loudly on an unexpected
format rather than returning a default.

## Acceptance criteria

1. Peak memory is measured for the same repository sizes as Axis A, at fixed change count, and recorded in
   the report.
2. **No new dev-dependency and no widened features** — verified against the manifests and `Cargo.lock`.
3. Sampling interval and per-point sample count are recorded; any point with no sample is marked
   **not measured**, distinctly from zero.
4. Axis A and Axis B timing figures remain comparable to DC-59's report — either unchanged, or produced by a
   separate pass with that stated.
5. The Linux-only constraint is documented in the harness module docs, with a clear skip on other platforms.
6. No production code changed — evidenced by the diff.
7. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

All seven are verifiable from the repository. Criterion 3 is the one that matters most: a missing sample
silently becoming a number would make the whole axis untrustworthy, and DC-56 would then be evidencing its
memory claim against a fabricated baseline.
