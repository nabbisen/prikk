# DC-59 Commit Benchmark Harness - Handoff

**Cleared to start.** Accepted by the project owner on 2026-07-29, at
`rfcs/accepted/DC-59-COMMIT-BENCHMARK-HARNESS.md`. No gate remains.
**Authored by** the architect. Design review v1 returned three blocking findings, all resolved at
`f8d0938` — the RFC you are working from is the revised one.
**Size:** medium. One test file, one committed report. **No production code.**

## What this is

Measure what `prikk commit` costs, and publish a curve nobody has to take on trust.

This workspace has **no benchmark infrastructure at all** — no `benches/`, no `[[bench]]` target, no
harness in any manifest. Every performance figure the project has produced (DC-50's ~5.8× hashing number,
DC-55's confirming table) was measured ad hoc and cannot be reproduced from the repository. You are
building the thing that ends that.

**You are not deciding anything.** Whether commit complies with NFR-PERF-01 is DC-56's call, and it needs
your curve to make it. Resist concluding.

## Three things that will bite you, all found in review

### 1. You cannot commit twice to the same repository

`crates/prikk-store/src/worktree_patch/node_authoring.rs:202-207` fails closed once the active WAL is
non-empty:

```
active WAL already contains patches for <ref>; run `prikk seal --ref <ref>` before committing again
```

So a repeated-trial loop errors on iteration two. **Each timed commit runs against a freshly generated
repository.** Variance comes from sampling several repositories per size point.

Do **not** seal between trials to work around this — seal cost is not what NFR-PERF-01 bounds, and it
would contaminate the measurement.

**Generation happens outside the timing window.** Time only the `commit` invocation.

### 2. Commit refuses to run without signing set up

`crates/prikk-cli/src/main.rs:124` calls `author_signer_from_env()`, which fails closed
(`main.rs:431-443`) unless both are set:

- `PRIKK_AUTHOR_KEY_ID`
- `PRIKK_AUTHOR_SEED` — 64 hex characters

A usable repository also needs the trust material the DC-55 fixture carries: `.prikk/trust/policy.toml`
and a maintainer public key under `.prikk/trust/keys/maintainer/`. Look at
`crates/prikk-cli/tests/fixtures/dc55_pre_swap_repo/` for the exact shape.

Use a **fixed author seed** across every run so results are comparable, and record it in the report. It is
benchmark material, not a credential.

### 3. The PRNG you might reach for is not reachable

`SplitMix64` in `crates/prikk-hash/src/tests/hash_differential.rs:41` is a **private** struct in a
`#[cfg(test)]` module of a different crate. You cannot use it from `crates/prikk-cli/tests/`.

Carry a small local copy in the harness, and say in the module docs that it is a deliberate duplicate of a
reviewed generator — not a second invented one. Do **not** promote the original; `prikk-hash` test
material was frozen by DC-55 on purpose.

## Build it

An integration test under `crates/prikk-cli/tests/`, `#[ignore]`d so it never slows the normal suite,
driving the real binary through `CARGO_BIN_EXE_prikk`. Follow
`crates/prikk-cli/tests/dc55_sha256_identity_end_to_end.rs` — it already generates a repository, drives the
binary, and asserts on results.

**No new dev-dependency.** Not criterion, not anything else.

Generation parameters: file count, directory breadth and depth, file size. Vary breadth and depth
deliberately — piling every file into one directory measures directory-entry scaling, not tree traversal.

## Measure two axes

| Axis | Vary | Hold fixed | What it shows |
|---|---|---|---|
| **A** | repository size — ≥4 points over two orders of magnitude (10 / 100 / 1,000 / 10,000 files) | change count = 1 file | **Whether the full-tree scan exists.** Cost growing while the change set is constant means the scan is doing the work |
| **B** | changed-file count | repository size | The cost NFR-PERF-01 explicitly *permits* — patch construction and signing scale with the change set |

**Axis A alone is not enough.** Without B there is no baseline for what proportionate cost looks like, and
nobody can tell a scan from expensive per-change work. A single latency number, or A alone, is the exact
failure this increment exists to prevent.

## The report

`rfcs/handoffs/DC-59-commit-benchmark-harness/benchmark-report-v1.md`. It must carry:

- Both curves as tables, with sample count per point
- Every generator parameter, including the fixed author seed
- **Filesystem** — commit includes fsync and NFR-PERF-01 names fsync in its bound. A tmpfs number and a
  journaling-filesystem number are different claims
- Machine context
- **The process-spawn floor** — time a trivial `prikk` invocation and report it. Driving the binary through
  `Command` puts startup inside every measurement; it is a roughly constant offset that does not hide
  Axis A's shape but may dominate at the 10-file end
- **Where signing cost appears** — Ed25519 signing is inside what NFR-PERF-01 permits and scales with the
  change set, so it shows up in Axis B, not in Axis A's growth. Say so, so DC-56 can read the curve
- The exact command that reproduces all of it

Write it so a reviewer can rebuild the curve. The *numbers* are hardware-dependent and will differ; **the
shape of Axis A is the claim**, and that must reproduce.

## Traps

- **Concluding.** The report states what was measured. DC-56 decides what it means.
- **Sealing between trials**, or timing generation along with the commit.
- **Reporting a median without saying how many samples**, or a curve without its parameters.
- **Touching the commit path.** This increment measures it as it exists — no optimisation, no caching, no
  traversal change, no production code at all.

## Definition of done

An `#[ignore]`d harness with no new dev-dependency; deterministic generation including trust material and
a fixed seed; one timed commit per generated repository with generation outside the window; Axis A over
≥4 sizes and Axis B over varying change counts; the report above; no production code changed, evidenced by
the diff.

## Submit with

The diff; the report; the exact reproduction command; test counts per touched crate before and after; an
explicit statement that no production code changed; and the full gate set from
`rfcs/EXECUTION-ORDER.md` §6 rule 9 including release-policy `check`, `boundary-check`, and
`reference-check`.
