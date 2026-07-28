# DC-41 Stage 3 - Hash Differential Evidence

**Scope.** Stage 3 only, per `stage-3-hash-differential-v1.md`. No CI change, no production code,
`proptest` not added (stage 4).
**Predecessor.** Stage 2 committed as `d5bd096`.

## `sha2` was already present — confirmed independently

Before adding anything, I confirmed the handoff's premise correction myself rather than trusting it:
`sha2 0.10.9` was already locked in `Cargo.lock` as a transitive dependency of `ed25519-dalek` (its
Ed25519 implementation uses `sha2`'s SHA-512 internally). This is why stage 3 adds a dependency **edge**,
not a new package, to the graph.

**Completeness note.** `sha2` is present twice over: also as a direct `[dependencies]` entry of
`tools/release-policy/Cargo.toml:17` (pre-existing, unchanged, `publish = false` — a tooling crate, not a
product crate). That strengthens rather than weakens the independence argument below — `sha2` was never a
stranger to this workspace's dependency graph.

## What changed

1. Root `Cargo.toml`: added `sha2 = "0.10"` to `[workspace.dependencies]`, with a comment noting it is
   dev-only and not part of the production dependency surface listed above it (`ed25519-dalek`,
   `getrandom`, `rustix`). **Revision note:** the candidate originally declared `sha2 = "0.10"` directly in
   `crates/prikk-hash/Cargo.toml`, matching this workspace's stated convention (dev-dependencies declared
   per-crate; `[workspace.dependencies]` reserved for shared production dependencies) and the pre-existing
   `tools/release-policy` precedent. That produced a real, if small, technical-debt concern: two
   independent `"0.10"` declarations (`prikk-hash` and `tools/release-policy`) that could drift out of sync
   under manual maintenance. Centralizing `prikk-hash`'s copy in `[workspace.dependencies]` fixes that for
   this crate without touching `tools/release-policy`, which is a separate, already-shipped crate outside
   this stage's scope — reducing two independent declarations to one real duplicate rather than eliminating
   duplication entirely. `Cargo.lock`'s resolved `sha2 0.10.9` entry, hash, and package count are unchanged
   by this revision (confirmed: rebuilt, all three identical to the pre-revision candidate).
2. `crates/prikk-hash/Cargo.toml`: added
   ```toml
   [dev-dependencies]
   sha2 = { workspace = true }
   ```
   **Explicit placement statement:** `sha2` is in `[dev-dependencies]` only. It does not appear in
   `[dependencies]` anywhere in this crate or the workspace. Per the stage-1 B4 finding, no mechanical
   gate detects misplacement of a third-party crate into `[dependencies]` — the DC-45 package-listing
   check inspects packaged file paths, and `boundary::check_dependencies` only guards the tool↔product
   edge over local crates — so this placement was verified by direct manifest inspection, not by a passing
   gate.
3. `Cargo.lock`: **+3 lines only**, exactly as predicted —
   ```diff
   name = "prikk-hash"
   version = "0.17.7"
   +dependencies = [
   + "sha2",
   +]
   ```
   Locked package count: **169 → 169** (no new package entered the graph; confirmed via
   `grep -c '^name = ' Cargo.lock` before and after). New hash: **`18a8b40aa83396974c2cacd9af56e7496d9f645cd07bda0e4164e4d0b68f0d53`**,
   which **supersedes** the prior frozen baseline `0cd51cbdc98210bc745dd6a7190fbcde30b35dfea4d1cd66b7f0b8459527c616`
   as the identity subsequent reviews should verify against. Unaffected by the workspace-dependency
   revision in item 1.
4. New `crates/prikk-hash/src/tests/hash_differential.rs`, referenced from `src/tests.rs` via
   `mod hash_differential;`. No other file changed.

## Independence of the oracle

`sha2` is already in the workspace's dependency graph via `ed25519-dalek`, so this is stated explicitly
rather than left for a reviewer to discover unaided: the differential remains sound because (a) it compares
two genuinely independent SHA-256 implementations — `prikk-hash`'s first-party code and RustCrypto's — and
their co-presence in one graph does not correlate their correctness; (b) `ed25519-dalek` uses `sha2`'s
SHA-512, a different algorithm from the SHA-256 under test here; and (c) the added dev-dependency edge does
not place `sha2` in `prikk-hash`'s **production** dependency graph, which is what the RFC's discipline
clause (differential dependencies must not enter object identity or runtime trust paths) exists to prevent.

## Randomized generator

No new dependency was added for randomness (`rand` is absent from the lockfile and would have introduced
new packages; `rand_core`, already locked, provides only traits). A ~15-line SplitMix64 generator is
defined inline in `hash_differential.rs`, seeded with the fixed constant `0x243F6A8885A308D3` (leading
fractional bits of pi). Its first six outputs are pinned by a dedicated self-check test
(`split_mix64_matches_published_self_check_sequence`), independently re-derived by me in Python before
being encoded — not copied from the handoff without verification:

