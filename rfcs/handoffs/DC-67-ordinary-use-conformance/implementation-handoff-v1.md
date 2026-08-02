# DC-67 Ordinary-Use Conformance Suite - Handoff

**Cleared to start.** Accepted by the project owner on 2026-08-02, at
`rfcs/accepted/DC-67-ORDINARY-USE-CONFORMANCE.md`.
**Authored by** the architect.
**Size:** medium, and almost entirely tests. No production code is expected — if you find yourself writing
some, that is a finding (see §3).
**Touches:** `crates/prikk-cli/tests/` and the store-level multi-generation helpers.

## 1. Why this exists — you found the evidence for it

Three times in four days, the defect that mattered was found by running a **sequence**, never by inspection
and never by an existing gate:

| Where | Found by |
|---|---|
| `plan_edit_text` reading an unstored `blob_id` (DC-65) | DC-59's Axis C failing for an unrelated reason |
| DC-64's incremental step, per-block `TextCache` | your own five-generation test, at generation 3 |
| DC-66's queue fold, empty `TextCache` | your own queue test |

All three are the same shape: a path that only misbehaves the **second or later** time it runs against a
given thing. This project's assurance is aimed hard at adversarial and structural failure and covers it
well. **This is the orthogonal axis.**

## 2. What to build

Ten ordinary user sequences, listed in the RFC §3, each run **through the compiled binary** at **N ≥ 3
generations** (a generation = mutate → commit → seal). Pick one N, use it everywhere, and justify it.

**Every sequence must end by deleting the worktree and rebuilding it from sealed history, asserting
byte-exact content.** This is the load-bearing technique and the reason the suite is worth writing:
`verify` passing proves history is *structurally* valid; rebuilding the content proves it is
*semantically* correct. The architect's DC-66 verification used exactly this and it is what produced
confidence nothing else could.

`prikk checkout --patch-materialize .` takes the **repository** path, not an output directory — the
architect got that wrong twice while verifying DC-66. Delete the file, materialize, read it back.

**Consolidate the CLI harness first.** DC-61, DC-65, and DC-66's test files each roll their own
`commit`/`seal`/key setup. Do not copy-paste it an eleventh time.

## 3. The rule that matters most: report, do not fix

**Every defect this suite finds gets reported as its own finding. Do not fix it inside this increment.**

A correctness fix folded into a test increment is the amendment-of-convenience this program has refused
four times. It also destroys the evidence: a suite that finds three defects and silently repairs them
looks identical to a suite that found none.

If a sequence fails, capture it, report it, and — if it blocks writing the rest — say so and stop.

## 4. Two results are both good; only one is bad

The RFC records a **prediction: this suite will find at least one further defect of this class.**

- If it finds defects, the prediction holds and each becomes a tracked finding.
- **If it finds none, say so plainly.** That is real evidence, cheaply obtained, and the suite becomes a
  permanent regression guard. Do not pad a clean result, and do not go looking for something exotic to
  justify the increment. Exotic cases are explicitly out of scope — that is DC-41's axis, and it is covered.

The only bad outcome is a suite that quietly avoids the sequences most likely to fail.

## 5. Traps

- **Fixing what you find.** §3.
- **Writing adversarial or fuzz cases.** Covered axis; not this increment.
- **Stopping at `verify`.** Structural validity is not content correctness. Criterion 2 is the rebuild.
- **Copy-pasting the CLI harness again.**
- **Testing at N = 2.** Two is the boundary that was missed originally; three shows the chain holds.
- **Claiming full coverage at the end.** Criterion 7 requires you to state what ordinary-use shapes remain
  **uncovered**. That list will not be empty, and pretending it is repeats the original error.

## 6. Definition of done

All ten sequences at the chosen N through the compiled binary; every one ending in a delete-and-rebuild
content assertion; the shared harness consolidated; every defect found reported and **not** fixed here; a
plain statement of the result including a clean one; the suite inside the ordinary
`cargo test --workspace --locked` gate or its exclusion justified with a runtime measurement; a statement
of what remains uncovered; full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9 with test counts before
and after, **commands reported verbatim**.

## 7. Standing request

You have now found three defects of this class by noticing that something failed for a reason unrelated to
what you were building, and tracing it instead of routing around it. **This increment is that instinct
turned into a gate.** If something here contradicts what the code actually does, stop and report it.
