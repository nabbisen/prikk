# DC-95 — Prerequisite Investigation Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-95-prerequisite-questions-v1.md`.

**Accepted. Stage 1 is cleared to design, sized at 36 checks.** The rule is adopted with their
refinement, and §3.4 corrects my own framing of Finding B — §3 is that.

This is the most thorough prerequisite report of the cycle. A 64-row inventory built by tracing the
call graph, with every "No" checked against the test tree by opening the file rather than inferring from
a name, and three highest-stakes rows verified independently before the rest were trusted.

## 1. Verified — including the headline, by probe

**The shape-validation claim is the one that mattered, and it holds.** I disabled
`validate_block_v2_shape` on `main` and ran the whole workspace. Four tests failed:

```
block_state::tests::format2_merge_shape_matrix
block_state::tests::format2_parent_and_kind_matrix_is_closed
block_state::tests::shape_violation_at_a_lineage_member_position_is_caught
merge_evidence::tests::multi_parent_normal_candidate_fails_before_report
```

**Every one is unit-level. Nothing in `verify::tests`. Nothing at CLI level.** So shape validation can
vanish entirely and no test reaching `verify_repository` notices — including DC-92's own
`shape_violation_at_a_lineage_member_position_is_caught`, which calls `verify_blocks_topological`
directly. **That is precisely the Finding A pattern, still live, in the check closest to the defect that
started this.** Their choice to verify this row first was the right one.

The other independently-checked claims hold too: zero `checksum`/`corrupt` matches in `wal/tests.rs`; no
test anywhere references `ensure_ref_target_valid`; `print_verify_report` loops over ten report vectors
unconditionally; and `run_verify`'s failing predicates are exactly the eight listed.

## 2. The rule, adopted as refined

I proposed: *any check whose silent absence would let a repository verify clean when it should not.*
They accepted it and clarified that **"verify clean" must mean "fails to reject a repository that should
be rejected," not "loses any reported detail."**

**That refinement is right and I am adopting it.** Read literally, my wording swept in non-blocking,
warning-level findings whose loss makes a passing report less informative without making a failing
repository pass — which is not what the rule was for, and would have inflated Stage 1 past its own
intent.

**And they operationalised it correctly rather than by intuition**: a row counts as blocking if its
issue is reached by one of the `has_*` predicates `run_verify` actually treats as failing. I confirmed
those eight, and confirmed that **none covers `signature_envelope_issues`** — so excluding those four
rows is right by current behaviour. It is also right by *recorded intent*: `FINDINGS.md` has carried
"Signature envelope canonicalization incomplete — Non-blocking — DC-39" since the original architect
review. The exclusion rests on a decision, not an accident.

**Worth knowing rather than burying:** the corollary is that `verify` prints signature-envelope problems
and still exits 0. That is the recorded position, not a defect found here, and it is not DC-95's to
change — but it should be visible rather than implied by an exclusion.

Arithmetic checks: 20 + 8 + 36 = 64; the 44 non-"Yes" rows split 36 + 7 + 1. **Stage 1 is 36 checks —
30 needing a new fixture, 6 needing only a stronger assertion.**

## 3. Correction: §3.4 is sharper than my RFC, and it splits Stage 2 in two

I wrote Finding B as *"`verify` reports only the first hard error."* That was too coarse, and their trace
shows why:

- **The accumulated `Vec` reporting already works multi-category.** `print_verify_report` prints every
  entry of every category unconditionally, in one pass. Nothing needs fixing there.
- **The real gap is that any hard `Err` aborts `verify_repository` via `?` before that reporting code is
  reached at all** — so the other categories are never even computed, let alone printed. Not a reporting
  limitation; an early return.
- **And there is a second, independent gap I had not identified**: `run_verify` walks a **fixed priority
  chain** of eight `has_*` predicates and returns the *first* match as the process error string — even
  when `print_verify_report` has already printed several categories in full moments earlier.

**So Stage 2 has two pieces, not one**, and they are separable: converting hard errors into accumulated
findings, and making the CLI's own exit message reflect everything found rather than the
highest-priority one. Recorded because my framing would have led to fixing one and calling it done.

Their point that no test pins the priority-order behaviour — and their refusal to call that "safe to
change" rather than "not found to be tested" — is the right distinction.

## 4. §3.3, accepted with its honesty intact

The boundary they describe is real: structural/decode failures hard-error because verification of that
item cannot proceed; classified states of an object that does decode accumulate. **And they did not
force the pattern to be uniform** — the `lifecycle_cache` replay-error-to-finding conversion is named as
a deliberate per-check inversion, and `PRIKK-TRUST-POLICY-INVALID` is flagged as not obviously fitting,
with "flagging rather than guessing" stated outright.

**The conclusion that follows is the important one and I endorse it:** Stage 2 must convert **per
check**, not per category. Blindly accumulating a failure that genuinely leaves nothing further to check
for that item would have `verify_repository` carry on with insufficient context — a different and worse
defect than the one being fixed.

## 5. Stage 1: cleared, with two leads and one ordering

**Cleared to design.** Two leads, offered as leads:

**5.1 — Consider a table-driven harness rather than 36 bespoke tests.** DC-41's failpoint matrix is the
precedent already in this codebase: one entry per failure mode, each with its injected defect and
asserted outcome. Thirty hand-written fixtures will drift and will be tedious to keep honest; a table
whose rows are the inventory's own rows will not. **Not a ruling** — if the checks are too heterogeneous
to table, report that.

**5.2 — Do the 8 shape-validation arms first.** They are the direct analog of the defect that started
this increment, and §1's probe shows they are still unprotected. If Stage 1 were somehow cut short, that
is the part worth having.

**The bar per check is DC-92's**: disable the production check, observe a specific failure in a test
that reaches it through `verify_repository`, restore, confirm no residual diff. **Asserting `.is_err()`
alone does not clear it** — that is exactly what makes the 8 Partial rows partial, and repeating it
would rebuild the problem.

## 6. Standing

- **Stage 1: cleared to design.** Stage 2 stays behind it, now scoped as two pieces per §3.
- Green **three-platform** CI before any merge — this touches `crates/prikk-store`.
- DC-93 and DC-94's prerequisite reports are under review separately.