```
python3 -c '
def next_val(state):
    MASK = 0xFFFFFFFFFFFFFFFF
    state = (state + 0x9E3779B97F4A7C15) & MASK
    z = state
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
    return state, z ^ (z >> 31)
state = 0x243F6A8885A308D3
for i in range(6):
    state, out = next_val(state)
    print(i, hex(out))
'
```
produced the same six values now asserted in the test.

## Input length distribution

Stated in code and re-verified empirically by branch-tagging a full replica of the shipped consumption
order (`length_for_case` **and** the subsequent `fill_bytes` call that draws the input bytes themselves)
against all 10,000 cases — not by reverse-inferring the band from the resulting length, which is unsound
here since later-boundary lengths like 119/120 fall numerically inside the 66-1024 multi-block range:

| Band | Lengths | Requested share | Measured share (10,000 cases) |
|---|---|---|---|
| Empty | 0 | exactly 1 (guaranteed, index 0) | 1 |
| Sub-block | 1-54 | ~25% | 2,489 (24.9%) |
| First-boundary neighbourhood | 55-57, 63-65 | ~25% | 2,527 (25.3%) |
| Multi-block | 66-1024 | ~25% | 2,481 (24.8%) |
| Later-boundary neighbourhood | 119-121, 127-129, 183-185 | ~25% | 2,502 (25.0%) |

Realised inputs span **957 distinct lengths** from 0 to 1024.

**Correction to an earlier draft of this note (caught at implementation review, C1):** a first verification
pass measured `1 / 2,482 / 2,506 / 2,512 / 2,499` and did not reproduce. Its probe modelled only the
`length_for_case` RNG consumption and omitted `fill_bytes`'s consumption of the input bytes themselves —
since the generator is a single running stream, omitting any consumption step desynchronizes every
subsequent draw from the real sequence. The reviewer's independent Python replication and my own corrected
Rust replica (this time including `fill_bytes`) now agree exactly on the table above. The substantive claim
was never affected — all four bands were near-evenly populated either way, and the differential's validity
does not depend on the exact counts — but the table itself needed to reproduce, since its stated purpose is
letting a reviewer judge coverage without reading the generator.

## Case count, seed, and result

- **10,000 cases**, single fixed seed `0x243F6A8885A308D3`, deterministic and reproducible by construction.
- **Zero mismatches.** Every case's `prikk_hash::sha256` output equals `sha2::Sha256`'s digest,
  byte-for-byte.
- No mismatch occurred, so the stop-work escalation path was not exercised this run; it remains coded into
  the assertion message (case index, input length, seed) for if it ever is.

## Measured runtime

The full differential test (10,000 cases) runs in the whole `prikk-hash` unittest binary alongside the
other 12 tests in **0.15s** total (unoptimized `dev`/`test` profile), isolated via
`cargo test -p prikk-hash --locked hash_differential::sha256_matches_rustcrypto_reference` and timed
separately at the same figure. This is negligible CI cost, consistent with the handoff's prediction; stage
4's much larger budgets can be planned against this measured baseline (~15µs/case at this profile).

## Cross-implementation agreement so far

With this stage, `prikk-hash` has now been checked against independent implementations on: 11 DC-40
state-root vectors (Python `hashlib`), 11 stage-2 fixed vectors (4 published + 7 Python `hashlib`), and
10,000 stage-3 randomized cases (RustCrypto `sha2`) — 10,022 total agreeing comparisons across two
independent tools. This is meaningful prior evidence, not a validation claim; stage 3's randomized coverage
is what a fixed-vector-only campaign cannot provide, but 10,000 cases still cannot cover the full input
space, which is why this is evidence, not proof.

## Test counts

- `prikk-hash`: **11 → 13** (stage-2 baseline 11; +2: the differential test and the PRNG self-check).
- `prikk-store`: **531 → 531** (unchanged; confirmed via full workspace run).

## Frozen identities

| Identity | Status |
|---|---|
| `Cargo.toml` (workspace root) | unchanged |
| `Cargo.lock` | **changed as intended** — new hash `18a8b40a...`, package count 169 → 169 |
| All other package manifests | unchanged |
| Command inventories (both) | unchanged |
| Oracle manifest | unchanged |
| `release-signers.toml` | unchanged; signer set still empty and fail-closed |

## Gate output

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean (after fixing 3 `indexing_slicing` lint violations in the new file — raw slice indexing replaced with `.get(...).unwrap_or(...)` / iterator-based copy, matching this crate's existing `word_at` helper style) |
| `cargo test --workspace --locked` | `prikk-hash` 13 passed, `prikk-store` 531 passed, all else unchanged |
| `cargo +1.85.0 build --workspace --locked` | clean (MSRV re-verified on the real integrated workspace, not just the isolated scratch-crate check from the handoff) |
| `cargo +1.85.0 test --workspace --locked` | no failures |
| `git diff --check` | clean |
| `cargo audit --no-fetch` | clean, 169 crate dependencies scanned, 0 advisories |
| release-policy `check` | all 154 oracle cases passed |
| `boundary-check` / `reference-check` | `valid: true` (confirms the seven product package listings still exclude test-only tooling) |

No production code (`sha256`, `to_hex`, or anything else in `lib.rs`) changed. No CI file touched.
`proptest` was not added.
