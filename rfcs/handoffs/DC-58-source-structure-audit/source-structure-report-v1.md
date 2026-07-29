# DC-58 Source Structure Report v1

**Date:** 2026-07-29 (batch 1), 2026-07-30 (batch 2, this update).
**Methodology.** ELOC = total line count (`wc -l`) per file. Verified against the RFC's own worked
examples before trusting it: `lifecycle_cache.rs` 974, `patch_replay/decode.rs` 733,
`payload/patch.rs` 652 — all three match `wc -l` exactly, confirming this is the intended metric
(not a comment/blank-stripped count).

**Scope.** "Implementation files" means every `.rs` file reachable from a crate's `lib.rs`/`main.rs`
via `mod` declarations **without ever passing through a `#[cfg(test)]`-gated edge**, computed by a
full transitive module-graph walk (not a path-name heuristic) across all 8 workspace-member and
tool crates. Verified complete: every `.rs` file on disk under each crate's `src/` is reached by the
walk exactly once, with none left over.

## Correction to the inherited baseline

The handoff's stated baseline (7 files over 500, 16 between 300-500 — "measured 2026-07-29, re-measure
rather than inheriting these") does not reproduce under implementation-only scoping. Properly
scoped, at batch-1 measurement time: **6 files over 500, 14 between 300-500**.

The discrepancy is explained, not just observed: 6 (the corrected over-500 count) +
`crates/prikk-object/src/vectors/hard.rs` (624 lines, `#[cfg(test)]`-gated DC-41/DC-55 evidence) = 7,
matching the inherited figure exactly. The most likely explanation is that the original count was a
raw sweep of all `.rs` files under `src/`, taken before applying the test-exclusion rule this RFC's
own Step 3 mandates — i.e., the inherited baseline is what this audit's scoping rule exists to
correct, not a measurement error to match. This report uses the corrected, implementation-only
numbers throughout.

## Test-support exclusions

Every file reachable only through a `#[cfg(test)]` edge is excluded from the thresholds below, by
the blanket rule the RFC states. Two exclusions carry additional evidentiary weight and are named
explicitly, per the RFC's specific instruction:

- **`crates/prikk-object/src/vectors/hard.rs`** (624 lines) — `#[cfg(test)]`-gated
  (`crates/prikk-object/src/lib.rs:16`). DC-41 and DC-55 identity evidence: frozen golden vectors.
  Splitting it for a line-count target would fragment the evidence base DC-41/DC-55 established.
  **Out of scope, permanently, not just for this pass.**
- **`crates/prikk-hash/src/tests/frozen_outgoing.rs`** (144 lines) — `#[cfg(test)]`-gated, DC-55's
  differential reference. Its own module doc states it must never be edited. **Out of scope,
  permanently.** (Also relevant to the inline-test-module finding below.)

**A third joined this list during batch 2, as a direct consequence of a split rather than a
pre-existing oversight:** `crates/prikk-store/src/lifecycle_cache/cache_ladder.rs` (848 lines, new).
See "`lifecycle_cache.rs`" below for why.

The complete exclusion list (all `#[cfg(test)]`-reachable files, ~180 files across the workspace) is
mechanical and not individually re-justified here beyond the blanket rule; the three above are
called out because their names and content could otherwise look like production code to a future
auditor running a naive sweep, which is exactly the trap this report exists to prevent.

## Files over 500 ELOC (split, unless an accepted cohesion exception)

| File | ELOC (before) | Decision | Status |
|---|---:|---|---|
| `crates/prikk-store/src/lifecycle_cache.rs` | 974 | Split | **Done, batch 2** |
| `crates/prikk-store/src/patch_replay/decode.rs` | 733 | Split | **Done, batch 2** |
| `crates/prikk-object/src/payload/patch.rs` | 652 | Split | **Done, batch 2** |
| `crates/prikk-store/src/worktree_patch/node_authoring.rs` | 601 | **Deferred** — per handoff, until DC-56 records an outcome (DC-56 may restructure its traversal; DC-59 benchmarks the path through it as-is) | Not touched, by design |
| `crates/prikk-store/src/text_span.rs` | 552 | Split | **Done, batch 2** |
| `crates/prikk-store/src/patch_replay.rs` | 537 | Split | **Done, batch 1** |

