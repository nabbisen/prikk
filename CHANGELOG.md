# Changelog

## 0.1.3 — Documentation / release hygiene

Documentation-only release. No source code change; identity anchors unchanged
(empty-PATCH `510ab866…5157`, populated `24031b48…c854`).

- Replaced `README.md` (maintainer-updated).
- Folded the v0.1.2 release-note errata: the four ignored `prikk-store` tests are now
  explained as DEV-ONLY worktree-authoring checkpoint tests, and worktree-authoring re-enable
  is added to the carry-forwards.
- CHANGELOG hygiene: removed a duplicate top heading and consolidated the v0.1.2 sub-slices
  under a single `0.1.2` release heading.

## 0.1.2 — DC-09 Phase 4.3 / 4.4 internal node-model groundwork

Internal/unwired node-model groundwork: store decode-model promotion, the node-lifecycle
substrate, and the lifecycle-cache trust ladder. Not consumed by any command path; identity
anchors unchanged.

### Phase 4.4 step 2b.2R-2 — create_node nonzero guard

Pre-threading substrate hardening from the 2b.2R review (P2): `NodeLifecycleState::create_node`
now rejects the reserved all-zero `node_id` at the central node-introduction boundary,
matching `seed_live_node` / `seed_tombstone`, instead of relying on decode/generator
correctness. Validation-only; both anchors unchanged. Test: `create_node_rejects_all_zero_node_id`
(restoration-equivalent re-create with a nonzero id continues to clear the tombstone).

### Phase 4.4 step 2b.2R — live/tombstone overlap closure

Closes a substrate P1 found in the steps 3–4 review: `NodeLifecycleState` could hold a node
as both live and tombstoned after delete → restoration-equivalent re-create, which violated
the cache's no-overlap invariant and would make replay-and-compare reject a correct post-
restore cache. Model/validation correction only; both anchors unchanged.

- `create_node` now clears any tombstone for the node on a restoration-equivalent
  reintroduction, so live and tombstone sets stay disjoint (no-op for a fresh node_id).
- `NodeLifecycleState::validate_internal_consistency` now rejects any node_id present in both
  the live and tombstone sets.
- `ReplayDerivedLifecycleState::from_replay` now returns `Result` and validates internal
  consistency, so the compared rung cannot certify against a malformed reference state.
- Tests: substrate `create -> delete -> restore` leaves the node live with no overlap and
  passes consistency; a post-restore baseline cache (node live, no tombstone) compares equal
  to the replayed state.

### Phase 4.4 step 2b.2-3/4 — replay-derived + compared rungs

Adds the top trust rungs and the decisive right-provenance/false-tombstone guarantee. Still a
private, unwired slice — no apply/seal/verify path consumes a cache. Additive; both anchors
unchanged.

- **`ReplayDerivedLifecycleState`** — an authoritative replay-derived `NodeLifecycleState`
  bound to a baseline. Must be produced only by authoritative replay over the walked chain;
  the real producer arrives with threading, so this slice constructs it via `from_replay`.
- **`ComparedLifecycleCache`** — a validated cache **proven equal** to authoritative replay
  for the same baseline. `from_validated_and_replay` checks the baseline matches, rebuilds a
  `NodeLifecycleState` from the validated cache, and requires it to equal the replayed state.
  This is the only cache-derived rung that may participate in restoration-equivalence /
  `node_id` reuse decisions once wired — and only because it equals replay.
- **Decisive guarantee:** a cache with correct provenance but false live/tombstone contents
  is rejected — the rebuilt state will not equal the replayed state (test:
  `compared_rejects_false_tombstone`).
- **P2-2 closed:** `ValidatedLifecycleCache::from_decoded_for_baseline` binds a cache to the
  caller's intended baseline, so a cache valid for one checkpoint cannot be accepted where a
  different baseline was meant.
