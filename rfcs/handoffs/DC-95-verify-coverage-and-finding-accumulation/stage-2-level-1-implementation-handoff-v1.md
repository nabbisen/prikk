# DC-95 Stage 2, Level 1 — Implementation Handoff v1

**Cleared to implement Level 1 only.** Design accepted by the project owner 2026-08-12,
`handoffs/DC-95-verify-coverage-and-finding-accumulation/stage-2-design-v1.md`. **Level 2 (item
containment inside `verify_objects` and `verify_refs`) is not authorized and must not begin.**

## 1. Read first, in this order

1. `stage-2-design-v1.md` — what and why.
2. `stage-2-prerequisite-3.3-3.4-ruling-v1.md` — the four binding constraints.
3. `stage-2-handoff-v1.md` §2 — the seven checks with no end-to-end test.
4. `verify.rs`'s own module doc — Stage 1's classification table and the upstream-gate rule.

## 2. Step 0, before writing any production code: falsify §4

The design's stage-dependency graph is **my reading of the current pipeline, not a derived fact.** It is
the single input the whole `NotEvaluated` model rests on.

**Derive it from the code independently and report it before implementing.** For each of the twelve
stages: what does it consume, what produces that, and what genuinely cannot run if the producer fails?

**§4 being wrong is a successful outcome, not a setback.** My framings have been narrower than this
codebase three times in the last week — RFC 101's problem statement, RFC 102's §3, and DC-95's own shape
count, each corrected by exactly this kind of independent derivation. Assume the same here.

Specifically doubted, because I asserted them from a single read:

- That `verify_objects` failing blocks nothing downstream. It feeds `signature_issues` and
  `merge_baseline_divergences`; confirm nothing else reads its output.
- That the per-envelope schema/trust loop depends only on `verify_refs`. `trust_verifier` is constructed
  before `verify_objects` and mutated by it — check whether a failed object pass leaves it in a state
  that makes later `verify()` calls meaningless rather than merely absent.

That second one is the shape of defect this whole increment exists to catch: **a stage that runs, reports
nothing, and is not actually evaluating anything.**

## 3. The invariant that is not negotiable

> **No stage may be silently absent from the report.**

Every stage resolves to `Evaluated`, `Failed`, or `NotEvaluated`. **`NotEvaluated` is blocking and names
the dependency that failed.**

**This is not deferrable to Level 2 and not descopable under pressure.** Level 1 without it produces a
`verify` that can return a clean-looking *incomplete* report — **strictly worse than today's
short-circuit**, because today a failure is at least loud. If something in Level 1 has to give, it is not
this.

## 4. Scope — what Level 1 touches

- The twelve top-level `?` in `verify_repository`, converted to stage containment.
- One new finding type carrying scope, code, message, and a **`blocking` flag** — mirroring
  `RefPublicationIssue`, which `doctor.rs:257-261` already handles correctly. **Do not add a type whose
  severity must be decided per-field in `doctor.rs`**; that shape is what lets repair proceed against
  damage it previously refused.
- `doctor.rs` wiring: derive severity from the flag.
- `main.rs`'s `run_verify` output path.
- `--stop-on-first-error`, preserving today's bounded walk.

**Not in scope:** the checks themselves. **No check is rewritten, moved, or deleted in Level 1.** If a
check appears to need conversion, that is a finding to report, not a change to make.

## 5. Do not disturb

- **Pipeline order between `classify_ref_state` and `require_retained_evidence`** — ruled load-bearing.
  Reclassification runs *after* the pushes it rewrites; separating them yields a **wrong but plausible**
  issue code that no Stage 1 test would catch.
- **The three provably unreachable checks** — topological cycle, duplicate pointer identity, duplicate
  ref-log identity. Round 6 ruled them kept, untested, with the argument recorded. **A refactor of error
  propagation is exactly when someone deletes a check that "can never fire."**
- **The four excluded non-blocking checks** and `signature_envelope_issues` staying non-blocking. If
  Level 1 appears to require changing that, stop and report — it reopens four Stage 1 exclusions.
- **`repair_repository`'s gate**, which must refuse for every defect it refuses today.

## 6. Tests

Stage 1's 641 must still pass. The ~27 asserting `Err` change **assertion shape only** — keep their doc
comments; the reachability lessons remain true.

**New, and each is an acceptance criterion:**

1. Two independent defects in **different** stages — both reported.
2. Per dependency edge: the dependent stage is `NotEvaluated` **and names the failed dependency**.
3. Per contained stage: **`repair_repository` still refuses.** The assertion is the refusal, not the
   finding's presence.

## 7. Gates and merge

The full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus a **green three-platform CI run** — this
touches filesystem-backed state.

## 8. Standing

- **A stop-and-report remains a complete outcome.** If Step 0 finds the dependency graph makes stage
  containment unsound, say so and stop.
- Level 1 merges before Level 2 is scoped. Stage 1's lesson about bundling applies inside Stage 2.
