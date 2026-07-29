# DC-55 First-Party SHA-256 Replacement - Implementation Evidence v1

**Date:** 2026-07-28
**Handoff followed:** `implementation-handoff-v1.md`, cleared to start after project-owner acceptance
of `rfcs/accepted/DC-55-FIRST-PARTY-SHA256-REPLACEMENT.md` (commit `a01e628`).
**Reproducibility note:** every claim below is written so a reviewer can reproduce it from the
repository alone — the seed for Step 3a, the exact commands for both backend runs, and the fixture
for the end-to-end check are all in the tree. This is what compensates for the design-review
independence gap the RFC's Status field records (see the RFC and `.git-exclude/reviewed/
prikk-dc55-design-review-v1.md`).

## B1 repair (2026-07-29, addendum after `.git-exclude/reviewed/prikk-dc55-implementation-review-v1.md`)

**Finding:** `cargo test --workspace --locked` failed on the committed tree (`083d6c0`) —
`dc55_sha256_identity_end_to_end` errored with `directory is absent: refs/tmp`. Root cause: six of
`RepositoryLayout::required_directories()` are empty at rest in the fixture (`objects/tag`,
`objects/attestation`, `refs/locks`, `refs/tmp`, `cache`, `quarantine`) and git cannot represent an
empty directory, so they were silently absent from the commit despite being present wherever the
fixture was originally created and tested.

**Fix.** `copy_fixture` in `crates/prikk-cli/tests/dc55_sha256_identity_end_to_end.rs` now opens a
`RepositoryLayout` on the copied fixture and `create_dir_all`s every entry from
`layout.required_directories()` after copying — not a hardcoded list, so a future layout change
cannot silently reintroduce the gap. A `.gitkeep`-style placeholder was considered and rejected: it
was verified to make prikk's own integrity checking fail closed
(`unexpected non-directory in object type directory`), confirmed by reproducing that exact error
before settling on directory recreation instead. This is documented in the file's module doc so a
future maintainer hitting `directory is absent` does not reach for `.gitkeep` first (review N2).

**Verification — required for re-review, all three items:**

1. **B1 fixed, root cause addressed generically.** `git diff` limited to `copy_fixture`'s body plus
   an explanatory doc comment; no change to the fixture's committed bytes or the identity claim.
2. **`cargo test --workspace --locked` passing on a fresh clone of the repaired commit, not the
   working tree.** Verified by committing the fix in a disposable scratch clone (never the project
   repository — no commit was made here), then cloning *that* scratch repository fresh a second time
   and running the full workspace suite there: all crates pass, including
   `dc55_sha256_identity_end_to_end`. Before applying the fix, the same two-hop-fresh-clone procedure
   independently reproduced B1's exact failure (`directory is absent: refs/tmp`), confirming the
   diagnosis rather than assuming it.
3. **N2's comment is in place** — see the module doc addition above `copy_fixture` at the top of the
   test file. N3 (whether any other fixture depends on empty directories) checked: `git ls-files | grep
   '\.prikk/'` shows the DC-55 fixture is the only checked-in repository fixture in the tree, so
   nothing else is exposed to this failure mode.

All other gates re-run clean on the repaired working tree: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
`cargo +1.85.0 test --workspace --locked`, `git diff --check`, release-policy `check` (154/154),
`boundary-check` (valid), `reference-check` (valid). Test counts unchanged from the original note.

## What did not change

- No call site among the 11 production sites (`prikk-object`: `id.rs:122`, `payload/patch.rs:17`;
  `prikk-store`: `wal.rs:372`, `layout.rs:379`, `state_root.rs:66,78,93,108`, `refs/log.rs:253`,
  `text_span.rs:150,168`) was edited. `cargo build --workspace --locked` succeeds with zero changes
  outside `crates/prikk-hash`, `Cargo.toml`, `tools/release-policy/src/boundary/placement.rs`,
  `crates/prikk-object/src/vectors/snapshot.rs` (assertion message only), and a new
  `crates/prikk-cli/tests/dc55_sha256_identity_end_to_end.rs` + fixture.
- The three public items `Sha256Digest`, `sha256`, `to_hex` are unchanged in signature and
  semantics.
- No product manifest other than `crates/prikk-hash/Cargo.toml` and root `Cargo.toml` changed.
  `release/`, `release-signers.toml`, both command inventories, and the oracle manifest are
  untouched (`git status --short` confirms).
- No persisted format, schema, or wire grammar changed. This RFC forbids that by design; §"Step
  3a" and §"Step 3c" below are the evidence that it held.

## Step 1 — baseline (unmodified tree)

Test counts confirmed **except one discrepancy, corrected**:

| Crate | Handoff's stated baseline | Actual (measured) |
|---|---:|---:|
| `prikk-store` | 543 | 543 — matches |
| `prikk-object` | 76 | 76 — matches |
| `prikk-replay` | **4** | **44** — mismatch |
| `prikk-hash` | 13 | 13 — matches |
| `prikk-crypto` | 5 | 5 — matches |
| `prikk-release-policy` | 57 | 57 — matches |

`prikk-replay` is a stale-documentation error, not a moved baseline: `git log -- crates/prikk-replay/
src/` shows the last commit touching that crate is `e8f780a` (DC-54, well before this session), and
`prikk-replay` has been reported as "unchanged" through DC-51, DC-54, and DC-50's bookkeeping. The
`4` figure was a typo I introduced myself in `rfcs/EXECUTION-ORDER.md` while fixing DC-51's B1
finding, which the DC-55 RFC/handoff then copied without independent re-verification. Corrected in
`rfcs/EXECUTION-ORDER.md` as part of this submission; does not affect DC-55's scope or claims.

Locked package count: **180**, `Cargo.lock` sha256 `601d0678b8481a750519e64bb19f66f8532301b4157d8353
d8d9211261c5da31` — matches the frozen baseline.

Pre-swap performance (this hardware, AMD Ryzen 9 9950X, release profile, outgoing first-party
implementation):

| Size | Throughput |
|---|---:|
| 64 B | 207.9 MB/s |
| 4 KB | 458.0 MB/s |
| 1 MB | 464.1 MB/s |

(DC-50's figures were 220/463/470 MB/s on different hardware — consistent order of magnitude, as
expected for a scalar reference implementation.)

## Step 2 — the swap

- `crates/prikk-hash/Cargo.toml`: `sha2 = { workspace = true }` moved from `[dev-dependencies]` to
  `[dependencies]`.
- `tools/release-policy/src/boundary/placement.rs:7`: `("prikk-hash", &[])` →
  `("prikk-hash", &["sha2"])`. Landed together with the manifest move (see single working-tree
  diff; both are part of the one candidate commit).
- `crates/prikk-hash/src/lib.rs`: `sha256` reimplemented over `sha2::Sha256`; `H0`, `K`, `compress`,
  `word_at`, `small_sigma0`, `small_sigma1` moved (not deleted) into new
  `crates/prikk-hash/src/tests/frozen_outgoing.rs`, `#[cfg(test)]`-only via the existing
  `#[cfg(test)] mod tests;` gate. `to_hex` untouched. Crate doc rewritten to state what the crate is
  now and cite DC-50/DC-55; the `#![forbid(unsafe_code)]` caveat about `sha2`'s accelerated backends
  is stated explicitly.
