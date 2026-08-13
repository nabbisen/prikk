# RFC (accepted) - DC-79 sha2 and getrandom Upgrade

**Status.** **ACCEPTED by the project owner 2026-08-09**, who directed these be scheduled.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** Three stale dependencies reported by the owner 2026-08-09, recorded in `FINDINGS.md`.
**Target.** 0.20.0. **Not urgent** — `cargo audit --no-fetch` is clean; this is staleness, not exposure.

## 1. Scope

- `sha2` `"0.10"` → `"0.11"`
- `getrandom` `"0.2"` → `"0.4"` (two majors)

`ed25519-dalek` is **DC-80**, deliberately separate: it carries behavioural risk to already-sealed
signatures, which these two do not.

## 2. Why `sha2` is the sensitive half

`crates/prikk-hash/src/lib.rs:18-25` runs `prikk_hash::sha256` on `sha2`, and that derives **every
`ObjectId` in the system.** SHA-256's output is fixed by the standard, so nothing should move —
**but "should" is not evidence**, and this project already owns the machinery to prove it: DC-41's hash
vectors and DC-55's differential exist for exactly this.

`getrandom` is milder: its output is never compared across versions, so nothing identity-bearing depends
on it. Its exposure is API churn and platform behaviour.

## 3. Blocking prerequisites

1. **Does either upgrade raise the workspace MSRV above 1.85?** Unverified and flagged rather than
   asserted. DC-46 set that floor and CI gates on `cargo +1.85.0`. **If either needs a newer toolchain,
   stop and report** — the MSRV contract is an owner decision.
2. **Does `sha2` 0.11 change any digest output?** Answer from the vectors, not from the changelog.
3. **What does `getrandom` 0.4 change at the call sites** that mint node ids and key material?
4. **Lock duplication before and after.** Three versions are present today (0.2.17, 0.3.4, 0.4.3) with
   the direct pin at 0.2; upgrading may *reduce* duplication. Report the actual delta.

## 4. Acceptance criteria

1. §3 answered and reported before any change.
2. **DC-41's hash vectors pass unchanged, and no expected-hash literal anywhere in the tree is edited.**
   This is the proof that no `ObjectId` moved — the same evidence that carried DC-75's format change.
3. All existing tests pass **unchanged** (889 at time of writing). A test that must change is a finding
   to report, not an edit to make quietly.
4. MSRV remains 1.85, **or** an owner ruling is obtained first and recorded.
5. `ALLOWED_THIRD_PARTY` untouched — both crates are already permitted; only versions move.
6. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, verbatim, with test counts and locked-package
   counts before and after.

## 5. Non-goals

`ed25519-dalek` (DC-80). Any other dependency. Any change to what `prikk-hash` or `prikk-crypto`
*compute* — this is a version move, and a behavioural difference is a finding, not scope.
