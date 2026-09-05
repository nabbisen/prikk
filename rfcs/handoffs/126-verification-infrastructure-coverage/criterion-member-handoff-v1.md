# RFC 126 §5 increment A — a benchmark member that criterion can actually live in

**Authority:** `rfcs/done/126-verification-infrastructure-coverage.md` §5, under **§6's ruling
(owner, 2026-09-01): option 4** — criterion in its own workspace member, outside `default-members`.
**Base:** current `main` (`7087428`). **Under `003-landing-work-on-main.md`.**

**Scope: stand up the member, with one real benchmark and the gate changes it forces.** Migrating
`dc59_commit_benchmark.rs` (1,064 lines) and `dc92_lineage_replay_benchmark.rs` (718 lines) is
**increment B** — do not touch either file here.

---

## 1. Read this first: four things I hit building a throwaway probe of this exact member

I built the member in a detached worktree before writing this, because §5 looked cheaper than it is.
**All four below are verified, not predicted.** They are why this handoff exists rather than a
one-line instruction to add criterion.

### 1.1 criterion 0.8 cannot be used — pin `0.7`

`criterion 0.8.x` declares `rust-version = 1.86`. **This workspace's MSRV is `1.85.0`**, and
`cargo +1.85.0 test --workspace --locked` builds every member, so 0.8 breaks the MSRV gate outright.

**`criterion 0.7` declares `1.80`.** I ran `cargo +1.85.0 check --workspace --all-targets` with 0.7
and a real bench target present: **clean.** Pin `"0.7"`.

**Do not "upgrade" it.** If a future increment wants 0.8, that is an MSRV rise and goes through
`rfcs/handoffs/119-release-policy-reset/msrv-rise-policy-and-gate-handoff-v1.md`, not through this
one.

### 1.2 `criterion_group!` cannot satisfy `missing_docs`

The workspace lint table sets `missing_docs = "warn"`, and the clippy gate is `-D warnings`. **The
macro expands to an undocumented function**, so the error comes from inside the expansion and no
amount of `///` on your own code fixes it:

```
error: missing documentation for a function
   --> tools/benchmarks/benches/<name>.rs
    = note: this error originates in the macro `$crate::criterion_group`
```

**Ruling: keep `[lints] workspace = true` on the member and put `#![allow(missing_docs)]` at the top
of each bench file, with a comment saying why.** Verified: clippy exits 0 with that, and every other
workspace lint — including AUD-06's newly-denied `unwrap_used`/`expect_used`/`indexing_slicing` —
still applies. **Do not drop `[lints] workspace = true`** to make this go away; that would silently
exempt benchmark code from all of them.

### 1.3 Adding any member fails `boundary-check`, in two places at once

```
"category": "workspace-members", "detail": { ... "prikk-benchmarks": "tools/benchmarks/Cargo.toml" ... }
```

`boundary-check` pins the expected member set, and **the same expectation is asserted as a unit test**
— `boundary::tests::workspace_and_product_boundaries_hold` (`tools/release-policy/src/boundary/tests.rs`),
which was the single failure in my probe's otherwise-clean MSRV run.

**Both must be updated deliberately**, and that visible edit to a reviewed constant *is* the control —
the same idiom as `UNSAFE_EXEMPT_CRATES` and `DECLARED_UNDOCUMENTED`. **`PRODUCTS` must not change:**
the new member is not a product, and `placement.rs` iterates `PRODUCTS`, which is exactly why
criterion cannot leak into the dependency-placement surface.

### 1.4 The dependency tree grows by 35 crates

180 → 215 packages in `Cargo.lock`. **`cargo audit --no-fetch` reports no advisories against the new
tree** — I checked. Report the count you get; if it differs from 35, say so.

## 2. What to build

**`tools/benchmarks`, package `prikk-benchmarks`**, mirroring `tools/release-policy`'s manifest shape:
`publish = false`, every `*.workspace = true` field, `[lints] workspace = true`.

**In `[workspace] members`, and NOT in `default-members`.** That is §6 option 4's whole point: it
keeps criterion out of every product manifest and out of the shipped graph. **It does not keep it out
of build time** — `cargo test --workspace` still builds it, exactly as it already builds
`tools/release-policy`. Say that plainly in your report; it is the honest cost of the ruled option.

**Dependency versions: literal, as `tools/release-policy` does** (`criterion = "0.7"`), **not
`{ workspace = true }`.** Keeping criterion out of the root `[workspace.dependencies]` table means no
product crate can reach it by name at all. **Check whether any gate objects to that and report** —
I found none, and `tools/release-policy` sets the precedent with eight literal versions, but this
cuts against the convention that product crates use the root table, so confirm rather than assume.

**One benchmark, not a suite.** The `commit` path is the right first subject — it is what
`dc59_commit_benchmark.rs` exists to measure and what the two performance walls in `ROADMAP.md`
concern. Build the smallest fixture that produces a stable number; **you may read
`dc59_commit_benchmark.rs` for its fixture-construction approach, but do not move or modify it.**

**A short `README.md` in the member** saying how to run it (`cargo bench -p prikk-benchmarks`), that
it is not in `default-members`, and that a baseline is stored by criterion between runs.

## 3. What this does not achieve — and must not be reported as achieving

**§5's stated motivation is that "DC-62's peak-RSS work can regress invisibly". Criterion does not
measure peak RSS.** It measures wall-clock time against a stored baseline. **This increment does
nothing for the RSS axis**, and a report implying otherwise would be wrong. That gap should be named
in your report so §5 is not marked complete on a false premise.

**No CI job.** Standing the member up and wiring a job that fails a build on a timing regression are
different decisions, and the second needs a conversation about flakiness on shared runners. Not here.

## 4. Gates and reporting

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9 against your final commit — **not reproduced
here**: `reference-check` rejects a policy-command line outside its registered sites.

**Run `cargo +1.85.0 test --workspace --locked` and treat it as the load-bearing one.** It is the gate
this increment is most likely to break, and 1.1 is the reason.

**`cargo bench -p prikk-benchmarks` is not part of the gate set** and must not become part of it in
this increment. Run it once to prove the benchmark executes, and report the output.

Local commits on `main`; **no push, no tag, no publish.** Report to `.git-exclude/review-request/`,
and state:

1. The `Cargo.lock` package count before and after.
2. Both boundary-gate edits, and confirmation that `PRODUCTS` was not touched.
3. Whether any gate objects to literal dependency versions in a `tools/` member.
4. That criterion is `0.7`, and the MSRV run's result.
5. That this increment does nothing for peak RSS.
6. Every place this handoff's claims proved wrong. **The +35, the two boundary sites, and the
   criterion version behaviour are from one throwaway probe of mine, not from a full build of the
   thing you are about to build.**
