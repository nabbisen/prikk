# DC-92 — Prerequisite Investigation Ruling v1

**Reviewing:** `.git-exclude/review-request/prikk-dc-92-prerequisite-questions-v1.md`, and the harness
at `98b6c12` on `dc-92-lineage-replay-memoization`.

**Investigation accepted. Cleared to design**, with one condition (§4) and one correction to my own
record (§2).

## 1. Verified

**Step zero handled correctly.** DC-75's measurement left no reproducible harness, so byte-for-byte
re-running was impossible. Building a new instrument and claiming only that the *shape* reproduces —
2.19x → 2.48x → 3.27x → 4.73x → 6.33x against the finding's 2.08x → 2.26x → 3.04x → 4.53x → 6.39x —
while stating plainly that 46.4 s versus 34.2 s at N=160 is hardware and methodology, is the honest
handling. **"Confirmed the same shape, not confirmed identical"** is exactly the right claim.

**The harness itself is sound.** `#[ignore]`d per DC-59's precedent; churn holds live tree size fixed
using DC-69 Axis D's technique so sealed-history depth is the only variable; timing wraps the CLI
invocation only. Reusing an existing isolation technique rather than inventing one is right.

**Placement of `benchmark-report-v1.md` under `rfcs/handoffs/` is correct.** I checked precedent before
raising it: DC-59's and DC-69's benchmark reports live there too. Durable measurement evidence produced
*by* an increment is not the same thing as a review-request package, and I nearly mis-flagged it.

## 2. Correction — to my record, not theirs

**§4.2 catches an error of mine, and it has been sitting in the register since 2026-08-08.**

`FINDINGS.md`'s O(N³) row, and DC-92 §3 which inherited its wording, both say: memoize
`walk_lineage_to_genesis` and reuse accumulated state across `verify_v2_lineage_roots`'s loop, *"which
the code's own shape suggests drops this to O(N)."*

**That is wrong, and their §4.2 shows why.** The inner fix drops **`derive_next_state_root`** from
O(i²) to O(i). It does not drop **`verify`** to O(N), because `verify`'s outer per-object loop makes N
such calls, one per block at its own depth — summing to **O(N²)**. Reaching O(N) requires the outer loop
to reuse state as well.

Their framing — one memo table keyed by block id, written as each block passes its own check and
consulted by both the inner lineage walk and the outer per-object loop, because both are asking the same
question from different entry points — is correct and is the shape to design toward. **Two layers, one
structure.** The register is corrected alongside this ruling.

## 3. Seal: hypothesis confirmed, with one honest qualification

1.07x → 1.55x → 2.09x → 2.88x → 3.54x across five doublings. Monotonic, well above the ~2x an O(N) cost
would settle at, nowhere near flat. **§2's hypothesis is confirmed: seal is superlinear in
sealed-history length**, and NFR-PERF-01's evidence has a real blind spot, since DC-59's harness marks
every seal as untimed setup.

**One qualification, non-blocking.** At `SAMPLES = 2`, "converging toward the 4x quadratic signature" is
a stronger claim than two samples per point support — 3.54x does not distinguish quadratic from, say,
N^1.8. What the data *does* establish robustly is the monotonic climb across five doublings, and that is
enough to act on; the exact exponent does not change what to build. Say "superlinear, consistent with
quadratic" rather than "converging on 4x" when this is quoted onward. Their sample count is justified
and I am not asking for more runs.

Reporting the NFR-PERF-01 consequence without re-scoping the requirement was correct. That remains the
owner's.

## 4. Condition: the stated invariant is narrower than the behaviour it must preserve

§4.5 states the memo invariant as: an entry may be written only as a byproduct of that block's patches
having been *replayed against its parent state and compared to its recorded `state_merkle_root`*.

**That is true and it is incomplete.** I went looking for a divergence between the two entry points and
did not find one — `validate_v2_lineage` calls `validate_block_v2_shape` for every lineage member
(`block_state.rs`, in the walk), and `verify_block_v2_state` calls it directly. **Both paths validate
shape today; my first reading that they differed was wrong.**

The risk is therefore not that the paths diverge. It is that **an implementation faithful to §4.5's
stated words could satisfy them while dropping shape validation** — because the invariant as written
mentions only replay-and-compare, and shape validation is the other thing every path currently performs.
`validate_block_v2_shape` is what rejects a `Merge` block without exactly two parents, a `Normal` block
without exactly one, and the unauthorized `Repair`/`Import` kinds. A memo that means "state verified"
must never be read as "block verified."

**Required:**

1. **Restate the invariant to cover everything the current path checks** — shape validation and schema
   version as well as replay-and-compare — not only the state-root comparison.
2. **Extend acceptance criterion 3 with a fourth negative control: a *shape* violation**, injected at a
   position reached as a lineage member, not as the outer loop's primary subject. The three state-root
   corruptions already required cannot catch a dropped shape check — they corrupt a different thing.

This is small and it is the difference between a memo that caches a result and a memo that quietly
narrows what verification means.

## 5. Accepted as reported

- **§4.3's trust argument.** Per-invocation, constructed empty, discarded on return — nothing between
  runs to tamper with or go stale, so NFR-PERF-04 is not engaged rather than being satisfied. Correct,
  and correctly not reaching for anything persisted.
- **§4.4's distinction.** The inner fix serves all three callers automatically; only `verify` has an
  outer loop, so only `verify` needs the outer half. Explicitly rejecting the flatter "one fix, three
  callers" framing was the right call — the flatter version would have understated what `verify` needs.
- **§4.2's honesty about the residual.** A per-invocation memo cannot help a *later* invocation, so
  repeated seals over a growing history stay O(i) each. Naming that limit rather than implying the fix
  is total is the right disposition, and closing it would need a persisted cache, which nobody is
  proposing.

## 6. Standing

- **DC-92: cleared to design and implement**, under §4.
- Green **three-platform** CI before merge — the implementation touches filesystem-backed state.
- Criterion 2's before/after curve is measured with the harness now committed on the branch, which is
  the right outcome of step zero: the next person to ask this question will not have to rebuild it.
