# DC-79 Handoff v1 — Addendum 2: accepted and complete

**Date:** 2026-08-09. **Authored by** the architect. **Review:**
`.git-exclude/reviewed/DC-79-implementation-review-v1.md`.

## 1. Accepted

All eleven gates re-run independently: 894 tests on **both** toolchains, zero failures, identical to
baseline; clippy clean on Linux, macOS and Windows targets; audit clean on 187 packages; all three
release-policy gates valid.

**Criterion 2 is met on the strongest evidence available.** No expected-hash literal edited anywhere —
the diff touches five files, none a test module — and
`sha256_matches_frozen_pre_dc55_implementation_across_randomized_cases` passes unchanged. **A randomized
differential against the frozen pre-DC-55 implementation is better than a fixed vector**, because it
re-derives agreement rather than re-asserting a constant.

## 2. My "cosmetic" framing was wrong, and you should have the accurate number

Ruling 1 called the duplication *"two versions of a hash crate… cosmetic."* **It is seven packages** —
`sha2 0.11` brings the whole RustCrypto `digest` 0.11 stack alongside the 0.10 one `ed25519-dalek 2.2.0`
still needs: `sha2`, `digest`, `block-buffer`, `crypto-common`, `cpufeatures`, `const-oid` each at two
versions, plus `hybrid-array` **genuinely new**.

**Six of seven are transient** and collapse under DC-80. **One is permanent.**

**The ruling stands, on an accurate basis rather than a flattering one** — +1 permanent crate and six
transient duplicates is a fair price for keeping DC-80 separately reviewable. **I should have checked the
transitive shape before calling it cosmetic**, and you had already given me the 180 → 187 number in the
prerequisite report, which I did not follow through on.

**DC-80's criterion 3 is broadened accordingly:** the whole duplicated stack must collapse, not just
`sha2`, with locked-package counts reported before and after.

## 3. Your `getrandom 0.2.17` flag was right on both counts

Confirmed orphaned — `cargo tree -i getrandom@0.2.17` prints nothing — and your reason for not pruning is
correct: the only scoped primitive was a broad `cargo update` bumping ~15 unrelated packages, which is
exactly the bundling this project avoids.

**One consequence worth carrying:** `cargo audit` scans the *lock*, so an orphan can raise a future
advisory on a version nothing builds. Harmless, and it argues for a maintenance-scoped refresh, as you
said.

## 4. A finding recorded, not against you

**`ALLOWED_THIRD_PARTY` cannot see transitive additions** — it governs direct dependencies only, so seven
packages entered and `boundary-check` passed, correctly. `cargo audit` covers known *vulnerabilities*,
not *additions*. Nothing would have surfaced the +7 without reading the lock diff by hand. Recorded in
`FINDINGS.md` as unowned; a lock-delta report at review time would close it cheaply.

## 5. Next

**DC-80**, carrying three inherited items: the broadened collapse criterion, `curve25519-dalek`
4.1.3 → 5.0.0 in scope for §2 question 2, and MSRV already confirmed at 1.85 for `ed25519-dalek 3`.
Handoff: `rfcs/handoffs/DC-80-ed25519-dalek-upgrade/implementation-handoff-v1.md`.