**All splittable over-500 files are now resolved.** Only `node_authoring.rs` remains over 500,
deferred by explicit design, not oversight.

### `patch_replay.rs` — split, batch 1

- `patch_replay.rs` (245 lines) — public API and baseline resolution.
- `patch_replay/read.rs` (134, new) — object-store reading helpers.
- `patch_replay/apply.rs` (206, new) — per-operation state-fold logic.

### `patch_replay/decode.rs` — split, batch 2

Mirrors `patch_replay.rs`'s own seam (types/dispatch vs. per-kind logic vs. low-level parsing):

- `patch_replay/decode.rs` (251 lines) — `DecodedPatchOperation`, `DecodedOperationKind`,
  `DecodedDeletePreimage`, `ensure_apply_supported`, top-level dispatch (`decode_patch_operations`,
  `decode_operation`).
- `patch_replay/decode/operations.rs` (338, new) — the seven per-operation-kind decoders
  (`decode_create_file` … `decode_replace_binary`). Itself over 300; recorded below.
- `patch_replay/decode/tlv.rs` (168, new) — the canonical TLV cursor/field reader
  (`TlvCursor`, `TlvField`), used by both the dispatcher and every per-kind decoder.

`prikk-store` test count unchanged (543). No identity artifact touched — this is the decode side of
the same wire format DC-54/DC-55 already covers; the split moved no logic.

### `payload/patch.rs` — split, batch 2 (identity-adjacent, extra verification)

The canonical **encode** side mirroring the decoder above, in `prikk-object`:

- `payload/patch.rs` (326 lines) — `PatchPayload`, `PatchPurpose` (+ its own field cursor),
  `Operation`, `OperationKind`, the text-span helpers (`text_span_hash`, `validate_text_anchor_id`).
- `payload/patch/operations.rs` (349, new) — the seven per-operation-kind payload structs
  (`CreateFile` … `ReplaceBinary`), each with its `validate()` and `encode_canonical()`. Itself over
  300; recorded below.

All items stayed `pub` and are re-exported at `payload/patch.rs` (`pub use operations::{...}`), so
`payload::patch::CreateFile` and the crate-root re-export at `payload.rs` are byte-identical paths
to before. Verified with the same rigor as an identity-bearing change despite being a pure move:
`prikk-object` test count unchanged (76), and `git status --short` on `vectors/snapshot.txt` and
`vectors/hard.rs` shows no diff — the canonical encoding these types produce is unchanged because no
line of `encode_canonical`/`validate` logic was edited, only relocated.

### `text_span.rs` — split, batch 2 (DC-55 evidence path, extra verification)

- `text_span.rs` (229 lines) — the identity primitives shared by both replay and authoring:
  `locate_text_span`, `splice_text`, `text_blob_id`, `occurrences`, `compute_span_id`, the anchor
  hashes, both error types.
- `text_span/authoring.rs` (206, new) — deterministic span selection for worktree authoring
  (`plan_authored_text_span`, `AuthoredTextSpan`, `TextSpanSelectionError`,
  `anchor_filtered_dup_index` shared with `inverse.rs`).
- `text_span/inverse.rs` (149, new) — direct-inverse derivation (`derive_inverse_edit_text`).

`plan_authored_text_span` and `derive_inverse_edit_text` re-exported at `text_span.rs` so every
existing `text_span::X` caller across the crate (11 call sites in
`patch_inverse.rs`/`patch_algebra/*`/`worktree_patch/node_authoring.rs`/etc.) is unaffected.
Re-verified specifically: all 19 `text_span::*` tests pass, including every `fdd01_text_span_v*`
golden vector and the DC-12/DC-55-adjacent span-selection and inverse tests — not just the aggregate
543-test count.

