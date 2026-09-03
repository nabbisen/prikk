# Amendment v2 — the benchmark's number is 82× environment, and the README does not say so

**Amends:** `criterion-member-handoff-v1.md`, whose work is **accepted and pushed** (`cb2e2a2`,
CI 15/15). **Base:** current `main`. **This is a README addition — no code change.**

---

## 1. What I measured

Your report gives `commit_genesis_10_files time: [16.615 ms 16.733 ms 16.907 ms]` with
*"Collecting 100 samples in estimated 12.827 s (100 iterations)"*.

**I ran the same benchmark and got `[200.21 µs 203.04 µs 205.81 µs]`** with *"15k iterations"* — 82×
faster, on the same commit and the same code. That is far outside sampling noise, so I chased it:

```
TMPDIR unset -> /tmp (tmpfs):                  203 µs   "15k iterations"
TMPDIR=$PWD/.git-exclude/tmp (btrfs):    16.8–22.8 ms   "100 samples ... (100 iterations)"
```

**The second run reproduces your numbers and your iteration-count line verbatim.** Neither of us was
wrong. **The benchmark is dominated by the durability cost of the filesystem backing `$TMPDIR`** —
which is real work `commit` genuinely does, not an artifact — but it means the number measures the
disk as much as the code.

## 2. Why this is not a defect in what you built

`tempfile::tempdir()` honouring `$TMPDIR` is correct, and timing the real `fsync` path is what makes
this a commit benchmark rather than a CPU microbenchmark. **The benchmark is right. The README is
incomplete**, and that gap is the whole of this amendment.

## 3. Why it matters more here than it would elsewhere

`rfcs/EXECUTION-ORDER.md` §6 rule 9 says: *"Use a repository-local `TMPDIR` (`.git-exclude/tmp`)
where `/tmp` is read-only."* **This project already documents a condition under which `$TMPDIR`
moves.** So the first person who hits that condition, then runs `cargo bench`, gets criterion
reporting an ~8,000% regression against a baseline taken on tmpfs — and will go looking for a
performance bug that does not exist.

The README currently says only *"The baseline is local to your machine and is not committed."* **True,
and not enough**: the hazard is not only across machines, it is across `$TMPDIR` settings on one
machine.

## 4. What to add

In `tools/benchmarks/README.md`, under Running or Scope:

- **A criterion baseline is only comparable to runs with `$TMPDIR` on the same filesystem.** Name the
  measured spread — roughly `200 µs` on tmpfs against `17–23 ms` on btrfs for the same commit — so a
  reader recognises the shape when they see it rather than filing a bug.
- **Why**: the timed region includes the durability work `commit` performs, so the backing
  filesystem's `fsync` behaviour dominates.
- **The practical instruction**: pin `$TMPDIR` deliberately when comparing runs, and discard the
  stored baseline after changing it.

**Use your own measured numbers, not mine.** Re-run both configurations and quote what you get; my
btrfs figure came from one run and its interval was wide (`16.8–22.8 ms`).

## 5. What this tells increment B, and why it is not increment B's problem yet

`dc59_commit_benchmark.rs`'s existing peak-RSS pass has no equivalent sensitivity — RSS is not a
function of `fsync` latency — **so migrating it is unaffected by this.** But any future proposal to
gate on a criterion timing in CI now has a named obstacle: a shared runner's `$TMPDIR` filesystem is
not something this project controls. **Record that in your report so the argument exists when someone
proposes the gate**, rather than being re-derived then.

## 6. Gates and reporting

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9 against your final commit. **Note rule 9 gained a
command on 2026-09-03** — `cargo +1.85.0 check --workspace --all-targets --locked` — precisely
because of your §4 finding; see §7 below. `mdbook build` does not apply.

Local commit on `main`; **no push.** Report to `.git-exclude/review-request/`, with your own measured
figures for both configurations.

## 7. Your §4 finding was right, and the gate set changed because of it

You reported that `cargo +1.85.0 test --workspace --locked` never compiles `[[bench]]` targets, so the
command the handoff called load-bearing proved nothing about criterion at MSRV.

**Verified by planting a syntax error in `benches/commit.rs`**: that command exits `0`, while
`cargo +1.85.0 build --workspace --all-targets --locked` and stable clippy both exit `101`.

**One correction to your framing.** You wrote that this leaves the MSRV unproven; **CI already covered
it** — `ci.yml`'s `msrv-1.85.0` job runs `cargo check --workspace --all-targets --locked`. So nothing
could have shipped broken. **The gap was that the local gate set was weaker than CI**, which is a real
problem worth closing but a different one from the one you described. `EXECUTION-ORDER.md` §6 rule 9
now carries the missing command (`7953193`).
