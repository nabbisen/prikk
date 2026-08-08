# DC-75 Handoff v1 — Addendum 1: DC-74 merged, §5 unblocked

**Date:** 2026-08-08. **Authored by** the architect.
**Read with** `implementation-handoff-v1.md`, which stands in full apart from §1's sequencing.

## 1. The §1 hold is lifted — the whole increment is yours

Handoff v1 §1 said **"§5 implementation: wait for DC-74 to merge."** **DC-74 is merged and accepted at
`3464e2a`** (review: `.git-exclude/reviewed/DC-74-implementation-review-v1.md`). Nothing is held back.
§4's investigation still comes first — **that is a design gate, not a sequencing one** — but there is no
longer another increment in the seal path.

**DC-74's release condition remains open, and DC-75 is what discharges it.** Criterion 5 stands: the same
commit that satisfies it discharges it explicitly in `MILESTONES.md`.

## 2. What the DC-74 review found that lands on you

**Fold this in when §5 revisits `dc74_merge_execution.rs`, which it will** — merge blocks becoming
`BlockKind::Merge` changes what those tests assert.

**Negative control result:** I replaced `if !evidence.is_confluent()` with `if false` and **four of the
five refusal tests still passed.** The cause is benign and worth understanding before you change any of
them: `derive_next_state_root` runs before any write and independently refuses conflicting adoptions — I
confirmed **zero objects written** with the gate disabled, so "no object, WAL, or ref write of any kind"
holds on two independent paths.

**What is untested is the diagnostic.** Without the gate a user sees `cannot create node at
already-occupied path …` instead of `merge refused: … not confluent`. Four of five tests accept either.
**One assertion on the refusal error text naming the outcome closes it.** Recommended, not required —
and the natural moment is when you are already editing those tests.

## 3. A scenario to re-run under multi-parent, because its safety may not survive your change

I constructed a case DC-74's suite does not cover: **a baseline older than the true merge base.**
Discovery is manual, and genesis is the easiest block id for a user to obtain, so this is ordinary user
error. `candidate_blocks:169-179` enforces that the baseline is *an* ancestor of both sides, never the
*lowest*.

Under DC-74 it **fails closed** — the shared blocks appear on both sides and the algebra rejects them as
a conflicting pair (`Conflict`, `pair_conflict`), target tip unchanged, `verify` clean. **Safe by
construction, not by a check.**

**That is exactly why it may not survive DC-75.** Whatever you decide in §3 changes how a block's
ancestry is walked and what "the state derived from this block" means. **Re-run this case under your
design and report what it does** — if multi-parent traversal makes an over-old baseline merge *succeed*
where it previously refused, that is a soundness regression introduced by the record, and I would rather
you find it than I do.

## 4. Two related findings recorded, neither yours

Both are now unowned rows in `MILESTONES.md`, recorded so they are not lost, **not assigned here**:

- **`required_attestation_ids` are cleared by every ordinary `seal`** (`seal.rs:191`), while
  `branch.rs:280` preserves them on closure. Merge matches seal exactly — **DC-74 introduced nothing**.
  Note the open question in the row: no surface currently *sets* the field, so this may be latent rather
  than live, and that should be established before anyone scopes it.
- **Neither `seal` nor `merge` inspects `RefState.closed`**, so advancing a closed branch silently
  reopens it. Documented as permitted; consistent, but unreported.

**Do not absorb either into DC-75.** If your §3 work makes one of them cheap to fix, say so and it will
be scoped — the same standing rule as handoff v1 §6.

## 5. Unchanged

§2 (what this is for), §3 (answer the design question with measurements before forming a view), §4's two
constraints — do not open `Repair`/`Import` by accident, and the four fail-closed tests **change with the
reason recorded**, they are not deleted — and §5's acceptance criteria and gate discipline all stand.