- `NodeLifecycleState` now derives `PartialEq`/`Eq` for the replay comparison.
- Carry-forwards still open: P2-1 (real store resolver must distinguish a missing/unreadable
  block from genesis — applies when the real resolver lands in threading) and P2-3 (structured
  error taxonomy before recovery/doctor branches on classes).

### Phase 4.4 step 2b.2-2 — walked-chain provenance

Makes lifecycle-cache provenance **operational**: the `replay_window_hash` is recomputed
over the actually walked single-parent block chain, never over cache-supplied data. Still a
private, unwired slice — no apply/seal/verify identity decision uses a cache. Additive; both
anchors unchanged.

- **`BlockParentResolver`** — a `block_id -> Vec<ObjectId>` seam (parents in seal order;
  empty at genesis), mirroring `BlobKindResolver`. Keeps the walk testable without a store
  handle; the real `Block`-reading resolver arrives with threading.
- **`DecodedLifecycleCache::verify_window_against_chain`** — walks `baseline_block_id` back
  to `lineage_horizon_id` over single-parent links and recomputes the window hash from the
  walked chain. Fails closed on a merge (multi-parent) block, a cycle, reaching genesis
  before the claimed horizon, a horizon that is not repository genesis (v1 adequate-horizon
  rule), or a hash mismatch.
- **`ValidatedLifecycleCache::from_decoded`** now also runs provenance verification (it takes
  both a blob and a block-parent resolver), so the `Validated` rung means structural +
  operational-provenance + blob-kind verified — design-v3's definition — and cannot exist
  with merely syntactic provenance.
- Tests: matching walked chain accepted; window-hash mismatch, merge block, non-genesis
  horizon, cycle, and genesis-before-horizon each rejected.

### Phase 4.4 step 2b.2-1 — blob-kind verification + Validated rung

Opens 4.4-2b.2 proper with the first blob-kind verification step and the first trust rung.
Still a private, unwired codec/import slice — no apply/seal/verify identity decision uses a
cache. Additive; both anchors unchanged.

- **`BlobKindResolver`** — a small `blob_id -> Option<BlobKind>` trait. `Ok(None)` means the
  blob is absent/unreadable and fails closed. Keeps verification testable without a store
  handle; a real store resolver arrives with the threading slice.
- **`ValidatedLifecycleCache`** — the first trust rung: a `DecodedLifecycleCache` whose every
  file entry's `NodeKind` has been checked against the referenced blob's `BlobKind`, reusing
  the canonical `NodeKind::from_file_blob_kind` rule. `from_decoded` **re-runs structural
  validation itself** (the input is not trusted to have come from `decode`, since fields are
  `pub(crate)`), then verifies blob kinds; a missing blob, a kind disagreement, a `SNAPSHOT`
  blob, or a resolver error fails closed. It is documented and structured as **not authority**
  for `node_id` reuse or restoration-equivalence — there is no method that yields such a
  decision; those wait for the replay-derived / replay-compared rungs.
- Tests: structural-invalid input rejected even when blob kinds resolve; Text and Binary
  matches accepted; Text↔Binary disagreement rejected; `SNAPSHOT` blob rejected; missing blob
  rejected; tombstone blob-kind mismatch rejected; resolver error propagated fail-closed.
- Review follow-ups: added explicit tombstone kind/content production-encode negatives (N1);
  verified `read_enum_u16` guards the wire type exactly once — the apparent double was two
  distinct call sites (node_kind tag 3, parent_policy tag 4), nothing to remove (N2).

### Phase 4.4 step 2b.2 — lifecycle cache codec hardening

Corrective patch opening 4.4-2b.2, closing the 4.4-2b.1 review errata. Validation-only;
no new data, no wiring into replay; both anchors unchanged.

