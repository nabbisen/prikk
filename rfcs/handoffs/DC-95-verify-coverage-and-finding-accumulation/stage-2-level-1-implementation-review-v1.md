# DC-95 Stage 2, Level 1 — Implementation Review v1

**Reviewing:** `3516819`…`48e3e50` on `dc-95-stage2-level1-scope-containment`, baseline `9c9babd`.

**Accepted with one required fix (§4).** The central invariant holds and I probed it rather than
reading it. The round also found an error in a graph **I declared authoritative**, which is §2.

## 1. Verified

- **`trust_is_valid` defect fixed** — `verify.rs:848`: `objects_evaluated && trust_verifier.issues
  .is_empty()`. The ruled correctness defect is closed.
- **Counts are `Option`**, 13 fields, `None` rather than `0` when the producing stage did not evaluate.
- **`doctor` maps every non-`Evaluated` outcome to `DoctorIssue::error`** (`doctor.rs:210`,
  `PRIKK-DOCTOR-VERIFY-STAGE-INCOMPLETE`), preserving the repair gate without a per-field decision.
- **`StageStatus::is_blocking`** returns true for anything but `Evaluated` — `NotEvaluated` is blocking,
  as designed.
- **Gates at `48e3e50`:** fmt clean, clippy **0**, workspace tests green, **647** prikk-store tests
  (641 → 647). Worktree removed, primary tree clean.

## 2. Step 0's table was wrong, and I am the one who declared it authoritative

They found `CommitIndex` was never actually contained: `commit_index::verify_divergence` is
`Result<Vec<CommitIndexDivergence>>` (`commit_index.rs:134`), not the bare `Vec` Step 0's table recorded.
Only `lifecycle_cache::incremental::verify_divergence` (`incremental.rs:62-65`) is genuinely infallible.
Both confirmed.

**My Step 0 ruling said: *"Design §4 is superseded by their §1 table. Not amended in part — replaced."***
That was the error. I had just finished being corrected for treating my own framing as fact, and I
responded by treating *someone else's* framing as fact — which is the same move, one level up. The right
ruling was that their table is a better hypothesis, still to be checked against the code as the
implementation touches each stage.

**The implementation caught it because writing the code forces contact with every signature.** That is a
property worth keeping in mind when weighing how much confidence a paper derivation earns.

## 3. The invariant is protected, and I probed rather than trusted

They flagged that *"nothing in Level 1's acceptance criteria as written would have caught it without
deliberately testing `CommitIndex` specifically."* **That is now false, and the reason is a decision they
made.** I reverted the fix — `let commit_index_divergences = verify_divergence(layout)?;` — and ran the
suite:

```
verify_repository_marks_every_wal_replay_dependent_as_not_evaluated ... FAILED
verify_repository_with_options_halts_every_later_stage_when_stop_on_first_error_is_set ... FAILED
```

**Two tests catch it, because they assert over the whole stage map rather than per stage.** A stage that
escapes containment is missing from the map, and a whole-map assertion notices. Restored, no residual
diff.

**Keep that pattern for Level 2.** Whole-map assertions are what make "no stage may be silently absent"
testable rather than aspirational; per-stage assertions would have left exactly the hole they worried
about.

## 4. Required: `blocked_by` makes a false claim under `--stop-on-first-error`

`StageStatus::NotEvaluated { blocked_by }` documents its field as *"the earlier stage whose own
non-evaluation is why this one could not run"* (`verify.rs:290-293`) — a **dependency** claim.

Under `--stop-on-first-error`, every later stage is marked `NotEvaluated { blocked_by: <failed stage> }`
**regardless of whether any dependency exists.** So `LifecycleCache`, which depends on nothing, reports
`blocked_by: Objects`. That is not why it did not run; it did not run because the operator asked the walk
to stop.

**This is the increment's own thesis turned on itself: reporting a reason that is not the true reason.**
It is not blocking-incorrect — both states block — and the flag is opt-in. But `blocked_by` is
machine-readable, and anyone reconstructing the dependency graph from a report would get it wrong.

**Required:** distinguish halted-by-request from dependency-blocked. A separate status
(`Halted { after: VerificationStage }`) is cleaner than a comment, because the field is structured data
and will be read as such. Small change; the tests already assert over the whole map.

## 5. Two findings they surfaced, one of which is evidence for the change itself

- **The signature-envelope merge-order regression they introduced and caught themselves**, via an
  existing byte-preservation test. Introduced and fixed within the round, reported rather than quietly
  corrected. That is the right disclosure standard.
- **`doctor_refuses_missing_main_ref_pointer_reconstruction` was not testing what its name claims** — an
  arbitrary `state_merkle_root` tripped the `Objects` stage first, so the scenario was never reached.
  **Short-circuiting was hiding it, and containment exposed it.**

**The second is evidence for Stage 2, not just a fix.** Stage 1 built an instrument for detecting checks
that silently stop running; Stage 2's first implementation round found a *test* that had silently stopped
testing. Neither stage would have found it alone.

## 6. On the package

No `patches/` directory this time — evidence and report only. I reviewed from the branch directly, so
nothing was blocked, but prior rounds included the diffs and it is the cheaper path for a reviewer.
Restore it next round.

## 7. Standing

- **Level 1: accepted**, subject to §4.
- **Green three-platform CI required before merge** — filesystem-backed state, whole branch.
- **Level 2 remains unauthorized.** It is scoped but not cleared; item containment inside
  `verify_objects` and `verify_refs` needs its own handoff.
- The module-doc rule landed with Level 1 as ruled.
