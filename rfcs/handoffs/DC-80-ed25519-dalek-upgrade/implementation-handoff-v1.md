# DC-80 ed25519-dalek Major Upgrade — Handoff v1

**Cleared to start on §1 only.** Accepted 2026-08-09, `rfcs/accepted/DC-80-ED25519-DALEK-UPGRADE.md`.
**Authored by** the architect. **Sequenced after DC-79.** Not urgent — no known vulnerability.
**Not routine either.**

## 1. What you are actually changing

The library that creates and verifies **every signature in the system**: author signatures on patches,
maintainer signatures on blocks, ref states and ref updates, and the trust-store policy signature that is
the product's **only** cryptographic verification call site (`crates/prikk-store/src/trust.rs:215`).

**This is a compatibility question about already-sealed artifacts, not a version move.**

## 2. The two failure directions, and they are not symmetric

- **3.x rejects a signature 2.x accepted** → verification of already-sealed history breaks. Loud, bad,
  and you will notice immediately.
- **3.x accepts a signature 2.x rejected** → **worse, because it is silent**, and it weakens the
  guarantee the whole project rests on.

Ed25519 verification has real strictness variation across implementations — malleability, cofactor
handling, strict versus permissive verification. **Assume nothing from the version number.**

## 3. Criterion 3 is the one I will check hardest

Both directions, constructed:

1. A **tampered** signature still fails under 3.x.
2. **A signature 2.x rejected is still rejected under 3.x.**

**The second is the one nobody thinks to write**, and it is exactly the silent direction. A suite that
only proves "valid things still verify" would pass while the guarantee quietly weakened — the same shape
as DC-74's refusal tests, where four of five survived removing the gate they existed to pin, and I found
it by negative control. I will run controls here too.

Criterion 2 stands alongside it: **a repository sealed with 2.x verifies identically under 3.x**, built
with the old version and verified with the new, through the compiled binary.

## 4. Prerequisites, before touching a manifest

1. **MSRV** — stop and report if the floor rises above 1.85.
2. **What changed in 3.x's verification semantics.** From the crate's own source and changelog, and
   **cite what you opened** — a path that cannot be opened is not evidence. That note is from DC-76's
   review, where the substance was right but the cited path did not exist.
3. **Is any already-sealed signature affected?** By construction, not by reading.

## 5. Hard limit

**If the upgrade appears to require changing what is signed, the signature preimage, or any envelope
shape — stop and report.** That is a format question and this increment does not own it.

`ALLOWED_THIRD_PARTY` untouched. Gates verbatim, counts before and after.
