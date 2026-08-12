# RFC (proposed) - DC-95 Verify Coverage and Finding Accumulation

**Status.** **ACCEPTED by the project owner 2026-08-11.** §3's four prerequisites precede either
stage; acceptance clears the investigation, not the implementation.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** Two findings registered during the DC-92 cycle, and the owner's stated preference for
maintainability and **verifiability of security**. The architect recommended these twice as the next
theme and then did not write the RFC; this closes that gap.
**Target milestone.** Product **M1** — `verify` is the product's central claim.

## 1. Why this ranks above tooling work

`prikk verify` is the claim. Everything else in the product — content addressing, signed envelopes,
block lineage, the durability contract — exists so that `verify` can re-derive rather than trust. Two
registered findings say that the command carrying that claim is under-instrumented and under-reported.

**Finding A — nothing proves `verify` actually state-verifies blocks end to end.** Every control for
format-2 block state calls `verify_block_v2_state`/`verify_blocks_topological` **directly**; none
reaches them through `verify_objects`. The architect disabled the wiring — DC-92's Phase A collection,
and separately the pre-DC-92 inline call — and **the entire workspace suite passed in both cases**. The
hole predates DC-92; DC-92 closed it for that one path with
`verify_repository_detects_block_with_state_root_mismatch`.

**The general question is untouched: what else does `verify` do that no end-to-end test would notice?**
A verifier whose wiring is unproven is a verifier that can silently stop verifying, and a green run
looks identical either way.

**Finding B — `verify` reports only the first hard error.** Defects propagate via `?` rather than
accumulating into the structured finding list `verify` already returns for other classes. A damaged
repository takes N runs to enumerate N defects: fix one, re-run, find the next. For the command whose
output *is* the security assurance, "here is one problem" and "here is every problem" are materially
different answers, and the first makes triage of a damaged repository iterative.

## 2. Two stages, strictly ordered, separately reviewable

**Stage 1 — coverage.** Establish, per check, whether disabling it is caught end to end. Close the gaps
that matter.

**Stage 2 — accumulation.** Change `verify` to collect findings rather than stop at the first.

**Stage 1 precedes Stage 2, and the ordering is the point.** Stage 2 changes error handling throughout
`verify`. Doing that on top of a suite that cannot detect a check silently going missing is how a
verifier loses a check during a refactor. **Stage 1 is the instrument Stage 2 is measured with.**

They are separately reviewable and must not be bundled — Stage 1's proof is "disabling each check now
fails a test"; Stage 2's is "the same defects are reported, and now all of them at once." Bundled, a
reviewer cannot tell which half a failure came from.

## 3. Blocking prerequisites

1. **Enumerate what `verify` checks, and how each is currently proved.** Work from `verify.rs` and
   `verify/`'s modules, not from the finding text. For each check: is there a test that reaches it
   through `verify_repository`, or only a unit test calling it directly? **This inventory is the
   increment** — its size decides Stage 1's size.
2. **Which gaps matter?** Not every check needs an end-to-end control. Propose a rule for which do —
   the architect's starting proposition, to accept or reject: **any check whose silent absence would
   let a repository verify clean when it should not.** That is the class Finding A is about.
3. **What does `verify` already accumulate, and why the split?** It returns structured findings for
   some classes (publication trust issues, ref divergence) and hard-errors for others. Report the
   existing boundary and whether it is principled or incidental — Stage 2 needs to know which it is
   before moving anything across it.
4. **What breaks if `verify` stops short-circuiting?** Enumerate callers and tests that depend on the
   first error being *the* error, including exit codes and any CLI output contract. If a caller relies
   on early termination for cost reasons on a damaged repository, say so.

## 4. Acceptance criteria

1. §3 answered and reported before either stage is designed.
2. **Amended 2026-08-11 after Stage 1 round 2:** the inventory's "36 rule-matching checks" was derived by
   reasoning about each check's role, and round 2 showed that reasoning can be wrong — two of its three
   checks proved **downstream-redundant** (something else catches the same defect), determinable only by
   disabling the check and looking. **36 is an upper bound, not a count**, and Stage 1 audits its own
   inventory as it goes. **Record each check's probe result** — load-bearing, or downstream-redundant and
   by which path — as a fact about today's code, not a property to rely on. Redundant checks still earn a
   regression guard on their own message (diagnostic value for an operator), labelled as such rather than
   counted as rule-matching controls.
3. **Stage 1: for every check in §3.2's class, disabling it fails at least one test that runs through
   `verify_repository`.** Demonstrated the way DC-92's controls were — disable the production check,
   observe the specific failure, restore, confirm no residual diff. **A check whose disablement is
   caught only by a unit test calling it directly does not count**; that is the exact gap this exists
   to close.
3. **Stage 1 changes no production behaviour.** Tests only. Any production change discovered as
   necessary is a finding to report, not to absorb.
4. **Stage 2: the same defects are reported as before**, plus the ability to report several at once.
   No defect class silently reclassified from error to finding, or the reverse, without it being
   stated.
5. **Stage 2 preserves fail-closed.** Accumulating findings must not turn a hard failure into a warning.
   A repository that failed verification before still fails it.
6. **What remains unproved is stated** in `verify`'s own documentation — whichever checks §3.2's rule
   excludes, and why. DC-90's criterion 5 standard: a passing run is not evidence of a guarantee it
   does not test.
7. Gate set per `EXECUTION-ORDER.md` §6 rule 9.

## 5. Non-goals

- **Any change to what `verify` checks.** Better proved and better reported, not stricter or laxer.
- **Performance.** DC-92 addressed that; this must not regress it, and the committed benchmark harness
  is available to confirm.
- **`doctor`'s repair paths.** Adjacent, separate.
- **The unowned findings this may surface.** Report them; they are registered, not absorbed.