- **P1 — production `encode()` validates before writing.** `encode()` now runs the same
  structural/cross-set `validate()` as `decode()` before serializing, so an internal
  caller cannot persist a cache the importer would later reject. `validate()` is now
  **structurally equivalent** to the decode path: beyond schema/policy/sorting/uniqueness
  and `seen_ids == live ∪ tombstoned`, it rejects the reserved all-zero `node_id` in live,
  tombstone, and `seen_ids` sets and rejects any kind/content discriminator mismatch,
  reusing the substrate's `ensure_node_id_nonzero` and `validate_kind_content_shape`
  (promoted to `pub(crate)`) rather than a parallel rule. The raw serializer is private and
  reachable in production only through the validated `encode`; a `#[cfg(test)]`
  `encode_unchecked` is used to craft malformed fixtures for decode negatives. Production
  encode is proven to reject unsorted live entries, a `seen_ids` mismatch, merge policy,
  all-zero ids (live/tombstone/seen), and file↔symlink kind/content mismatches.
- **P2-1 — non-canonical TLV tag order rejected.** Decode now requires nondecreasing field
  tags at both the top level and inside each node record (repeated tag 10/11 entries still
  allowed in-region), so a persisted cache has one canonical byte form. Tests cover a
  header field and a node-record field presented out of order.
- **P2-2 (error taxonomy)** remains a message-class mapping for now, per the review — to be
  promoted to a structured class before any recovery/doctor path depends on the outcomes.

### Phase 4.4 step 2b.1 — lifecycle cache codec

Adds the persisted lifecycle-cache wire format and its decoder/importer
(`lifecycle_cache`), a derived, rebuildable accelerator for `NodeLifecycleState`. Per
design v3 §0 the decoded value is **not validation authority**: it cannot seed a
`node_id`-reuse decision. Additive and identity-neutral; not wired into replay; both
anchors unchanged.

- `DecodedLifecycleCache::{encode, decode}` over `PRIKK-NODE-LIFECYCLE-CACHE-v1\0` magic
  plus canonical `FieldRecord` TLV. Wrong/short magic is rejected before TLV decode;
  repeated live/tombstone entries use `record_list_item` (`0x21`) and an entry sent as a
  plain `record` (`0x20`) is rejected.
- Fail-closed structural + cross-set validation: unknown top-level/nested tags, duplicate
  singleton fields, file/symlink discriminator (files require `blob_id`+`normalized_mode`
  and forbid a target; symlinks require a target and forbid `blob_id`/field 5 even when
  zero), live entries strictly sorted by canonical `repo_path` with unique path and id,
  tombstones strictly sorted by raw `node_id`, `seen_ids` a multiple of 32 / strictly
  ascending / nonzero, no id both live and tombstoned, and `seen_ids == live ∪ tombstoned`.
- `compute_window_hash` fixes the exact `replay_window_hash` preimage
  (`PRIKK-LIFECYCLE-CACHE-WINDOW-v1 || u64be(count) || raw32(block_id)…`): deterministic,
  count-bearing, order-sensitive, domain-separated.
- Blob-kind verification, provenance-vs-baseline staleness, replay reconstruction, and
  replay-and-compare are deferred to the next slice; no `ValidatedLifecycleCache` /
  `ReplayDerivedLifecycleState` / `ComparedLifecycleCache` ladder is exposed yet, so no
  type here can be mistaken for replay-derived authority.

### Phase 4.4 step 2a — baseline seeding substrate

Adds the baseline-seeding API to `NodeLifecycleState` and closes the substrate-level
4.4-2 errata, so a baseline cache cannot inject node state an operation could not.
Additive and identity-neutral; both anchors unchanged.

- `seed_live_node` / `seed_tombstone` seed the live clean tree and the non-live
  lifecycle history (`seen_ids` + `latest_tombstone_by_id`) needed for
  restoration-equivalence across a snapshot boundary. Both reject the reserved
  all-zero `node_id` (erratum P1-3), validate the kind/content discriminator through a
  shared `validate_kind_content_shape` (erratum P2-2), and reject duplicate live ids,
  duplicate live paths, and tombstones for currently-live nodes.
