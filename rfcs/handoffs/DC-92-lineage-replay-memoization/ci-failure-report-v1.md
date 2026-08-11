# DC-92 — CI Failed. Branch Does Not Merge.

**Run:** `31477781436`, head `ca7ef74`. **Two jobs failed:** `non-linux build (macos-latest)` and
`non-linux build (windows-latest)`. Everything else passed, including `macOS mutation test suite` and
both read-only conformance jobs.

`main`'s own run (`31477780240`) is green and unaffected.

## 1. The failure

Eight errors, identical on both targets, all in `crates/prikk-cli/tests/dc92_lineage_replay_benchmark.rs`:

```
error: unused import: `prikk`                              (line 42)
error: constant `MEMORY_SAMPLE_INTERVAL` is never used     (line 224)
error: constant `MEMORY_DEPTH_TREE_SIZE` is never used     (line 228)
error: constant `MEMORY_DEPTH_VALUES` is never used        (line 229)
error: constant `MEMORY_TREE_DEPTH` is never used          (line 233)
error: constant `MEMORY_TREE_VALUES` is never used         (line 236)
error: constant `MEMORY_FLOOR_TREE_SIZE` is never used     (line 238)
error: constant `MEMORY_FLOOR_SAMPLES` is never used       (line 239)
```

**Cause.** The memory axis measures peak `VmHWM` by reading `/proc/<pid>/status`, which is Linux-only,
so its functions carry `#[cfg(target_os = "linux")]` (lines 246, 253, 278, 317, 327). Their supporting
constants and the `prikk` import do not. Off Linux the functions vanish, the constants and import
become unused, and `-D warnings` turns that into a build failure.

**Fix: gate the constants and the import the same way their consumers are gated.** Test-only,
production code untouched, and it does not affect any measurement — the memory axis remains Linux-only
by nature, which is fine and matches DC-62's own harness.

## 2. Why no local gate caught it — and this is on both of us

`EXECUTION-ORDER.md` §6 rule 9, **as amended 2026-08-09**, requires cross-target clippy for
`x86_64-pc-windows-gnu` and `x86_64-apple-darwin` on any increment touching `#[cfg(target_os)]` code.
That gate reproduces this failure exactly. It was not run.

**Their side:** the gate summaries for `c0f3734`, `5eee2de`, `4bb851d`, and `ca7ef74` each state "No
`#[cfg(target_os)]` code touched." That was true up to `98b6c12` and **false from `c0f3734` onward** —
`git log -S 'cfg(target_os = "linux")'` puts its introduction squarely in the memory-axis commit. The
claim was carried forward across three subsequent rounds without being re-checked against what had
changed underneath it.

**My side, and it is the same mistake one level up:** I re-ran gates myself at all four of those commits
and did not run cross-target clippy at any of them — because I read their "no `cfg` code touched" line
and accepted it. For DC-88 and for DC-92's first implementation round I *did* run it, precisely because
their reports said cfg code was touched. **So I applied the gate conditionally on their assertion
rather than checking the condition myself**, which is the one thing my role exists not to do. A
`grep -rn "cfg(target_os" ` on the diff would have taken seconds and I never ran it.

**A third error, mine, in this very investigation:** my first reproduction ran
`cargo clippy … --target $t` **without `-- -D warnings`** and reported clean. It was not clean; warnings
simply were not errors. I caught it, re-ran with the actual rule-9 command, and reproduced all eight
errors on both targets. Recorded because reporting "I reproduced nothing" from a mis-typed gate command
would have sent this back to you as a CI-only mystery.

## 3. Required

1. **Gate the seven constants and the `prikk` import** with `#[cfg(target_os = "linux")]`.
2. **Run rule 9's cross-target clippy** — with `-- -D warnings` — before resubmitting:
   ```
   cargo clippy --workspace --all-targets --all-features --locked --target x86_64-pc-windows-gnu -- -D warnings
   cargo clippy --workspace --all-targets --all-features --locked --target x86_64-apple-darwin -- -D warnings
   ```
3. **Correct the gate summary's claim.** The harness has contained `#[cfg(target_os)]` code since
   `c0f3734`; the cross-target gate has applied to every round since and should be reported as run, not
   as inapplicable.

Nothing about the accepted work changes. The memoization, the topological bound, the seven controls,
and both measurement axes all stand — this is a test-harness portability defect on top of them.

## 4. Standing

- **DC-92 does not merge** until this is fixed and the run is green on all eight jobs.
- No re-review of the accepted rounds is needed; the fix is test-only and the condition is mechanical.
- I will run cross-target clippy myself on the resubmission, and from here on I check whether the
  condition holds rather than reading whether it was claimed.