### `lifecycle_cache.rs` — split, batch 2 (structural, not just line-count)

This file was unusual: on inspection, roughly 850 of its 974 lines were already `#[cfg(test)]`-gated
**item by item**, not as a whole module. Per the file's own doc comment, the entire
`DecodedLifecycleCache` → `ValidatedLifecycleCache` → `ComparedLifecycleCache` trust ladder is
scaffolding for blob-kind verification, provenance, and replay-compare — "later slices" not yet
wired into production. Only two resolver traits, the real store-backed resolver, the replay entry
points, and `ReplayDerivedLifecycleState` are genuinely always-compiled.

Rather than split by responsibility within "implementation," this split follows the file's own
existing compilation boundary:

- `lifecycle_cache.rs` (117 lines) — the true production surface: `BlobKindResolver`,
  `BlobContentResolver`, `mod store_resolvers` + `StoreBackedResolver` re-export, `mod replay`,
  `ReplayDerivedLifecycleState`, `replay_derived_state`.
- `lifecycle_cache/cache_ladder.rs` (848 lines, new) — the entire trust-ladder scaffolding, moved
  verbatim with per-item `#[cfg(test)]` attributes removed and replaced by gating the whole new
  module (`#[cfg(test)] mod cache_ladder;`). Re-exported (also `#[cfg(test)]`) so
  `lifecycle_cache::tests` and `lifecycle_cache/replay/tests.rs` keep resolving every `super::X` /
  `crate::lifecycle_cache::X` path unchanged.

**This is not merely a line-count fix.** `cache_ladder.rs` is now reached only through a
`#[cfg(test)]` edge, so this report's own module-graph walk correctly reclassifies it as test-support
— the same category as `vectors/hard.rs`. 848 lines that were miscounted as "implementation" against
this audit's own methodology are now counted correctly. Added to the exclusion list above.

Required a handful of `pub(super)`/`pub(crate)` visibility widenings (`encode_unchecked`,
`CACHE_SCHEMA_VERSION`, and re-exporting `certified_compared_cache` and `BlockParentResolver`) to
keep two separate test files (`lifecycle_cache/tests.rs` and `lifecycle_cache/replay/tests.rs`)
compiling against the moved items — caught immediately by `cargo test` failing to compile, fixed,
re-verified. `prikk-store` test count unchanged (543).

## Files between 300 and 500 ELOC (recorded decision required)

Three files joined this band as a direct, expected consequence of the splits above (each is a
cohesive single-purpose file under the 500 mandatory-split line, so "leave as is" applies the same
as the pre-existing 14):