- `validate_internal_consistency` now also requires every live and every tombstoned
  `node_id` to be recorded as seen (erratum P1-4, whole-state check).
- `rename_node` gains the same path-index lockstep guard as `delete_node` (erratum
  P2-1), failing closed rather than silently healing a desynchronised index.
- Tests raised to 24: cross-boundary restoration-equivalence accept and non-equivalent
  reject (the identity-resurrection case), all-zero seed rejection, duplicate id/path
  rejection, tombstone-for-live rejection, and seed kind/content rejection.
- Deferred to the next slice (cache format + threading): cache provenance/staleness
  binding (P1-1), the materialization-bytes vs lifecycle-identity payload split (P1-2),
  the symlink `normalized_mode == 0` parse check, and threading `NodeLifecycleState`
  through replay/inverse/rollback.

### Phase 4.4 step 1 — node lifecycle substrate

Introduces the node-aware replay substrate. Additive and identity-neutral: a new
isolated module with no changes to any object/payload/encode path; both identity
anchors are unchanged.

- Added `prikk-store::node_lifecycle`: a derived, rebuildable `NodeLifecycleState`
  (`live_by_id` / `path_to_id` / `latest_tombstone_by_id` / `seen_ids`) that is
  explicitly **not a root of trust** (FDD-02 §12). It centralises the node rules so
  replay/inverse/rollback cannot diverge on them: per-`CleanTree` live-node
  uniqueness, rejection of currently-live `node_id` reuse, restoration-equivalence of
  a non-live reintroduced `node_id` to its latest deletion preimage (kind, content
  payload, mode, path — non-liveness necessary but not sufficient, DC-09a §4), and
  `node_id` preservation across rename.
- Review errata: `create_node` fails closed on an inconsistent kind/content
  discriminator (symlink-as-file or file-as-symlink); the path index is keyed by the
  canonical `RepoPath`; `delete_node` enforces `live_by_id`/`path_to_id` lockstep; and
  a `validate_internal_consistency()` helper checks the live-node bijection (for
  assertions and the 4.6 deep-verify validator).
- 17 unit tests covering uniqueness, live-reuse rejection, restoration-equivalence
  (file accept plus blob/mode/path/kind-mismatch rejects; symlink target match and
  mismatch), kind/content discriminator rejection, rename id-preservation and
  occupied-target rejection, non-live delete/rename rejection, and the consistency
  helper.
- The module is `dead_code`-allowed at declaration: it is threaded into the replay
  pipeline in the next 4.4 step (which first settles how the clean-tree baseline
  carries node identity).

### Phase 4.3 — store decode-model promotion

Promotes the store patch decoder from a two-variant, path-keyed supported subset
into a typed node-addressed stream over all seven FDD-03 §9.3 operation kinds.
Identity-neutral: the empty-PATCH anchor and populated framing vector are unchanged.
Applies design-review errata P1 (decode success must not imply apply support) and P2
(retain validated `op_seq`).

- Replaced `SupportedPatchOperation` with `DecodedPatchOperation { op_seq, kind }` and a
  seven-variant `DecodedOperationKind` (plus a discriminated `DecodedDeletePreimage`),
  and renamed `decode_supported_patch_operations` -> `decode_patch_operations`. Every
  well-formed §9.3 kind now decodes into its typed variant; symlink `DeleteNode` and the
  four other node-addressed kinds are no longer rejected at decode time.
- Added `ensure_apply_supported` as the single apply-support gate (erratum P1): decode
  is structural, applicability is a separate decision. Audited all callers — `patch_replay`
  apply and `patch_inverse` derivation gate before matching; `rollback_verify` now gates
  each decoded operation explicitly rather than relying on decode success to prove
  replayability.
