# DC-95 Stage 1, Round 1 — Condition Accepted

**Reviewing:** `f423667` on `dc-95-verify-coverage-and-finding-accumulation`.

**Accepted. No further conditions. Round 1 is complete and the standard for the remaining 28 is set.**

## 1. The condition is discharged, and the proof is in the failure message

I re-ran both directions myself.

**With shape validation intact:** the test passes, all eight rows.

**With `validate_block_v2_shape` disabled:**

```
case "root-with-parent": expected verify_repository to reject a shape violation
```

**Compare that to the same probe before the fix:**

```
case "root-with-parent": expected error containing "Root Block must have zero parents",
got: state root does not match authoritative replay
```

The message moved from *"a different check caught it"* to *"nothing caught it."* `verify_repository`
now returns `Ok` — the repository genuinely verifies clean when shape validation is absent. **That is
the rule's own premise demonstrated on the rule's own terms**, not a message-shape assertion standing in
for it.

Gates re-run at `f423667`: fmt, clippy, `cargo test --workspace --locked`, `cargo +1.85.0 test
--workspace --locked`, 615 prikk-store tests, `git diff --check`, `cargo audit`, all three release-policy
checks — clean.

## 2. The part I would have accepted a shortcut on, and they did not take one

I named `naive_continue` as the tool for this, since it was DC-92's own fix. **They did not use it, and
explained why rather than silently diverging**: no row here builds on an already-corrupted ancestor —
each fixture's parent is a real, valid block — so `derive_next_state_root` resolves directly and
`naive_continue`'s from-scratch continuation is unnecessary.

That reasoning is correct. `naive_continue` exists because DC-92 needed a root for a block whose ancestor
was itself corrupt; that condition does not hold here. **Reporting the difference from a precedent I
pointed at, rather than either following it mechanically or quietly ignoring it, is the right handling.**

**And the per-row derivation is the careful part:** they re-derived `state_derivation_parent`'s match
arms rather than assuming, and noted that non-`Merge` kinds resolve through `parent_block_ids.first()`
regardless of shape validity — so `root-with-mainline-field` and `merge-without-mainline` both resolve to
`None`, *ignoring the very fields that make them invalid*. Getting that wrong would have produced roots
that look derived but are not, and the test would have passed for the wrong reason again.

## 3. The standard is now set, which was the point of the condition

Their closing statement is the one I wanted round 1 to produce:

> every fixture's non-shape/non-target fields must be independently correct, computed from what the code
> would actually derive if the check under test were absent — not assumed, not placeholder values —
> checked by disabling the production check and confirming the repository **genuinely verifies clean**,
> not merely that some error fires.

**That is the bar for the remaining 28 checks across four clusters.** Fixing it at round 1 cost one
round; carrying it to round 4 would have cost four, and the earlier rounds would have needed redoing.

## 4. Standing

- **Round 1: complete.** 8 of 36.
- **Round 2** next — `verify/objects.rs`'s remaining 7, the same construction family, so the table
  carries.
- Green **three-platform** CI before any merge; this touches `crates/prikk-store`.
- Stage 2 remains behind all of Stage 1, scoped as the two pieces the prerequisite ruling identified.
