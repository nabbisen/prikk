# DC-79 sha2 and getrandom Upgrade — Handoff v1

**Cleared to start on §1 only.** Accepted 2026-08-09, `rfcs/done/DC-79-SHA2-GETRANDOM-UPGRADE.md`.
**Authored by** the architect. **Not urgent** — `cargo audit` is clean. **Sequenced behind DC-76 and
DC-78**; take it when the queue reaches it, or earlier if you judge you have capacity, since it touches
`prikk-hash`/`prikk-crypto` and cannot collide with either.

## 1. The half that matters

`sha2` derives **every `ObjectId`** — `prikk_hash::sha256`, `crates/prikk-hash/src/lib.rs:18-25`.
SHA-256's output is fixed by the standard so nothing should move, **but this project does not ship
"should".** Criterion 2 is the proof: **DC-41's hash vectors pass unchanged, and no expected-hash
literal anywhere in the tree is edited.**

That is the same evidence that carried DC-75's format change — where I confirmed the format was
byte-stable precisely because no hash literal moved in the commit. **If you find yourself editing an
expected hash, stop and report; you have found something far more interesting than a version bump.**

`getrandom` is the mild half — output is never compared across versions, so nothing identity-bearing
depends on it.

## 2. Answer these before touching a manifest

1. **MSRV.** Does either upgrade raise the floor above 1.85? DC-46 set it, CI gates `cargo +1.85.0`.
   **Stop and report if so** — the MSRV contract is the owner's.
2. **Does `sha2` 0.11 change any digest output?** From the vectors, not the changelog.
3. **What does `getrandom` 0.4 change** at the node-id minting and key-material call sites?
4. **Lock duplication, before and after.** Three versions are present today — 0.2.17, 0.3.4, 0.4.3 —
   with the direct pin at 0.2 and the newer two arriving transitively. Upgrading may *reduce*
   duplication. Report the real delta rather than assuming either way.

## 3. Limits

`ALLOWED_THIRD_PARTY` untouched — both crates are already permitted, only versions move. No other
dependency. No behavioural change: if either crate computes something different, that is a **finding to
report**, not scope.

Gates verbatim per `EXECUTION-ORDER.md` §6 rule 9, with test **and locked-package** counts before and
after.
