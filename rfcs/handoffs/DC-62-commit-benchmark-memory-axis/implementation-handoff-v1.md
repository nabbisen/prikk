# DC-62 Commit Benchmark Memory Axis - Handoff

**Cleared to start.** Accepted by the project owner on 2026-07-30, at
`rfcs/accepted/DC-62-COMMIT-BENCHMARK-MEMORY-AXIS.md`. No gate remains.
**Authored by** the architect.
**Size:** small. One axis added to an existing harness, plus a report section.
**Touches:** `crates/prikk-cli/tests/dc59_commit_benchmark.rs` and its committed report.
**No production code.**

## What this is

Add peak-memory measurement to DC-59's existing commit benchmark.

**Why:** DC-56's design review found that commit reads every file's full contents into memory —
`worktree_files.rs:11-14`, `bytes: Vec<u8>` per file, held in a `BTreeMap` across authoring. So commit
memory is O(total worktree bytes) regardless of change size. DC-56 will fix that with a changed-path index,
and its acceptance criterion 5 requires evidencing the fix. DC-59's harness measures wall-clock only.

**You are measuring, not fixing.** Whether the footprint is acceptable is DC-56's question, exactly as
latency was.

## The constraint that shapes the whole design

**No new dev-dependency**, carried from DC-59 criterion 1. That rules out the easy routes, and the design
review already worked out what remains:

- `Command::output()` (used today at `dc59_commit_benchmark.rs:275-284`) **waits for the child and returns
  after it exits** — so nothing can read `/proc/<pid>` afterward.
- `rustix` is workspace-declared with `features = ["fs"]` only. No `getrusage`, no `wait4`.
- `std` exposes neither.

**So: switch the timed path from `.output()` to `.spawn()`, poll `/proc/<child_pid>/status` for `VmHWM`
while the child runs, keep the maximum, then `wait()` and collect output as before.**

`VmHWM` is the kernel's own peak-RSS high-water mark, so one successful read late in the run captures the
peak. Sampling exists to ensure *at least one* read lands while the process is alive — not to reconstruct a
curve.

## The one way this produces a false result

**A missed sample must be reported as "not measured" — never as zero, never omitted.**

Sampling cannot catch the peak of a run shorter than the interval. At 10 files a commit takes ~4 ms, so a
sample may not land at all. That is acceptable and expected — memory is a large-repository question, and at
10,000 files the run is ~516 ms with ample room.

What is not acceptable is a missing sample silently becoming a number. **Do not default it.** If it defaults
to zero, the report shows memory improving from 0 at the small end, and DC-56 would then evidence its memory
claim against a fabricated baseline. This is acceptance criterion 3 and the thing I would look at first.

## Timing must stay comparable

The elapsed measurement stays exactly as DC-59 defined it, so Axis A and Axis B remain comparable with the
existing report.

**If sampling perturbs timing measurably, run the two as separate passes** rather than one — and state which
you did. A memory axis that quietly invalidates the latency figures would cost more than it adds.

## Linux-only, deliberately

`/proc` is Linux. DC-37 already establishes Linux-only mutation support, and this harness is `#[ignore]`d and
not in the gate set.

Record the constraint in the module docs, and make other platforms **skip clearly** rather than fail
confusingly.

## Report

Add a memory axis to `rfcs/handoffs/DC-59-commit-benchmark-harness/benchmark-report-v1.md`, or a v2 alongside
it — your choice, say which. Cover the same repository sizes as Axis A, at fixed change count.

Record: the sampling interval, the **sample count actually obtained per point**, any point marked *not
measured*, and the reproduction command. The claim under test is that peak memory tracks total worktree size
— which is what DC-56 must later flatten.

## Traps

- **Defaulting a missed sample to a number.** The one way this increment lies.
- **Letting sampling change the timing figures** without saying so.
- **Reaching for `libc` or widening `rustix`'s features.** Both are new-dependency changes; widening a
  workspace dependency's features also touches DC-51's placement surface.
- **Parsing `/proc` loosely.** `VmHWM` is a stable field; fail loudly on an unexpected format rather than
  returning a default — same reasoning as the missed sample.
- **Touching production code.** This measures the commit path as it exists.

## Definition of done

Peak memory measured across Axis A's repository sizes at fixed change count; no new dev-dependency and no
widened features; sampling interval and per-point sample count recorded; unsampled points marked *not
measured*; Axis A and Axis B timing comparable to DC-59's report or produced by a stated separate pass;
Linux-only constraint documented with a clean skip elsewhere; no production code changed.

## Submit with

The diff; the report section; **verification that no dependency changed** — manifests and `Cargo.lock`
diffed and shown empty; the sampling interval and per-point sample counts; explicit statement of any point
marked *not measured*; confirmation that timing figures are comparable or that a separate pass was used; test
counts per touched crate before and after; and the full gate set from `rfcs/EXECUTION-ORDER.md` §6 rule 9 run
on a **clean checkout of the commit**, stated as such.
