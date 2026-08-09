# DC-82 Mutation Dispatch Collapse — Handoff v1

**Cleared to start on §3 only.** Accepted 2026-08-09,
`rfcs/accepted/DC-82-MUTATION-DISPATCH-COLLAPSE.md`. **Authored by** the architect.
**Sequenced after DC-81 closes** (it is blocked only on CI evidence), **before Windows.**

## 1. Why this exists, stated plainly

DC-81 §6 set a gate-reduction target and **DC-81 moved the other way**: `fsutil/` 110 → 135,
`anchored.rs` 31 → 44. **That is not a miss on your part — I recorded §6 mid-increment and never issued
an addendum pointing you at it.** Same class as the criterion-3 and criterion-5 problems: my process.

But the direction matters. Extrapolated, Windows takes `anchored.rs` to about 57. **Ten call sites each
branching per platform does not scale**, and it is cheaper to fix with two implementors than three.

## 2. A proposition to test, not a design to build

**Make "unsupported" an implementor rather than an arm.** If `NoDurability` implements
`DurabilityContract` by returning `unsupported_mutation()` from every method, one gated type alias picks
the active implementor and **every call site becomes unconditional**. Each future platform then adds one
line, not ten arms.

**Test it. If it does not survive contact with the code, say so** — that is a valid deliverable, and my
design assertions this cycle have needed correction more than once.

## 3. The one thing this must not break

**DC-71's guarantee.** `unsupported_mutation()` is a **runtime** error, not a compile error — that is
what lets `prikk-store` compile on FreeBSD, illumos, or any target with no implementor, so read-only
commands still work there. **Criterion 5 asks you to demonstrate this, not assert it:** show the crate
still builds for such a target and that mutation fails at runtime.

## 4. The bar

- **Zero `target_os` at `anchored.rs`'s ten mutation call sites.**
- **Production-code gate count in `fsutil/` in single digits**; report production and test counts
  separately, plus the before/after total.
- **No behaviour change** — every test unchanged, on Linux **and** on the macOS CI job DC-81 adds.
- **DC-76's nine negative controls still fail when their guarantee is removed.** A refactor that quietly
  unpins them would be the worst outcome available here, and I will re-run some.

Gates: rule 9 **as amended**, eleven.

## 5. Stop-and-report

If the collapse appears to require changing `DurabilityContract`'s method set or any of the nine
guarantees, **stop and report**. That is a contract change and this increment does not own one.