- Retained validated `op_seq` in the decoded wrapper (erratum P2).
- Migrated decode tests: malformed/oneof/all-zero-`node_id`/wrong-wire negatives remain
  decode errors; each of the seven well-formed kinds asserts its typed decoded variant
  **and full decoded field values** (review erratum E1, so 4.4 application can depend on
  them), and the not-yet-wired kinds assert `UnsupportedObjectType` at the apply gate.

## 0.1.1 Housekeeping

Repository structure and developer-ergonomics pass. No identity-byte or behavior
changes; the empty-PATCH anchor and populated framing vector are unchanged.

- Relocated `prikk-store` unit tests from the central `src/tests/` directory to the
  project-standard co-located layout: `src/<module>/tests.rs` (and
  `src/patch_replay/tests/` for the three patch-decode test modules). Shared fixtures
  and cross-module harnesses moved to a single `src/test_support.rs`.
- Added `rfcs/proposed/` with a node-model plan RFC capturing the deferred
  application work (4.3–4.6) and the tracked carry-forward items (symlink target
  validator, duplicate scalar-field rejection, preconditions).
- Aligned the workspace `Cargo.toml` version (`0.1.0` -> `0.1.1`) with the active
  CHANGELOG line.
- Made the worktree-patch test module pass the CI clippy gate
  (`cargo clippy --workspace --all-targets -- -D warnings`): targeted
  `#[allow(clippy::indexing_slicing)]` on the four DEV-ONLY authoring-checkpoint tests
  (deliberate `Vec` indexing in assertions) and removed a needless borrow on a
  byte-slice literal.

## 0.1.0 DC-09 Phase 4.2

Operation-record identity reconciliation to FDD-03 §9.3 (code reconciliation effort,
architect-ratified across increments 4.2a–4.2e). Identity/read-validation surface
only; application of node-addressed operations is deferred to the node model.

- Reconciled all seven operation-kind payloads to their FDD-03 §9.3 records: `CreateFile`,
  `DeleteNode` (was `DeleteFile`), `EditText` (node-addressed, span-anchored, 9-field),
  `ReplaceBinary` (node-addressed), `RenamePath`, `ChangePerm`, `CreateSymlink` — all
  node-bearing records reject an all-zero `node_id` on encode and decode.
- Enforced the FDD-03 §9.2 operation-kind oneof on the read path (a record with more
  than one kind field is rejected as malformed, not decoded last-wins).
- Enforced the FDD-03 §9.2.1 `op_seq` canonical invariant on the read path
  (one-based, contiguous, unique, physical order == ascending `op_seq`).
- Added the `ReplaceBinary` binary-only blob-kind enforcement primitive
  (`ensure_blob_kind_is_binary`); wiring into real application is deferred to the node model.
- Retired the pre-FDD full-file `EditText` apply/inverse path and its worktree generation.
- **Worktree patch authoring (`commit --from-worktree`) is fail-closed in this snapshot**
  for create/delete/modify/text changes: every §9.3 mutation operation is node-addressed
  and requires node-id tracking and minting (deferred to increments 4.4/4.4a). This release
  does not support worktree authoring; replay of node-addressed operations is likewise deferred.
- Byte-level `(tag, value_type)` layout tests and read-side validator negatives added for
  every operation record; empty-PATCH anchor and the populated framing vector held throughout.

## 0.1.0 PR-030

Sealed rollback block/history classification after normal seal.

- Added sealed rollback block classification after rollback drafts are sealed by the existing seal path.
- `load_ref_history()` now reports `rollback_patch_count` and `is_rollback_block` for each history entry.
- `prikk log` now displays rollback block classification for sealed history entries.
- `verify_repository()` now counts sealed rollback blocks and sealed rollback-marked Patch objects.
- `prikk verify` now displays sealed rollback block and rollback Patch counts in addition to active rollback draft WAL records.
- Shared rollback Patch payload validation between active WAL verification and sealed Block/history classification.
- Fixed an obvious duplicate-parameter transcription defect in inverse planning source while touching rollback-adjacent code.
- Kept rollback-specific ref publication, rollback authorization, worktree rollback writes, arbitrary-span rollback, commutation, confluence, audit plugins, and sync deferred.

