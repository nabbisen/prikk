# DC-58 Source Structure Report v1

**Date:** 2026-07-29 (re-measured; see "Correction to the inherited baseline" below).
**Methodology.** ELOC = total line count (`wc -l`) per file. Verified against the RFC's own worked
examples before trusting it: `lifecycle_cache.rs` 974, `patch_replay/decode.rs` 733,
`payload/patch.rs` 652 — all three match `wc -l` exactly, confirming this is the intended metric
(not a comment/blank-stripped count).

**Scope.** "Implementation files" means every `.rs` file reachable from a crate's `lib.rs`/`main.rs`
via `mod` declarations **without ever passing through a `#[cfg(test)]`-gated edge**, computed by a
full transitive module-graph walk (not a path-name heuristic) across all 8 workspace-member and
tool crates. Verified complete: every `.rs` file on disk under each crate's `src/` (281 total) is
reached by the walk exactly once, with none left over.

## Correction to the inherited baseline

The handoff's stated baseline (7 files over 500, 16 between 300-500 — "measured 2026-07-29, re-measure
rather than inheriting these") does not reproduce under implementation-only scoping. Properly
scoped: **6 files over 500, 14 between 300-500** at measurement time, before this report's own split
work (see below).

The discrepancy is explained, not just observed: 6 (my over-500 count) + `crates/prikk-object/src/
vectors/hard.rs` (624 lines, `#[cfg(test)]`-gated DC-41/DC-55 evidence) = 7, matching the inherited
figure exactly. The most likely explanation is that the original count was a raw sweep of all `.rs`
files under `src/`, taken before applying the test-exclusion rule this RFC's own Step 3 mandates —
i.e., the inherited baseline is what this audit's scoping rule exists to correct, not a measurement
error to match. This report uses the corrected, implementation-only numbers throughout.

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
  permanently.** (Also relevant to the inline-test-module finding below — see there.)

The complete exclusion list (all `#[cfg(test)]`-reachable files, ~180 files across the workspace) is
mechanical and not individually re-justified here beyond the blanket rule; the two above are called
out because their names and content could otherwise look like production code to a future auditor
running a naive sweep, which is exactly the trap this report exists to prevent.

## Files over 500 ELOC (split, unless an accepted cohesion exception)

| File | ELOC (before) | Decision | Status |
|---|---:|---|---|
| `crates/prikk-store/src/lifecycle_cache.rs` | 974 | Split recommended | **Queued, next batch** |
| `crates/prikk-store/src/patch_replay/decode.rs` | 733 | Split recommended | **Queued, next batch** |
| `crates/prikk-object/src/payload/patch.rs` | 652 | Split recommended | **Queued, next batch** |
| `crates/prikk-store/src/worktree_patch/node_authoring.rs` | 601 | **Deferred** — per handoff, until DC-56 records an outcome (DC-56 may restructure its traversal; DC-59 benchmarks the path through it as-is) | Not touched, by design |
| `crates/prikk-store/src/text_span.rs` | 552 | Split recommended | **Queued, next batch** |
| `crates/prikk-store/src/patch_replay.rs` | 537 | **Split, done this batch** | Complete — see below |

### `patch_replay.rs` — split, committed this batch

