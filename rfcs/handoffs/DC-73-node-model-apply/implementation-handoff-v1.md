# DC-73 Node-Model Operation Apply - Handoff

**Cleared to start.** Accepted by the project owner on 2026-08-04, at
`rfcs/accepted/DC-73-NODE-MODEL-APPLY.md`.
**Authored by** the architect.
**Size:** unknown until §2's first two questions are answered — plausibly small.
**Touches:** `patch_replay/decode.rs`'s materialization arms, `patch_inverse.rs`'s inverse arms, and their
deferral markers.

## 1. This is the first increment here that adds capability rather than fixing a defect

Nine increments preceded it this week; every one was a correction. **That changes what "done" means:**
there is no defect report to close against, so the acceptance criteria and the tests you write *are* the
specification. Take criterion 3 seriously — it is the only thing that will prove this worked.

## 2. Answer these before designing — the second one I would get wrong

| Question | Note |
|---|---|
| **Which of the four are reachable today?** | `ReplaceBinary` and `ChangePerm` are authored by `commit` (`node_authoring.rs:391,406`). `RenamePath` is never authored — renames become delete+create. `CreateSymlink` authoring is refused. **Confirm by attempting to author each**, do not take this from me |
| **Does `checkout --patch-materialize` already handle `ReplaceBinary`?** | **Two in-tree sources contradict each other.** DC-67 reported it failing alongside `ChangePerm`; `patch_replay.rs:5` says `ReplaceBinary` *is* reconciled. **Settle it by running a checkout against a real binary edit.** Reading will not resolve a contradiction between two readings |
| **What does inverting `ChangePerm` need?** | The prior mode must come from somewhere — tombstone, baseline state, or the operation. Decides whether inverse is a small addition or needs newly recorded data |
| **Is `RenamePath` inverse meaningful when renames are never authored?** | If nothing produces it, its inverse is dead code with a maintenance cost |

## 3. Scope follows reachability

**Do the reachable pair first**: `ChangePerm` and `ReplaceBinary`. Repositories containing them **exist
today** and cannot be fully checked out or rolled back — that is live user harm, not a hypothetical.

`RenamePath` and `CreateSymlink` have dead apply paths because nothing authors them. Implementing their
apply adds untested code reachable only by a future increment.

**Criterion 4 is not optional either way.** If you leave them unimplemented, **their marker must stop
saying "pending node model"** and say they await an *authoring* path. A stale deferral naming the wrong
blocker is precisely how "increment 4.4" came to look like one increment when it is two unrelated
questions — and it cost me a wrong framing in the roadmap that opened this.

## 4. Traps

- **Trusting my §1 correction without checking.** I already corrected the roadmap once here
  (lifecycle-state apply is complete for all seven; the gap is materialization and inverse). That
  correction is itself a reading.
- **Implementing all four because the marker lists four.**
- **Testing that it compiles or that `verify` passes.** Neither shows a mode bit survived a round trip.
  Criterion 3: rebuild the worktree from sealed history and assert byte-exact content **including the
  mode**.
- **Leaving a stale marker.** §3.
- **Widening into symlink or rename *authoring*.** Both are explicit non-goals with their own reasons.

## 5. Definition of done

§2's four questions answered and reported before design; `ChangePerm` and `ReplaceBinary` materializing on
checkout and inverting for rollback, tested end to end through the compiled binary at N ≥ 3 sealed
generations with a delete-and-rebuild content assertion including mode; `RenamePath`/`CreateSymlink` either
implemented with a stated reason or left with a **corrected** marker; `MILESTONES.md`'s checkout-replay row
and `docs/src/guide/rollback/`'s limitations updated to match what now works; no format change, no new
operation kind, no identity movement; full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9 with test
counts before and after, **commands verbatim**.

## 6. Standing request

Every blocking finding this week came from running something rather than reading it — and one came from two
documents disagreeing, which is exactly §2's second question. If something here contradicts what the code
actually does, including anything I have asserted, stop and report it.
