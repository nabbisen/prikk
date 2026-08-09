# DC-80 Handoff v1 — Addendum 1: two RFC amendments the handoff predates

**Date:** 2026-08-09. **Authored by** the architect. **Handoff v1 stands**; these two items were added to
the RFC after it was written and are not in it.

## 1. Criterion 3 is broader than handoff v1 implies

It originally read *"`sha2` collapses back to a single version."* **DC-79's review measured the real
delta and it is wider:** landing `ed25519-dalek 3` collapses **six** duplicated packages, not one —
`sha2`, `digest`, `block-buffer`, `crypto-common`, `cpufeatures`, and `const-oid`, each of which DC-79
left at two versions.

**`hybrid-array` is a permanent addition of the 0.11 stack and correctly stays.**

**Report the locked-package count before and after.** DC-79 took it 180 → 187; this increment should take
most of that back.

## 2. `curve25519-dalek` 4.1.3 → 5.0.0 is in scope for §2 question 2

`ed25519-dalek 3` pulls it along — found by an architect probe during DC-79's review, resolving a
throwaway project against `ed25519-dalek = "3"`. **It declares `rust-version: 1.85`, so MSRV holds**, and
that answers §2 question 1 in advance.

But it is **a second major bump in the signature path**, and §2 question 2 must cover its changes too,
not only `ed25519-dalek`'s. **Cite what you opened**, as ever.

## 3. Unchanged, and still the point

**Criterion 4's negative control in both directions.** A tampered signature must still fail — and **a
signature 2.x rejected must still be rejected under 3.x.** The second is the one nobody thinks to write
and the one whose failure is silent. It remains what I will check hardest.

The hard stop also stands: **if the upgrade appears to require changing what is signed, the signature
preimage, or any envelope shape, stop and report.** That is a format question this increment does not own.

Green macOS run before merge, per the standing rule.
