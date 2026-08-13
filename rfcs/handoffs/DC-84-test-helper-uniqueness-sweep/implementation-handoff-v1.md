# DC-84 Test Helper Uniqueness Sweep — Handoff v1

**Cleared to start.** Accepted 2026-08-09, `rfcs/done/DC-84-TEST-HELPER-UNIQUENESS-SWEEP.md`.
**Small, test-only, not urgent** — after DC-80 unless you judge otherwise.

## 1. This is your DC-83 §2 finding, ruled in scope

You reported that the other thirteen helpers **and `unique_temp_dir` itself** lack a true counter, and
asked whether it warranted an increment or should stay recorded and unowned.

**It warrants one.** Your own measurement moved it out of the theoretical category — 214 collisions in
128,000 samples is a rate, not a possibility, and `unique_temp_dir` backs **580 `prikk-store` tests**.
The only reason it has not manifested there is contention and luck.

## 2. Two jobs

**Add a real atomic counter** to `unique_temp_dir` and the thirteen siblings, in the shape you landed in
`format_transition.rs`. **Prefer one shared helper to fourteen copies** if the crate boundary allows —
and if it does not, say so rather than manufacturing a crate for it.

**Rename `monotonic_suffix`.** It returns a wall-clock timestamp and the name says counter. **That name
caused a real error: I cited the function as the correct pattern in DC-83's handoff on the strength of
its name, without reading it — and my instruction to "mirror it exactly" would have left the bug in
place.** You caught that. A name that misleads a careful reader once will do it again.

## 3. The bar

**Demonstrate it, in your own DC-83 shape** — a barrier-synchronized multi-thread sampling test showing
zero collisions. **One demonstration for the shared helper is enough**; do not add one per call site.

All tests unchanged, both toolchains, macOS green, **no production code touched**. Gates per rule 9 as
amended.

## 4. Not this increment

Retry or serialization — the fix is uniqueness, as before. Any change to what tests assert. Anything
that ships: every helper here is test-scoped and this must not become a reason to touch production.
