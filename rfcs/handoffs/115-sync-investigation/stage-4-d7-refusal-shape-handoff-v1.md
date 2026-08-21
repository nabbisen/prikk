# RFC 115 Stage 4 — D7: seal what is missing, refuse only what is absent

**Design:** `rfcs/handoffs/115-sync-investigation/design-v1.md` **§12 (D7). Read it in full — it states
the principle, and §12.2 says why this is correctness rather than convenience.**
**Origin:** `.git-exclude/reviewed/RFC-116-claim-design-stress-round-v1.md` §2.
**Base:** current `main`. **Precedes RFC 116 stage 2.**

**This amends behaviour in a merged, reviewed increment**, on the owner's ruling. It also **inverts two
shipped tests**. Read §3 before touching them, or you will restore the defect.

---

## 1. What is wrong today

`seal_from_accepted_claim` refuses when a claim names both already-sealed and not-yet-sealed patches:

```
"recognition claim {id} names {n} already-sealed patch(es) and {m} not-yet-sealed patch(es)
 -- refusing a partially-applied seal"
```

**That condition is reachable by ordinary use, and when it fires the unsealed patches become
permanently unreachable.** `merge_execute.rs:173` sets a merge block's `patch_ids =
adopted_patch_ids` — patches **already sealed on the other branch in the same repository** — so one
patch is described by two claims. A receiver that sealed the first claim, then meets a second naming
that patch plus a new one, refuses forever, and no other claim covers the new patch.

## 2. The change — D7's three-state rule

For every patch a claim names, classify its state **in this repository**:

| State | Outcome |
|---|---|
| **Sealed** (present, reachable from a ref tip) | **skip** — its effect is already in this repository's state |
| **Present, unsealed** | **seal** — this is the work |
| **Absent** | **refuse the whole claim** — unchanged |

> **The operation supplies exactly those patches whose effect is missing and which are available to
> supply. Absence is the only refusal.**

### 2.1 Concretely

- **Delete the `sealed_count != 0` refusal** (`seal_from_accepted.rs:136-142`).
- **The block gets the unsealed subset, not the whole claim.** `seal_from_accepted.rs:213` currently
  passes `claim.patch_ids.clone()`. It must pass **the claim's own sequence filtered to the unsealed
  patches, order preserved.** D6 gives the claim a total order; restricting a total order to a subset
  is well-defined, so no ordering decision is needed here — **do not sort, do not dedup, do not
  reorder.**
- **`AlreadySealed` stays as an outcome but stops being a separate rule**: it is now the degenerate
  case where the unsealed subset is empty. Keep the variant; derive it from the filter rather than from
  its own `sealed_count == len` branch.
- **The absent-patch refusal at `seal_from_accepted.rs`'s existence loop is unchanged.** It is D7's
  only refusal and RFC 115 §8.4's no-partial-apply discipline is untouched.

### 2.2 The consequence to state in the module doc

**The sealed block may now carry fewer patches than the claim names, and that is correct.** The skipped
patches' effects are already in this repository's state, so `derive_next_state_root` over the remainder
produces a state that accounts for **all** of them. **Blocks differ between repositories; state
converges** — RFC 115 §2.4-§2.7, unchanged. Say this where an implementer will meet it, because "the
block doesn't match the claim" otherwise reads as a bug.

## 3. Two shipped tests whose meaning inverts

**Both were correct under the old rule. The rule changed.**

- **`row7_a_partially_sealed_claim_refuses`** — asserts the refusal D7 removes, including the
  `"partially-applied"` message text. **Invert it**: the same fixture must now **seal the unsealed
  remainder**, leaving the already-sealed patch alone. Name it for what it proves.
- **`row11_the_sealed_block_carries_the_claims_order_verbatim`** — the block now carries the claim's
  order **restricted to the unsealed subset**. Keep the verbatim-order property, narrow its subject.
  With nothing already sealed, the two are identical, so **add** a mixed case rather than replacing the
  simple one.

**Must still hold, unchanged:** `row9_sealing_the_same_claim_twice_is_a_no_op_the_second_time` (now the
degenerate case of the same rule), `the_no_op_path_is_byte_identical_even_for_an_unadopted_signer`
(§4 below), `row6` (absent patch refuses), and rows 1-5, 8, 10.

## 4. The no-op path's trust ordering must survive

`verify_signer_trusted` is deliberately **after** the no-op determination, because that path writes
nothing — ratified in the Stage 4 review §3, and pinned by
`the_no_op_path_is_byte_identical_even_for_an_unadopted_signer`.

**Re-deriving `AlreadySealed` from the filter must not move that check earlier or later by accident.**
The invariant is: **an unadopted signer meeting a fully-sealed claim still performs no trust-gated act
and writes nothing.** That test must keep passing; if your restructuring makes it fail, the
restructuring is wrong, not the test.

## 5. The motivating case must be tested end to end

Add a test reproducing §1's actual deadlock, not just a synthetic mixed set:

1. Receiver accepts and seals a claim's patches.
2. A second claim names one of those patches **plus** a new unsealed one.
3. The second seal **succeeds**, sealing only the new patch.
4. The new patch is afterwards reachable from the ref tip.

Step 4 is the one that matters — it proves the deadlock is gone, not merely that the refusal was
removed.

## 6. Out of scope

- **The claim schema.** D7 needs no field; RFC 116's stress round confirmed no third amendment.
- **`check_recognition_claim_consistency`** — Stage 4 does not call it, and this does not change that.
- **RFC 116's negotiation artifacts and per-ref sequencing.** Next increment.
- **`accept_exchange_artifact`** — already follows D7's rule (`accept.rs:261` skips held objects,
  `:186-190` refuses absent referents). **Do not "align" it; it is the reference.**

## 7. Tests and controls

Each needs a control **observed failing**:

| Property | Control |
|---|---|
| A mixed claim seals the unsealed remainder | Restore the `sealed_count != 0` refusal → the inverted row 7 fails |
| The block carries only the unsealed subset, in claim order | Pass `claim.patch_ids` unfiltered → the mixed-case row 11 fails |
| A fully-sealed claim is still a no-op | Make the empty filter seal an empty block → row 9 fails |
| An **absent** patch still refuses the whole claim | Disable the existence loop → row 6 fails |
| The no-op path still writes nothing for an unadopted signer | §4's test, unchanged |
| The deadlock is gone end to end | §5's test; without the fix it refuses at step 3 |

Rows 1 and 4 are the pair that matters: **seal-what-is-missing** and **refuse-what-is-absent** must both
be pinned, or the rule degenerates into "never refuse."

## 8. What to report

1. Control output for each row of §7 — actual failure text, and the single line mutated.
2. **How you restructured `AlreadySealed`**, and evidence §4's invariant survived it.
3. §5's end-to-end test, including step 4's assertion.
4. **The full gate set against the exact commit, after the last edit**: `cargo fmt --all -- --check`;
   `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`;
   `cargo test --workspace --locked`; `cargo +1.85.0 test --workspace --locked`; `git diff --check`;
   `cargo audit --no-fetch`; release-policy `check` / `boundary-check` / `reference-check`.
   Cross-target clippy pair only if this diff contains `#[cfg(target_os)]`.
5. Test counts before and after, per crate. **`snapshot.txt` must not change** — no schema here.
6. Anything here that turned out to be wrong. **Say so plainly.**

**Stop and escalate, do not guess**, if: filtering breaks an ordering assumption D6 left implicit; §4's
invariant cannot be preserved without moving the trust check; or the end-to-end test in §5 cannot be
built because the deadlock is not reachable the way §1 describes — **that last one would mean my
analysis is wrong, and I would rather hear it than have it worked around.**
