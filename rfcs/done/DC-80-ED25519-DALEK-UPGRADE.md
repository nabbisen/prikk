# RFC (accepted) - DC-80 ed25519-dalek Major Upgrade

**Status.** **ACCEPTED by the project owner 2026-08-09**, who directed these be scheduled.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** `FINDINGS.md`, 2026-08-09. **Target.** 0.20.0, **after DC-79**.
**Not urgent** — no known vulnerability. **But not routine either.**

## 1. Why this is its own increment

`ed25519-dalek` `"2"` → `"3"` changes the library that **creates and verifies every signature in the
system** — author signatures on patches, maintainer signatures on blocks, ref states, ref updates, and
the trust-store policy signature that is the product's only cryptographic verification call site
(`crates/prikk-store/src/trust.rs:215`).

**A major bump here is a compatibility question about already-sealed artifacts, not a version move.**
Ed25519 verification has a real history of strictness variation — malleability, cofactor handling, and
the strict-versus-permissive verification distinction. Two failure directions, and they are not
symmetric:

- **Rejecting signatures that 2.x accepted** breaks verification of history already sealed. Loud, and
  bad.
- **Accepting signatures that 2.x rejected** is **worse** — silent, and it weakens the guarantee the
  whole project rests on.

DC-55 spent an entire increment on a hash implementation swap, with a frozen reference implementation
and differential evidence. **This deserves the same treatment.**

## 2. Blocking prerequisites

1. **Does the upgrade raise MSRV above 1.85?** Stop and report if so — owner decision.
2. **What changed in 3.x's verification semantics** relative to 2.x? Answer from the crate's own source
   and changelog, and **cite what you opened** — a path that cannot be opened is not evidence.
   **Also cover `curve25519-dalek` 4.1.3 → 5.0.0**, which `ed25519-dalek 3` pulls along — found by
   DC-79's probe 2026-08-09. It declares `rust-version: 1.85`, so MSRV holds, but a second major bump
   in the signature path is in scope for this question.
3. **Is any already-sealed signature affected?** The decisive question, and it must be answered by
   construction, not by reading.

## 3. Acceptance criteria

1. §2 answered and reported before any change.
2. **A repository sealed with 2.x verifies identically under 3.x** — built with the old version, then
   verified with the new one, through the compiled binary. Not argued; constructed.
3. **The duplicated `digest` stack collapses back to single versions.** Added 2026-08-09 from DC-79's
   investigation; **broadened the same day after DC-79's review measured the real delta.**
   `ed25519-dalek 3` requires `sha2 0.11`, so landing after DC-79 collapses **six** duplicated packages,
   not one: `sha2`, `digest`, `block-buffer`, `crypto-common`, `cpufeatures`, and `const-oid`, each of
   which DC-79 left at two versions. **`hybrid-array` is a permanent addition of the 0.11 stack and
   correctly stays.** Report the locked-package count before and after; DC-79 took it 180 → 187.
4. **A negative control in both directions**, and this is the criterion that matters most:
   - a **tampered** signature still fails under 3.x;
   - a signature 2.x **rejected** is still rejected under 3.x.
   The second is the one nobody thinks to test, and it is the failure direction that is silent.
5. All existing tests pass unchanged. A test that must change is a finding to report.
6. MSRV remains 1.85, or an owner ruling is obtained first.
7. `ALLOWED_THIRD_PARTY` untouched — the crate is already permitted; only the version moves.
8. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, verbatim, with counts before and after.

## 4. Non-goals

`sha2` and `getrandom` (DC-79). Changing what is signed, the signature preimage, or any envelope shape —
**if the upgrade appears to require any of those, stop and report**; that is a format question and this
increment does not own it.