- Root `Cargo.toml`: `sha2 = "0.10"` moved out of the `# Testing` group into the production
  third-party group (alongside `ed25519-dalek`, `getrandom`, `rustix`).
- `crates/prikk-object/src/vectors/snapshot.rs`: both the module doc and the `assert_eq!` failure
  message amended to carve out identity-preservation increments as a stop-work condition, per the
  RFC's Risks section — the warning now appears at the moment of failure, not only in prose above it.
- `tools/release-policy/src/boundary/placement/tests.rs`: `disallowed_third_party_in_product_
  dependencies_fails` used `prikk-hash` + `sha2` as its example of a disallowed pairing; since that
  pairing is now legitimately allowed, retargeted it at `prikk-error` + `rand` (still a
  zero-third-party crate) and added two new tests: `sha2_is_allowlisted_for_prikk_hash_since_dc55`
  (passes) and `sha2_remains_disallowed_for_other_zero_allowlist_crates` (using `prikk-replay`,
  fails) — DC-51's placement gate is still exercised for exactly the case it exists to catch,
  just no longer using `prikk-hash` as the negative example.

`cargo build --workspace --locked` succeeds with no call-site edits required.

## Step 3a — the equivalence campaign (outgoing vs incoming)

`crates/prikk-hash/src/tests/hash_differential.rs` repurposed: `reference_sha256` (which wrapped
`sha2::Sha256` directly) replaced with `frozen_outgoing::sha256`, so the comparison is now the
current (`sha2`-backed) `sha256` against the frozen pre-DC-55 first-party implementation — not a
self-comparison.

