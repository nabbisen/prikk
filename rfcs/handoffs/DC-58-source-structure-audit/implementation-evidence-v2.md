# DC-58 Source-Structure Audit - Implementation Evidence v2 (batch 2)

**Date:** 2026-07-30
**Follows:** `implementation-evidence-v1.md` (batch 1, accepted per
`.git-exclude/reviewed/prikk-dc57-ruling-dc58-dc59-implementation-review-v1.md` §3, "DC-58 batch 2
may proceed").
**Handoff followed:** `implementation-handoff-v1.md`.

## What this batch delivers

All four over-500-line files queued at the end of batch 1:

1. `crates/prikk-store/src/patch_replay/decode.rs` (733 → 251), split into `decode/operations.rs`
   (338, new) and `decode/tlv.rs` (168, new).
2. `crates/prikk-object/src/payload/patch.rs` (652 → 326), split into `payload/patch/operations.rs`
   (349, new).
3. `crates/prikk-store/src/text_span.rs` (552 → 229), split into `text_span/authoring.rs` (206, new)
   and `text_span/inverse.rs` (149, new).
4. `crates/prikk-store/src/lifecycle_cache.rs` (974 → 117), split into
   `lifecycle_cache/cache_ladder.rs` (848, new) — see below for why this one is structural, not just
   a line-count reduction.

Plus: `main.rs` re-measured per the batch-1 review's N1 (still 497, no action needed), and the
source-structure report updated to final status (all files now have a recorded decision; only
`node_authoring.rs` remains over 500, by design).

**DC-58 is now complete** against its own definition of done, modulo the permanent, by-design
exceptions (`node_authoring.rs` deferred, `frozen_outgoing.rs`'s inline test excluded) — see the
report's final table.

## The `lifecycle_cache.rs` split is not the same shape as the other three

The other three splits (`decode.rs`, `payload/patch.rs`, `text_span.rs`) each separate genuinely
distinct, always-compiled responsibilities within one file — same pattern as batch 1's
`patch_replay.rs`.

`lifecycle_cache.rs` was different on inspection: ~850 of its 974 lines were already
`#[cfg(test)]`-gated on individual items, because the whole trust ladder they implement
(`DecodedLifecycleCache` → `ValidatedLifecycleCache` → `ComparedLifecycleCache`) is explicitly
scaffolding for capability not yet wired into production, per the file's own module doc. The split
moved that entire scaffold into a new file gated as a whole module instead of item-by-item, which
means the module-graph walk that defines "implementation file" for this report now correctly
excludes it — 848 lines that were being counted as implementation ELOC against this audit's own
methodology no longer are. This is recorded as a new test-support exclusion in the report, not
silently absorbed into the "split" line item.

## Verification depth, per file

- **`decode.rs` / `decode/operations.rs` / `decode/tlv.rs`**: standard split verification (build,
  clippy, full `prikk-store` suite, identical 543 count).
- **`payload/patch.rs` / `payload/patch/operations.rs`** (identity-adjacent): same, plus explicit
  `git status --short` confirmation that `vectors/snapshot.txt` and `vectors/hard.rs` — the FDD
  golden-vector files that would show any canonical-encoding drift — have no diff.
- **`text_span.rs` / `text_span/authoring.rs` / `text_span/inverse.rs`** (DC-55 evidence path): same,
  plus running the 19 `text_span::*` tests specifically by name — all `fdd01_text_span_v*` vectors,
  the DC-12 span-selection tests, and the direct-inverse tests — not just trusting the aggregate
  543-test count to catch a regression in this specific area.
- **`lifecycle_cache.rs` / `lifecycle_cache/cache_ladder.rs`**: build succeeded on the first attempt
  (production surface only), but `cargo test` initially failed with three compile errors —
  `encode_unchecked` private, `CACHE_SCHEMA_VERSION` re-export visibility, and
  `certified_compared_cache` unresolved from a *second*, separate test file
  (`lifecycle_cache/replay/tests.rs`) that also reaches into this module. All three are visibility
  fixes only (`pub(super)`/`pub(crate)` widened to match what the two test files already needed),
  no logic touched. Re-verified full green after the fixes.

## Test counts, before / after (this batch)

| Crate | Before batch 2 | After batch 2 | Delta |
|---|---:|---:|---|
| `prikk-store` | 543 | 543 | 0 |
| `prikk-object` | 76 | 76 | 0 |
| `prikk-replay` | 44 | 44 | 0 |
| `prikk-hash` | 14 | 14 | 0 |
| `prikk-crypto` | 5 | 5 | 0 |
| `prikk-release-policy` | 59 | 59 | 0 |
| `prikk` (prikk-cli) | 27 passed, 1 ignored | 27 passed, 1 ignored | 0 |

Identical across every crate, across four separate splits — this is the increment's correctness
claim and it holds throughout, not just in aggregate at the end.

## What did not change

- No public module path or public API changed anywhere — `cargo build --workspace --locked` succeeds
  identically before and after.
- No identity artifact changed: `vectors/snapshot.txt`, `vectors/hard.rs`,
  `state_root/tests/vectors.rs`, `text_span/vectors.rs` — no diff, checked after every individual
  split, not once at the end.
- `node_authoring.rs` untouched.
- Locked package count and `Cargo.lock` unchanged.

## Gate output

All green, both toolchains, run after all four splits together:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --locked` — all crates pass, counts above
- `cargo +1.85.0 test --workspace --locked` — identical counts
- `git diff --check`
- release-policy `check` — all 154 oracle cases passed
- release-policy `boundary-check --format json` — `valid: true`
- release-policy `reference-check --format json` — `valid: true`

## Acceptance criteria, against the accepted RFC's list (final)

1. Source-structure report committed and complete. **Met.**
2. Test-support exclusions enumerated with reasons, including `vectors/hard.rs` and
   `frozen_outgoing.rs`, plus the newly-discovered `cache_ladder.rs`. **Met.**
3. Every file over 300 has a recorded split decision; every file over 500 is split or carries an
   accepted cohesion exception. **Met** — 5 of 6 over-500 files split, the sixth
   (`node_authoring.rs`) deferred by explicit design rather than exempted.
4. The 3 inline `mod tests` blocks relocated. **2 of 3**, third excluded with reasoning (permanent).
5. Public module paths and observable behaviour unchanged. **Met**, evidenced above, per-split.
6. Full gate set and test counts before/after. **Met.**
