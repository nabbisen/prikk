# DC-78 Handoff v2 — Addendum 5: fix accepted, Stage 1 merged on macOS evidence

**Date:** 2026-08-09. **Authored by** the architect.

## 1. Accepted, and verified where it mattered

**`maintainer_key_id_rejects_case_insensitive_collision ... ok` on the macOS job**, branch
`dc-78-history-exchange` at `3455bea`. Every job green. **The regression is closed on the platform where
it existed**, not merely on the one where it was never observable.

**Merged at `cba0459`.** Main's code is byte-identical to that macOS-verified tree — the only delta is an
architecture-doc section your branch predates. Both workflows green on main.

## 2. Two things in your submission worth naming

**You verified my fix-shape claim before applying it.** I asked for that because I have been wrong about
code I only read, and you checked `existing_id != key_id` is exact-string rather than folded — which is
what makes the unconditional call safe for idempotent re-adds. I re-confirmed it at `trust.rs:104`.

**You stated what you could not verify.** ext4 cannot reproduce the bug, so a green Linux run proves the
fix did not break Linux and nothing more. Saying so, instead of letting a passing suite imply more than
it shows, is exactly the discipline that makes the macOS run meaningful rather than ceremonial.

**And you removed the stale doc comment** — *"key_id reaching here is always genuinely new"* — rather
than leaving a sentence describing an invariant the fix had just falsified. That is the
`monotonic_suffix` lesson applied without being asked.

## 3. Your re-merge preference was right and I took it

Reverting the revert restored Stage 1 under SHAs already reviewed twice; your branch contributed only
the fix. Rebuilding would have replayed the same diff under new hashes for nothing.

## 4. What the record says

`EXECUTION-ORDER.md` states the sequence plainly: merged prematurely on local gates alone, reverted,
fixed, re-merged on macOS evidence. **Not smoothed over** — the standing correction it produced is worth
more than a tidy history, and the correction is mine: **filesystem-backed changes merge only after a
green macOS run.**

## 5. Next: D3

Verify reporting which key sealed each block — §D3, the part of provenance that is missing rather than
merely unsurfaced. Then **D4/D6 together with ruling 4**, per addendum-3.

**One carried item for the D3 package**, per addendum-3 §4: report the `repository-layout.md` fix there,
since it landed without its own review.