```
$ cargo test -p prikk-hash --locked hash_differential
test tests::hash_differential::split_mix64_matches_published_self_check_sequence ... ok
test tests::hash_differential::sha256_matches_frozen_pre_dc55_implementation_across_randomized_cases ... ok
```

- Seed: `0x243F_6A88_85A3_08D3` (unchanged from DC-41 stage 3 — the leading fractional bits of π).
- Case count: 10,000 randomized cases (same length distribution as DC-41 stage 3) + all 11 fixed
  vectors (run separately, see Step 3b row 4).
- Result: **zero mismatches.**

This test is not a one-time script — it is a permanent addition to `cargo test -p prikk-hash`, so it
re-runs on every future CI invocation (see Step 4).

## Step 3b — six corroborating artifacts, individually

| # | Artifact | Result |
|---|---|---|
| 1 | `crates/prikk-object/src/vectors/hard.rs` | 28 passed, 0 failed, no regeneration. Includes `empty_patch_anchor_matches_fdd_golden_vector` and `codec_sample_object_id_is_stable`. |
| 2 | `crates/prikk-object/src/vectors/snapshot.txt` | `git status --short` shows **no diff** on this file — byte-identical, untouched. `generated_snapshot_matches_committed` passed without `PRIKK_REGEN`. |
| 3 | `crates/prikk-store/src/state_root/tests/vectors.rs` | 2 passed (`accepted_literal_preimages_and_leaf_hashes_are_stable`, `accepted_empty_and_odd_even_reduction_roots_are_stable`). |
| 4 | `crates/prikk-hash/src/tests.rs` | 11 fixed vectors, all green (4 canonical FIPS 180-2/RFC 6234, 7 independently computed) — part of the 14-test `prikk-hash` run below. |
| 5 | `crates/prikk-store/src/text_span/vectors.rs` | 15 passed, covering the 58 committed literal cases. |
| 6 | `tests/fixtures/object-id-vectors.md` | Checked by hand: vector `5f8711b3f84991d60b65221d66ed5ec260d28cc19c5c4ed3c1fe44d334265fe6` matches `snapshot.txt`'s `patch_payload` row exactly (`grep patch_payload snapshot.txt` confirms the identical hex tail). No test consumes this file; this is the required manual statement. |

## Step 3c — end-to-end repository check

Built the `prikk` binary from the **unmodified (outgoing)** tree and created a fixture repository
exercising all 11 call sites: `init` → two `CreateFile`s → `commit` → trust a maintainer → `seal`
(first WAL/ref-log/state-root cycle, `Root` block) → edit a tracked text file → `commit` (exercises
`text_span.rs` via `EditText`) → `seal` again (second cycle, `Normal` block, `parents: 1`).

Checked in at `crates/prikk-cli/tests/fixtures/dc55_pre_swap_repo/` (relocatable — content-addressed
storage means no path is embedded in any object; confirmed by `verify` passing after copying it to
a different absolute path).

Then rebuilt `prikk` from the **swapped (incoming)** tree and ran, against a fresh copy of the
fixture:

```
$ prikk verify
checked objects: 8
checked blocks: 2
checked ref-log records: 2
ref publication issues: 0
...
$ prikk doctor
issue summary: errors=0, warnings=0, info=1
```

Every block ID, ref-state, and ref-log record produced by the outgoing binary was independently
re-verified — recomputed and matched — by the incoming binary. This is the check that would catch a
`layout.rs:379` storage-key change or a `wal.rs`/`refs/log.rs` checksum change directly, rather than
by inference from Step 3a.

Added as a **permanent regression test**:
`crates/prikk-cli/tests/dc55_sha256_identity_end_to_end.rs::post_swap_binary_verifies_pre_swap_repository_clean`,
which copies the checked-in fixture to a temp directory and re-runs this exact check on every future
`cargo test -p prikk`.

**Staging note for whoever commits this candidate:** the repository's `.gitignore` has a blanket
`*.log` rule, which matches `crates/prikk-cli/tests/fixtures/dc55_pre_swap_repo/.prikk/refs/logs/
*.log` — the ref-log file the fixture needs. `git add` must use `-f` on that specific path (or the
fixture directory), or the committed fixture will silently be missing its ref-log and the new test
will fail for everyone after the first clone.