| File | ELOC | Decision |
|---|---:|---|
| `crates/prikk-cli/src/main.rs` | 497 | Leave as is — CLI dispatch table; many small `run_*` handlers are the intended shape for an entry point, not accidental bulk. **Re-measured at the start of this batch per review N1 — still 497, three lines under the line; watch on every future batch that touches CLI surface** |
| `crates/prikk-cli/src/args.rs` | 411 | Leave as is — argument parsing for every subcommand in one place is more auditable than scattered per-command parsers |
| `crates/prikk-store/src/lifecycle_cache/replay.rs` | 404 | Leave as is — untouched by this batch's `lifecycle_cache.rs` split (that split followed the existing cfg(test) boundary, not this file); still a cohesive lineage-walk implementation |
| `crates/prikk-store/src/refs.rs` | 384 | Leave as is — already the thin top of a `refs/` module tree |
| `crates/prikk-store/src/layout.rs` | 380 | Leave as is — one cohesive responsibility (path accessors) |
| `crates/prikk-store/src/wal.rs` | 376 | Leave as is — single-responsibility WAL read/write/replay logic |
| `crates/prikk-store/src/doctor.rs` | 369 | Leave as is — diagnostic checks naturally enumerable in one place |
| `crates/prikk-store/src/patch_inverse.rs` | 362 | Leave as is — already has a `patch_inverse/read.rs` sibling |
| `crates/prikk-object/src/payload/patch/operations.rs` | 349 | Leave as is (new this batch) — the seven per-operation-kind payload structs; splitting further (e.g. one file per kind) would trade one readable file for seven tiny ones with no cohesion gain |
| `crates/prikk-store/src/patch_replay/decode/operations.rs` | 338 | Leave as is (new this batch) — same reasoning as above, the decode-side mirror |
| `crates/prikk-object/src/payload/refs.rs` | 335 | Leave as is — one payload type's full codec, consistent with sibling payload files |
| `crates/prikk-store/src/verify.rs` | 333 | Leave as is — already has `verify/objects.rs`, `verify/ref_publication.rs`, `verify/trust.rs` siblings |
| `crates/prikk-object/src/payload/patch.rs` | 326 | Leave as is (reduced from 652 this batch) — envelope types plus the `PatchPurpose` field cursor; cohesive |
| `tools/release-policy/src/oracle/self_test.rs` | 318 | Leave as is — already has four siblings |
| `tools/release-policy/src/oracle/self_test/matrix.rs` | 318 | Leave as is — one cohesive matrix-construction responsibility |
| `crates/prikk-cli/src/output.rs` | 313 | Leave as is — already split into four submodules |
| `tools/release-policy/src/policy/evidence.rs` | 303 | Leave as is — already has a two-file split |

None of these 17 propose a cohesion *exception* in the RFC's formal sense (that term applies only to
files over 500 ELOC that would otherwise require splitting); each is a "leave as is" decision at or
under the 500-line mandatory-split threshold, which the RFC allows to be recorded without separate
acceptance.

## Inline `mod tests` blocks

Three found in batch 1, matching the handoff's re-measured count exactly:

| File | Action |
|---|---|
| `crates/prikk-object/src/id.rs` | **Relocated** to `crates/prikk-object/src/id/tests.rs`, batch 1 |
| `crates/prikk-object/src/canonical.rs` | **Relocated** to `crates/prikk-object/src/canonical/tests.rs`, batch 1 |
| `crates/prikk-hash/src/tests/frozen_outgoing.rs` | **Excluded, not relocated** — itself `#[cfg(test)]`-gated DC-55 evidence under the same exclusion as `vectors/hard.rs`. Its module doc forbids editing it; moving its internal test block would still be editing it. |

All three resolved as of batch 1; unchanged in batch 2.

## What did not change (either batch)

- No public module path or public API changed anywhere. `cargo build --workspace --locked` succeeds
  with the same public surface after every split.
- No identity artifact changed across both batches: `crates/prikk-object/src/vectors/snapshot.txt`,
  `crates/prikk-object/src/vectors/hard.rs`, `crates/prikk-store/src/state_root/tests/vectors.rs`,
  `crates/prikk-store/src/text_span/vectors.rs` — none touched, confirmed via `git status --short`
  after every individual split, not just once at the end.
- `node_authoring.rs` untouched, as deferred.
- No test weakened, deleted, or disabled to satisfy a line-count target.

## Final status against DC-58's definition of done

| Requirement | Status |
|---|---|
| Committed source-structure report | **This document.** |
| Test-support exclusions enumerated with reasons | **Done** — 3 explicitly named (2 pre-existing, 1 discovered by the `lifecycle_cache.rs` split) |
| `node_authoring.rs` recorded as deferred with reason | **Done** |
| Every file over 300 has a recorded split decision | **Done** — every over-500 and every 300-500 file (23 total across both batches) has a recorded decision in this document |
| Every file over 500 split or carrying an accepted cohesion exception | **Done** — 5 of 6 split (across both batches); 1 (`node_authoring.rs`) deferred by explicit design, not an unresolved item |
| Inline `mod tests` blocks relocated | **2 of 3**, third excluded with reasoning recorded (permanent, not a gap) |
| Public module paths and observable behaviour unchanged | **Verified** — test counts identical after every split, all gates green, both toolchains |
