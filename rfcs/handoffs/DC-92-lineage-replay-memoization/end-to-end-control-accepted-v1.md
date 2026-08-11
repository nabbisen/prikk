# DC-92 — End-to-End Control Accepted

**Reviewing:** `ca7ef74` on `dc-92-lineage-replay-memoization`.

**Accepted. No further conditions. DC-92 is ready for its three-platform CI run and merge.**

## 1. My mess, first

The uncommitted probe they found on `main` was mine, and their account of it is correct.

What happened: my first `git worktree add … main` failed, because `main` is checked out in the primary
tree and cannot be added twice. The following `cd` into the missing directory failed too — and **the
rest of that command block therefore ran in the primary working tree**, so the probe edit landed in the
real checkout instead of an isolated one. I re-ran the experiment correctly in a detached worktree and
got the answer, but never noticed the primary tree was still dirty and never cleaned it.

**Stashing it rather than discarding it was the right call**, and so was leaving the unrelated DC-54
stash alone. Nothing was lost because the experiment was fully narrated in the review; but it could
have been swept into a commit, and that would have been mine to answer for.

I have dropped `stash@{0}` — my artifact, no longer needed. `stash@{1}`, the pre-existing DC-54 stash,
is untouched. Working tree clean, one worktree (the primary), verified.

The discipline I should have followed and now will: assert the worktree exists before editing anything
in it, and finish every probe session with `git status --porcelain` on the **primary** tree, not only
on the worktree.

## 2. The control does what it was asked to do

`verify_repository_detects_block_with_state_root_mismatch` builds a real layout and store and writes a
`Root` block claiming `MerkleRoot([0xEE; 32])` — shape-valid, schema-valid, wrong only in what it
claims. Built rather than byte-corrupted, for the reason the existing controls document: content
addressing turns a post-hoc corruption into a different, self-consistent object.

**I re-ran my own original probe against it.** Disabling Phase A's collection — the exact experiment
that previously left the entire workspace suite green — now **fails this test**:

```
verify::tests::verify_repository_detects_block_with_state_root_mismatch ... FAILED
```

The wiring between Phase A's collection and Phase B's verification is now covered. That is precisely
what the condition asked for, and it is verified rather than asserted.

Their own confounding check disabled the Phase B call instead — a different cut through the same wiring.
Two independent disables, both caught. Good.

**Gates re-run by me at `ca7ef74`:** fmt, clippy `--workspace --all-targets --all-features --locked
-D warnings`, `cargo test --workspace --locked`, `cargo +1.85.0 test --workspace --locked`, **614**
prikk-store tests, `git diff --check`, `cargo audit --no-fetch`, all three release-policy checks —
all clean.

## 3. Not touching `FINDINGS.md` was correct

They read the review's registration of the broader wiring-coverage gap as my act, not an instruction to
them, and left it alone. That is exactly right — the register is the architect's, and reporting a
finding and recording it are two different acts. Recording the reasoning here because getting that
boundary right consistently is worth more than any single entry in the file.

## 4. What DC-92 delivers on merge

- `verify`: **O(N³) → O(N)**, independently reproduced (ratios flat at ~2x; 46.4 s → 2.7 s at N=160).
- `seal`: **O(N²) per call → near-flat**, a cost that had never been benchmarked at all because
  DC-59's harness marks every seal as untimed setup.
- Peak memory: **599 MB → 15.1 MB** at the worst measured corner, bounded by lineage frontier rather
  than history length — a regression this increment introduced and then removed within its own cycle.
- A committed, re-runnable benchmark instrument on both time and memory axes, which is what DC-75
  failed to leave behind and why step zero could not reproduce the original measurement.
- Six unit-level controls plus the end-to-end one, each verified non-confounded by disabling the
  specific production check it targets.

## 5. Standing

- **DC-92: merges after a green three-platform CI run.** It touches filesystem-backed state.
- The broader gap — what else `verify` does that no end-to-end test would notice — stays registered and
  unowned.
- DC-91 remains proposed and awaiting the owner. Nothing else is assigned.