## Step 4 — differential disposition

**Chosen: keep the frozen outgoing implementation as the differential's permanent reference**
(RFC item 5, option 2). Reasoning: it costs zero new dependencies, remains genuinely independent of
`sha2` (a different code path, not merely a different call), was already reviewed under DC-41, and
is immutable by construction — nothing about a frozen `#[cfg(test)]`-only module invites future
maintenance. The cost the RFC names — first-party SHA-256 code remains in the tree as test
scaffolding — is accepted; "maintain" does not meaningfully apply to code that is never touched
again and carries a module-level comment saying so.

This choice also meant Step 3a's one-time proof and Step 4's standing regression coverage collapse
into the same test file rather than requiring two separate pieces of infrastructure — reusing
`hash_differential.rs`'s existing `SplitMix64`/distribution machinery from DC-41 stage 3 rather than
building new scaffolding.

## Step 5 — both backends

```
$ cargo test -p prikk-hash --locked            # default features
test result: ok. 14 passed; 0 failed

$ cargo test -p prikk-hash --locked --features sha2/force-soft
test result: ok. 14 passed; 0 failed
```

Both runs include Step 3a's full 10,000-case campaign and all 11 fixed vectors, and both matched the
same committed vectors.

Runtime probe on this hardware: `is_x86_feature_detected!("sha") = true` (AMD Ryzen 9 9950X, `sha_ni`
present per `/proc/cpuinfo`). So the default run genuinely exercised the accelerated backend and the
`force-soft` run genuinely exercised the scalar fallback — both paths covered by construction, not
by hardware luck, and independently reproducible by any reviewer via the two commands above.

## Step 6 — performance, post-swap

| Size | Before (outgoing) | After (incoming, `sha2`) | Ratio |
|---|---:|---:|---:|
| 64 B | 207.9 MB/s | 1195.2 MB/s | 5.75x |
| 4 KB | 458.0 MB/s | 2679.0 MB/s | 5.85x |
| 1 MB | 464.1 MB/s | 2763.3 MB/s | 5.95x |

Consistent with DC-50's ~5.8x figure measured on different hardware. Not a re-litigation of DC-50;
recorded for DC-42's use as a real, current baseline.

## Test counts, before / after

| Crate | Before | After | Delta |
|---|---:|---:|---|
| `prikk-store` | 543 | 543 | 0 |
| `prikk-object` | 76 | 76 | 0 |
| `prikk-replay` | 44 | 44 | 0 |
| `prikk-hash` | 13 | 14 | +1 (`frozen_outgoing`'s self-check vector) |
| `prikk-crypto` | 5 | 5 | 0 |
| `prikk-release-policy` | 57 | 59 | +2 (placement-gate coverage for the new allowlist entry) |
| `prikk` (new integration test file) | — | 1 | +1 (`dc55_sha256_identity_end_to_end`) |

Locked package count: **180**, unchanged. `Cargo.lock` sha256:
`601d0678b8481a750519e64bb19f66f8532301b4157d8353d8d9211261c5da31` — byte-identical before and
after. No new dependency; `sha2` was already resolved in the graph via `ed25519-dalek`.

## Gate output

All green, both toolchains:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked` — all crates pass, counts above
- `cargo +1.85.0 test --workspace --locked` — identical counts
- `git diff --check`
- `cargo audit --no-fetch` — 180 crate dependencies scanned, 0 advisories
- release-policy `check` — all 154 oracle cases passed
- release-policy `boundary-check --format json` — `valid: true`
- release-policy `reference-check --format json` — `valid: true`

## Acceptance criteria, against the accepted RFC's list

1. Implemented over `sha2`; outgoing code retained test-only per item 5; three public items
   unchanged; docs rewritten. **Met.**
2. Item 1a's campaign: ≥10,000 cases + 11 fixed vectors, seed and results recorded above. **Met.**
3. Item 1b's six artifacts, byte-identical, individually reported. **Met.**
4. Item 1c's end-to-end check passes, and is now a permanent test. **Met.**
5. Both backend runs pass against the same committed vectors, runtime probe recorded. **Met.**
6. Allowlist and manifest move land in the same commit; `boundary-check` passes. **Met** — verify at
   commit time that both changes are staged together.
7. Differential disposition implemented (kept, per item 5) and rationale recorded. **Met.**
8. Fresh performance figures recorded for DC-42. **Met.**
9. Full gate set and test counts before/after. **Met.**