Split into three files along existing natural seams (the file already had one submodule,
`decode.rs`, so this follows the file's own established pattern):

- `patch_replay.rs` (245 lines) — public API (`PatchReplayPlan`, `prepare_patch_replay_plan`) and
  baseline resolution (`resolve_worktree_baseline`, `resolve_node_lineage_bounds`,
  `WorktreeBaseline`, `replay_supported_patch_chain`).
- `patch_replay/read.rs` (134 lines, new) — object-store reading helpers: block-chain walking,
  blob/patch/snapshot loading. All `pub(super)`.
- `patch_replay/apply.rs` (206 lines, new) — per-operation state-fold logic
  (`apply_decoded_operation`, `apply_edit_text`, `ReplayLiveNode`). All `pub(super)`/private.

No item moved was edited beyond its visibility modifier (`pub(super)` where a sibling module now
needs to call it). `prikk-store` test count unchanged (543 before and after); `cargo clippy` and
`cargo fmt` clean.

### Remaining four over-500 files — queued, not yet split

Time-boxed within this batch to one fully-verified split plus the report itself, per the RFC's own
staging instruction ("report first, then splits in reviewable batches... each batch is
independently verifiable"). Brief structural notes for the next batch, from a first pass over each
file's top-level items (not yet a committed decision):

- **`lifecycle_cache.rs` (974)** — largest file in the workspace; likely splits along
  construction/query/invalidation lines, but deserves its own careful read before committing to a
  seam, given its size and that it already has sibling modules (`lifecycle_cache/replay.rs`,
  `lifecycle_cache/store_resolvers.rs`) whose relationship to the root file should inform the split.
- **`patch_replay/decode.rs` (733)** — per-operation-kind decoders; a strong candidate to split
  one-file-per-operation-kind (mirroring `patch_replay/apply.rs`'s per-operation match arms), but
  needs a check that the "single source of truth" gate (`ensure_apply_supported`) doesn't get
  duplicated or desynchronized across files.
- **`payload/patch.rs` (652)** — likely splits by payload variant, similar caution as above given
  this is canonical-encoding code with identity implications; any split here should be verified
  against `snapshot.txt` and `vectors/hard.rs` with the same rigor as DC-55's swap, even though no
  encoding logic changes.
- **`text_span.rs` (552)** — span localization and splice logic; DC-55's evidence campaign exercises
  this path, so a split should be re-verified against the existing text-span test vectors
  specifically, not just the aggregate pass count.

## Files between 300 and 500 ELOC (recorded decision required)

| File | ELOC | Decision |
|---|---:|---|
| `crates/prikk-cli/src/main.rs` | 497 | Leave as is — CLI dispatch table; many small `run_*` handlers are the intended shape for an entry point, not accidental bulk |
| `crates/prikk-cli/src/args.rs` | 411 | Leave as is — argument parsing for every subcommand in one place is more auditable than scattered per-command parsers |
| `crates/prikk-store/src/lifecycle_cache/replay.rs` | 404 | Leave as is — pending `lifecycle_cache.rs`'s own split above; revisit together |
| `crates/prikk-store/src/refs.rs` | 384 | Leave as is — already the thin top of a `refs/` module tree (`refs/log.rs`, `refs/publication.rs`, `refs/verify.rs`, `refs/evidence.rs`, `refs/pointer.rs` all already exist as siblings); this file is the coordinating root, not an unsplit monolith |
| `crates/prikk-store/src/layout.rs` | 380 | Leave as is — one cohesive responsibility (path accessors for `RepositoryLayout`); splitting path-accessor methods across files would reduce, not improve, auditability |
| `crates/prikk-store/src/wal.rs` | 376 | Leave as is — single-responsibility WAL read/write/replay logic |
| `crates/prikk-store/src/doctor.rs` | 369 | Leave as is — diagnostic checks are naturally enumerable in one place; a reviewer checking "does doctor cover X" benefits from one file |
| `crates/prikk-store/src/patch_inverse.rs` | 362 | Leave as is — already has a `patch_inverse/read.rs` sibling; root file is the inversion logic itself, cohesive |
| `crates/prikk-object/src/payload/refs.rs` | 335 | Leave as is — one payload type's full codec, consistent with sibling payload files (`blob.rs`, `block.rs`, etc.) which are similarly sized and not flagged |
| `crates/prikk-store/src/verify.rs` | 333 | Leave as is — already has `verify/objects.rs`, `verify/ref_publication.rs`, `verify/trust.rs` siblings; root file coordinates, doesn't duplicate |
| `tools/release-policy/src/oracle/self_test.rs` | 318 | Leave as is — already has `self_test/matrix.rs`, `self_test/profile.rs`, `self_test/candidate.rs`, `self_test/responsibility.rs` siblings |
| `tools/release-policy/src/oracle/self_test/matrix.rs` | 318 | Leave as is — one cohesive matrix-construction responsibility |
| `crates/prikk-cli/src/output.rs` | 313 | Leave as is — already split into `output/help.rs`, `output/merge_evidence.rs`, `output/verification.rs`, `output/worktree.rs`; root file is thin re-exports plus a few direct print functions |
| `tools/release-policy/src/policy/evidence.rs` | 303 | Leave as is — already has an `evidence/sequence.rs`, `evidence/governance.rs` split; root file is the coordinating evidence-check entry point |

None of these 14 propose a cohesion *exception* in the RFC's formal sense (that term applies only to
files over 500 ELOC that would otherwise require splitting); each is a "leave as is" decision at or
under the 500 line mandatory-split threshold, which the RFC allows to be recorded without separate
acceptance.

## Inline `mod tests` blocks

Three found, matching the handoff's re-measured count exactly (confirms the handoff's "it is three
files, not a campaign" correction was accurate):

| File | Action |
|---|---|
| `crates/prikk-object/src/id.rs` | **Relocated** to `crates/prikk-object/src/id/tests.rs`, this batch |
| `crates/prikk-object/src/canonical.rs` | **Relocated** to `crates/prikk-object/src/canonical/tests.rs`, this batch |
| `crates/prikk-hash/src/tests/frozen_outgoing.rs` | **Excluded, not relocated** — this file is itself `#[cfg(test)]`-gated evidence (DC-55's frozen differential reference) under the same exclusion this report applies to `vectors/hard.rs`. Its own module doc says it must never be edited; moving its internal test block would still be editing the file. Judgment call: the RFC's Step 4 didn't anticipate this overlap with Step 3's exclusion, but applying Step 3's own reasoning here is the more conservative reading. |

Both relocations preserve behaviour exactly (content moved verbatim; only the `mod tests { ... }`
inline body became `mod tests;` plus a sibling file). `prikk-object` test count unchanged (76 before
and after).

## What did not change

- No public module path or public API changed. `cargo build --workspace --locked` succeeds with the
  same public surface.
- No identity artifact changed: `crates/prikk-object/src/vectors/snapshot.txt`,
  `crates/prikk-object/src/vectors/hard.rs`, `crates/prikk-store/src/state_root/tests/vectors.rs`,
  `crates/prikk-store/src/text_span/vectors.rs` — none touched (confirmed via `git status --short`
  against this batch's diff).
- `node_authoring.rs` untouched, as deferred.
- No test weakened, deleted, or disabled to satisfy a line-count target.

## Batch status against DC-58's definition of done

| Requirement | Status |
|---|---|
| Committed source-structure report | **This document.** |
| Test-support exclusions enumerated with reasons | **Done**, including both explicitly-required files |
| `node_authoring.rs` recorded as deferred with reason | **Done** |
| Every file over 300 has a recorded split decision | **Done** — 20 of 20 (6 over-500 + 14 between 300-500) have a recorded decision in this document |
| Every file over 500 split or carrying an accepted cohesion exception | **Partial** — 2 of 6 resolved (1 split this batch, 1 deferred by design); 3 recommended-but-not-yet-split, queued explicitly as the next batch, not silently dropped |
| Inline `mod tests` blocks relocated | **2 of 3**, with the third's exclusion reasoned above |
| Public module paths and observable behaviour unchanged | **Verified** — test counts identical, gates green |