## 0.1.0 PR-029

Active rollback draft verification before seal.

- Added active rollback draft verification for the supported patch-operation subset.
- Added `verify_active_rollback_draft()` and `RollbackDraftVerification`.
- Added CLI command `prikk rollback-draft-verify [path] [--ref REF]`.
- Rollback drafts now use a dedicated development signature marker key: `dev-placeholder-rollback-author`.
- Repository verification now counts rollback draft WAL records and validates that rollback draft payloads decode under the supported replay subset.
- Kept seal publication, rollback refs, rollback authorization, worktree mutation, arbitrary-span rollback, commutation, confluence, audit plugins, and sync deferred.

## 0.1.0 PR-028

Conservative rollback draft append to an empty active WAL.

- Added conservative rollback draft append for the supported patch-operation subset.
- Added `append_rollback_draft()` and `RollbackDraftReport`.
- Added CLI command `prikk rollback-draft --append-inverse [path] [--ref REF] -m <message>`.
- Requires an explicit `--append-inverse` flag, a non-empty message, an empty active WAL, and no partial WAL tail.
- Appends a signed inverse Patch envelope to the active WAL only; ref publication remains the existing `seal --allow-no-audit` path.
- Kept rollback ref policy, authorization, worktree mutation, arbitrary-span rollback, commutation, confluence, audit plugins, and sync deferred.

## 0.1.0 PR-027

Non-mutating rollback preview for the supported patch-operation subset.

- Added non-mutating rollback preview for the supported patch-operation subset.
- Added `prepare_rollback_preview()` and `RollbackPreviewPlan`.
- Added CLI command `prikk rollback-preview [path] [--ref REF]`.
- Combines unsigned inverse planning with supported patch replay validation.
- Compares the current replayed target state with the latest snapshot baseline and reports `would-create`, `would-delete`, and `would-replace` file-level changes.
- Kept rollback refs, authorization policy, worktree writes, commutation, confluence, arbitrary-span rollback, audit plugins, and sync deferred.

## 0.1.0 PR-026

Read-only inverse planning for the supported patch-operation subset.

- Added read-only inverse planning for the supported patch-operation subset.
- Added `prepare_patch_inverse_plan()` and `PatchInversePlan`.
- Added CLI command `prikk inverse-plan [path] [--ref REF]`.
- Derives unsigned inverse Patch payloads for supported `CreateFile`, `DeleteFile`, `ReplaceBinary`, and full-file `EditText` operations.
- Reports an unsigned inverse Patch ID hint without writing or publishing it.
- Kept rollback refs, authorization policy, conflict witnesses, commutation, confluence, arbitrary-span inverse handling, audit plugins, and sync deferred.

## 0.1.0 PR-025

Opt-in full-file `EditText` generation from UTF-8 worktree modifications.

- Added opt-in full-file `EditText` generation from snapshot-baseline worktree modifications.
- Added `WorktreePatchCommitOptions` and `commit_worktree_changes_with_options()`.
- Added CLI support for `prikk commit --from-worktree --text-edits -m <message>`.
- Kept default `commit --from-worktree` behavior compatible: modified tracked files still emit `ReplaceBinary` unless text mode is requested.
- Text mode emits `EditText` only when both baseline and current file bytes are valid UTF-8; binary or invalid UTF-8 modifications fall back to `ReplaceBinary`.
- Added worktree patch tests for text edit emission and binary fallback.
- Kept arbitrary span discovery, text diff minimization, inverse, commutation, conflict witnesses, audit plugins, and sync deferred.

## 0.1.0 PR-024

Conservative full-file `EditText` replay for exact-span replacements.

