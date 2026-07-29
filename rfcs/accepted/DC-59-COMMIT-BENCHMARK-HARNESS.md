# RFC (proposed) - DC-59 Commit Benchmark Harness

**Status.** **Accepted by the project owner on 2026-07-29**, after design review v1 returned three
blocking findings (measurement loop, unreachable PRNG, missing signing prerequisite) and all were resolved
in revision at `f8d0938`. Implementation may begin.
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

**Content parameters:** file count, directory breadth and depth, and file size, with content derived from
a fixed seed.

**The PRNG is duplicated in the harness, deliberately** (per review v1 B2). An earlier draft proposed
reusing the `SplitMix64` in `crates/prikk-hash/src/tests/hash_differential.rs`; that is not reachable —
it is a private struct in a `#[cfg(test)]` module of a different crate. The alternatives are to promote it
into shared test-support, which touches `prikk-hash` material DC-55 froze on purpose, or to carry a small
local copy. Carry the copy, and say in the harness documentation that it is a deliberate duplicate of a
reviewed generator rather than a second invented one.

**Identity prerequisites — without these the measured command refuses to run** (per review v1 B3). Commit
requires author signing: `crates/prikk-cli/src/main.rs:124` calls `author_signer_from_env()`, which fails
closed unless both `PRIKK_AUTHOR_KEY_ID` and `PRIKK_AUTHOR_SEED` (64 hex characters) are set
(`main.rs:431-443`). A usable repository also needs the trust material the DC-55 fixture carries:
`.prikk/trust/policy.toml` and a maintainer public key under `.prikk/trust/keys/maintainer/`.

Use a **fixed author key seed** across all runs so measurements are comparable and reproducible. Record it
in the report; it is benchmark material, not a credential.

Record every generator parameter in the report. A curve whose inputs cannot be reconstructed is not
evidence.

### 3. The measurement loop: one commit per generated repository

**Commit cannot be run twice against the same repository** (per review v1 B1).
`crates/prikk-store/src/worktree_patch/node_authoring.rs:202-207` fails closed once the active WAL is
non-empty — "active WAL already contains patches for {ref}; run `prikk seal` before committing again". A
naive repeated-trial loop errors on its second iteration.

**Each timed commit runs against a freshly generated repository.** Variance comes from sampling several
repositories at each size point, not from repeating commits within one. Rejected alternatives, recorded so
they are not re-proposed:

| Alternative | Why not |
|---|---|
| Seal between trials | Seal cost enters the loop. NFR-PERF-01 does not bound seal, so this measures the wrong thing |
| Regenerate and re-time in one repository | Same as the chosen approach, but invites including generation cost in the timing window |

**Generation happens outside the timing window.** Time the `commit` invocation only.

State the sample count per size point in the report. This makes the run more expensive than a
repeated-trial loop would be — generating many large repositories is the dominant cost of this increment,
and that is expected rather than a surprise to discover during implementation.

### 4. Two axes, because one cannot distinguish the hypotheses

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

### 5. Committed, reviewable report

Results land at **`rfcs/handoffs/DC-59-commit-benchmark-harness/benchmark-report-v1.md`** (per review v1
N1 — an unnamed location invites two increments disagreeing), with: generator parameters including the
fixed author seed, both curves as tables, sample count per point, machine and filesystem context, and the
exact command that reproduces them. Not pasted into a review note — the point is that the next increment,
and the next reviewer, can re-run it.

**Say where signing cost sits.** NFR-PERF-01's bound explicitly includes *signature*, so Ed25519 signing
is inside what the requirement permits — but a curve that cannot separate signing from traversal cannot
answer DC-56's question. Axis B isolates it: signing scales with the change set, so its contribution shows
up there and not in Axis A's growth. State this in the report rather than leaving the reader to infer it.

**Record the process-spawn floor** (per review v1 N2). Driving `CARGO_BIN_EXE_prikk` through `Command`
puts process startup inside every measurement — a roughly constant additive offset that does not hide
Axis A's shape but may dominate at the small end. Time a trivial invocation and report it as the floor, so
a reader comparing the 10-file and 100-file points knows what is baseline.

State the filesystem, since commit cost includes fsync and NFR-PERF-01 names fsync in its bound. A number
measured on tmpfs and a number measured on a journaling filesystem are different claims.

### 6. Scope discipline

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

**Filesystem noise swamping the signal.** Commit includes fsync. Sampling several repositories per size
point with a recorded median is needed, or run-to-run variance will exceed the effect being measured.
Repeated commits within one repository are not available as a variance-reduction technique — see design
§3.

**The harness becoming a maintenance burden.** It is `#[ignore]`d and not in the gate set, so it can rot
undetected. Accept this deliberately: it is a measurement instrument, run when a performance question is
open. Say so in its module documentation so a future reader does not mistake dormancy for neglect.

## Acceptance criteria

1. A committed benchmark harness exists, is excluded from the default test run, and **adds no new
   dev-dependency** — the constraint that actually binds (per review v1 N3; a test harness could not add a
   production dependency in any case, so the original wording constrained nothing).
2. Repository generation is deterministic and parameterised; the same parameters reproduce byte-identical
   repositories. Generation includes trust material and a fixed author key seed, so the measured command
   runs.
3. **Each timed commit runs against a freshly generated repository**, with generation outside the timing
   window and sampling across repositories at each size point.
4. Axis A measured across at least four repository sizes spanning two orders of magnitude, at fixed change
   count.
5. Axis B measured across varying change counts at fixed repository size.
6. The report at `rfcs/handoffs/DC-59-commit-benchmark-harness/benchmark-report-v1.md` records both
   curves, generator parameters, the fixed seed, sample count per point, the process-spawn floor, machine
   and **filesystem** context, where signing cost appears, and the exact reproduction command.
7. No production code changed — evidenced by the diff.
8. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after per rule 10.

Criteria 1–5, 7 and 8 are verifiable from the repository. Criterion 6's *numbers* are hardware-dependent
and will differ per reviewer — but the **shape** of Axis A is the claim that matters, and that must
reproduce. Say so in the report: a reviewer confirming the curve's shape has confirmed the finding.
