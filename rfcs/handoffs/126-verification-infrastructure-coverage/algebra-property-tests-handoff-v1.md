# Property tests for the patch algebra — implementation handoff

**Authority:** `rfcs/proposed/126-verification-infrastructure-coverage.md` §2.
**Base:** current `main` (`9491bf0`). **Under `003-landing-work-on-main.md`.**

**Scope: §2 only.** §5 (benchmarks, criterion in its own member) and the kernel doctests are separate
increments. §3/§4 shipped at `1d324a5`.

---

## 1. Do not build the property the RFC names — it cannot fail

RFC 126 §2 says:

> generate operation pairs and sequences; assert **classifier says `Commutes` ⇒ oracle states equal**

**That is already an invariant of the code, not a property of it.** `commutation.rs:13-42`:

```rust
PairClass::Independent => match prove_pair_replay(baseline, evidence, candidate_scope, left, right) {
    Ok(()) => Ok(CommutationResult::Commutes { .. }),
    …
}
other => Ok(CommutationResult::DoesNotCommute { pair_class: other }),
```

and `prove_pair_replay` returns `Ok(())` **only** when `left_then_right == right_then_left`. So
`Commutes` is unreachable unless the oracle already agreed. **A proptest asserting the RFC's sentence
would be green forever and prove nothing** — the "control passing for the wrong reason" shape, here
guaranteed by construction rather than by accident.

**I checked the obvious way it could still be unsound and it is closed**: the equality that gates
every `Commutes` is `OracleState { lifecycle, texts }`, and `NodeContent::File` carries `{ blob_id,
mode }` — so file modes *are* compared. **Do not spend the increment re-deriving that.**

## 2. What is actually untested: the refusal directions

**`DoesNotCommute` and `Unknown` never consult the oracle.** They are returned on `PairClass` grounds
alone, from the classifier's static analysis. So the classifier's conservatism is entirely unchecked:
**a pair it refuses may be one whose two orders produce identical states.**

RFC 126 §2 notes that a gap in the pairwise theory "can cause a spurious refusal safely" — true for
*soundness*, and it costs availability, and **nothing currently measures how much.**

**Property A — the one worth building.** Generate applicable operation pairs. For every pair the
classifier answers `DoesNotCommute` or `Unknown`, run the oracle anyway (both orders, compare
`OracleState`). **Report every pair where the oracle says the states are equal.**

Each hit is one of two things, and the report must say which:
- **a genuine over-refusal** — the classifier could have proven commutation and did not; or
- **deliberate conservatism** — e.g. `RenamePath`/`CreateSymlink`/same-node-different-span, which
  `types.rs::UnknownReason` defers on purpose. **These are expected hits, not failures.**

**So this property must not be a bare `assert!`.** A test that fails on the first deliberate deferral
is useless. Shape it as a **classified sweep**: run N cases, bucket the hits by `PairClass`/
`UnknownReason`, and assert only on buckets that should be empty — with the deliberately-deferred
kinds listed by name, in the allowlist-with-reasons idiom this project uses three times over
(`UNSAFE_EXEMPT_CRATES`, `DECLARED_UNDOCUMENTED`, `RFC114_ADMITTED_BUT_UNWRITTEN`).

**If the sweep finds a bucket nobody expected, that is the finding of this increment** — name it and
stop rather than widening the allowlist to make the test green.

## 3. Property B — composition, where a pairwise theory can still be wrong

`check_confluence` requires all cross pairs to commute **and then** replays both full sequence orders
and compares. `FinalStateInequality` exists precisely for "pairwise passed, composition did not."

**Generate sequences, not just pairs**, and look for the reverse: **cases where every cross pair
commutes and the two full orders still disagree.** If the pairwise theory is complete, that set is
empty. If it is not, each case is a real finding about composition — and this is the only property in
this handoff whose failure would be a correctness result rather than an availability one.

## 4. The other vacuity trap: generated operations that cannot apply

A randomly generated pair usually will not apply to a random baseline — the preconditions
(`old_blob_id`, `old_mode`, `old_span_hash`, an existing `NodeId`) will not match. **A property that
`prop_assume!`s its way past those discards most of its inputs and proves almost nothing while
looking green.**

**Generate the baseline and the operations together**, so operations are applicable by construction:
build a small lifecycle state, then derive operations *from* it (edit a node that exists, delete a
node that exists, create at a free path). `patch_replay/tests/proptest_round_trip.rs` already has
`node_id_strategy`, `repo_path_strategy`, `canonical_mode_strategy` and per-kind strategies —
**reuse them for the leaf values; do not reuse `operation_kind_strategy()` wholesale**, because it
generates operations unmoored from any state.

**Required in the report: the discard rate.** How many generated cases reached the oracle comparison
versus were discarded as inapplicable. **A property with a high discard rate is not evidence, and I
will read this number before the assertions.**

## 5. Constraints

- **No new dependency.** `proptest` is already a `prikk-store` dev-dependency; `boundary-check`
  enforces the rest.
- **Do not change the algebra to make a property pass.** If a property fails, that is a finding for a
  ruling — the classifier's conservatism is a design position, not a bug by default.
- **Deterministic seeds committed.** A proptest failure that cannot be replayed is a rumour.
  `proptest-regressions/` is tracked in this repository — if a run appends a seed, it is part of the
  commit.
- **Keep runtime honest.** Say how long the new tests add to `cargo test --workspace`; the whole suite
  runs on three platforms on every push.

## 6. Controls

1. **The discard rate** for each property (§4), as a number.
2. **Property A's classified sweep as a result**: how many pairs were refused, how many of those the
   oracle would have allowed, bucketed by reason, with the deliberately-deferred kinds named.
3. **Property B's result**: whether any all-pairs-commute sequence produced disagreeing full orders.
4. **At least one property demonstrated failing** — perturb the classifier or the oracle in a scratch
   tree, show the property catch it, revert. **A property never seen to fail is not evidence**, and
   that is doubly true here, where §1 shows the RFC's own suggested property could never have failed.
5. Runtime added to the suite.

## 7. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9 against the final commit, clippy as a single invocation
per target with the exit code captured explicitly. **Note the set grew on 2026-09-02**: it now
includes `RUSTDOCFLAGS="-D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps`.

**No CI control — that is mine at push time**, and after RFC 124 I will run it before calling this
done.

One commit on `main`, local, **no push, no tag**.