- Added conservative `EditText` replay for full-file exact-span replacements.
- Added canonical decode support for `EditText` patch operations in the supported patch replay decoder.
- Added `full-file` anchor replay validation: current file bytes must be valid UTF-8 and must hash to the recorded `old_span_hash`.
- Split supported patch-operation decoding into `patch_replay/decode.rs` to keep the replay module within the project file-size guidance.
- Added a patch replay test for full-file text edit replay.
- Kept worktree text diff generation, arbitrary span discovery, inverse, commutation, conflict witnesses, audit plugins, and sync deferred.

## 0.1.0 PR-023

Explicit patch deletion planning and opt-in removal of files deleted by supported patch replay.

- Added a content-anchored text edit payload validation scaffold.
- Added fixed `TEXT_SPAN_HASH_BYTES = 32` and `text_span_hash(bytes)`.
- Added `validate_text_anchor_id()` for v1 anchor identifier validation.
- Changed `EditText.old_span_hash` to a fixed 32-byte value.
- Added tests for anchor validation, stable span hashing, and invalid anchor rejection.
- Fixed a replay-source transcription defect in the supported `ReplaceBinary` branch.
- Kept worktree text diff generation, text replay, inverse, commutation, conflict witnesses, audit plugins, and sync deferred.

## 0.1.0 PR-022

Explicit patch deletion planning and opt-in removal of files deleted by supported patch replay.

- Added read-only explicit deletion planning via `prikk checkout --patch-delete-plan`.
- Added opt-in deletion during supported patch materialization via `prikk checkout --patch-materialize-delete`.
- Deletion is limited to files explicitly removed by replayed `DeleteFile` operations.
- Deletion is refused unless the current worktree file bytes still match the operation's old Blob bytes.
- Arbitrary untracked files and modified deleted files are never removed.
- Added deletion planning/materialization tests and documentation.
- Kept general destructive pruning, text edits, renames, chmod, symlinks, merge/conflict algebra, audit plugins, and sync deferred.

## 0.1.0 PR-021

Opt-in supported patch replay materialization without destructive removals.

- Added opt-in supported patch replay materialization via `prikk checkout --patch-materialize`.
- Added `materialize_patch_checkout()` and `PatchMaterializationReport`.
- Reuses the PR-020 supported replay subset: `CreateFile`, `DeleteFile`, and `ReplaceBinary`.
- Writes only validated replay-result files through the same conservative materializer used by snapshot checkout.
- Refuses conflicting existing files and never deletes extra worktree files.
- Keeps destructive removal, content-anchored text edit replay, renames, chmod, symlinks, merge/conflict algebra, audit plugins, and sync deferred.

## 0.1.0 PR-020

Minimal worktree-to-patch draft generation for missing, modified, and untracked files, still without patch replay or full algebra.

- Added read-only supported patch replay planning via `prikk checkout --patch-plan`.
- Added `prepare_patch_replay_plan()` and `PatchReplayPlan`.
- Replays single-parent block chains from oldest to newest.
- Loads snapshot Blob baselines and applies supported `CreateFile`, `DeleteFile`, and `ReplaceBinary` operations.
- Verifies `old_blob_id` preconditions for delete/replace operations.
- Keeps text-span edits, renames, chmod, symlinks, merge/conflict algebra, and worktree writes deferred.

## 0.1.0 PR-019

Minimal worktree-to-patch draft generation for missing, modified, and untracked files, still without patch replay or full algebra.

- Added minimal worktree-to-patch draft generation from snapshot-baseline changes.
- Added `prikk commit --from-worktree -m <message>`.
- Emits file-level `CreateFile`, `DeleteFile`, and `ReplaceBinary` operations only.
- Writes Blob objects referenced by generated operations before appending the Patch envelope to WAL.
- Keeps rename detection, content-anchored text-span edits, patch replay, audit plugins, and sync deferred.

## Earlier PRs

See `rfcs/IMPLEMENTATION-STATUS.md` and `rfcs/done/PR-*-HANDOFF.md` for earlier implementation history.
