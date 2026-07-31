# DC-65 Implementation Summary

Companion to `prerequisite-questions-v1.md` (§1's four answers, required reading first) and
`rfcs/accepted/DC-65-TEXT-EDIT-BASELINE-CONTENT.md`'s acceptance criteria.

## The fix, per site

**`crates/prikk-store/src/worktree_patch/node_authoring.rs` (`plan_edit_text`).** Was:
`read_file_blob_bytes(object_store, base.blob_id)` — a direct, unconditional object read. Now:
`current_text_for_node` tries the stored blob first (`read_file_blob_bytes_if_present`, `Ok(None)`
on a missing object, not an error), and on a miss falls back to
`crate::lifecycle_cache::materialize_edited_text` — a full replay of the node's lineage that returns
the node's materialized text from the accumulated `TextCache`, the same structure `apply_edit_text`
already builds internally. Requires the commit's `baseline_block_id`/`horizon_id`, threaded in from
`author_inner` (available whenever `baseline_files` is non-empty, which implies a `Published`
baseline — the `Genesis` case never reaches `plan_edit_text` at all, since there is nothing to edit).

**`crates/prikk-store/src/patch_algebra/evidence.rs` (`StorePatchAlgebraEvidence::baseline_text`).**
Same shape: on `read_blob` returning `Evidence::Missing`, falls back to the same
`materialize_edited_text`, converting a materialized result to `Evidence::Known` and a genuine replay
failure to `Evidence::Unreadable`. Required widening `baseline_block_id`/`lineage_horizon_id` from
`#[cfg(test)]`-only fields to always-present ones — they were already computed and passed at the
single production call site (`merge_evidence.rs:44`), just previously discarded outside test builds.

**`crates/prikk-store/src/lifecycle_cache/replay.rs`.** Added
`replay_lineage_with_materialized_text`, returning `(NodeLifecycleState, TextCache)` instead of
discarding the cache. `TextCache`'s type alias widened from private to `pub(crate)` so callers outside
this module can name it. `replay_chain_with_appended_patches` (the shared internal fold both
`replay_lineage` and `replay_with_appended_patches` call) now returns the pair too; both existing
callers just discard the second element — no behavior change for either.

## A fifth site, found only after the first three were fixed: DC-64's incremental step

Fixing `plan_edit_text` made a class of patch sealable for the first time — a text file's second (or
later) edit. That patch, once it exists, gets incrementally applied by `resolve_baseline_state`
(DC-64) on the *next* commit whenever the eligibility conditions hold — which they do starting at the
third real commit in the reproduction sequence. `apply_one_block`'s `TextCache` is fresh and empty for
a single-block step, unlike full replay's, which accumulates across the entire lineage from the true
start. A block containing an `EditText` whose current content is itself an *earlier*,
already-cached-away block's unstored `EditText` result cannot be resolved from that empty cache, and
the fallback read (which assumes a real stored blob) fails — `lifecycle replay: blob … required for a
state effect is missing`, a different error site than the original defect, same root cause.

**This was caught by testing, not inspection**: the five-generation CLI reproduction
(`crates/prikk-cli/tests/dc65_text_edit_baseline.rs`) failed at generation 3 after the first three
fixes, with exactly this error. Traced to `lifecycle_cache/incremental.rs`'s `try_incremental_step`,
which now treats `LifecycleReplayError::MissingBlobForLifecycleEffect` from `apply_one_block` as a
fifth, principled fallback trigger — retroactively withdrawing eligibility for this commit and
routing to the unmodified full-replay path, which (its `TextCache` spanning the whole lineage) always
succeeds. Full rationale and why this cannot mask genuine corruption:
`rfcs/handoffs/DC-64-baseline-reconstruction-cost/incremental-baseline-cache-design-v1.md` §3a,
updated alongside two erratum notices correcting that document's original (now-disproven) claims that
an empty `TextCache` "changes nothing about correctness" and that "a full replay of the same lineage
would encounter the identical malformed data and fail the same way."

**This is not scope creep into DC-64's performance question** (explicitly a non-goal here) — it is a
correctness fix to code that, before this increment, could never be exercised on this path, because
the very patches that would reach it could never be authored. DC-65 made them authorable; the
incremental cache's own materialization gap had to be closed in the same increment or the fix would
be incomplete for any repository past its first couple of edits.

## Verification

- Five consecutive sealed edits to one file, driven through the compiled binary exactly as a user
  would, succeed and produce the correct final content (`dc65_text_edit_baseline.rs`). Independently
  re-verified by materializing the same sealed history through `checkout --patch-materialize` into a
  fresh worktree and by `prikk verify` reporting zero divergence on both DC-56's and DC-64's caches.
- Four consecutive sealed edits at the store level
  (`text_file_edited_across_four_sealed_commits_succeeds`), crossing DC-64's incremental-cache
  eligibility boundary (generations 3+ are incremental, not full-replay), confirming the fifth
  fallback trigger engages correctly rather than merely happening to work once.
- Four consecutive sealed `ReplaceBinary` edits (`binary_file_replaced_across_four_sealed_commits_succeeds`)
  confirming criterion 4's "unaffected" finding with an explicit regression test, not just the
  read-the-code argument in §1.
- Manually reproduced against the unfixed candidate first (byte-identical error and blob id to the
  architect's independent reproduction on `6064da6`), then against the fixed candidate, for both the
  original defect and the DC-64 interaction.

## Coverage finding (criterion 5)

**What was missing.** Every existing test that touches `EditText` creates a node once and edits it
once (`text_baseline_modified_file_authors_edit_text`, and the CLI suite's various single-edit
scenarios). Nothing in 561 prior store tests, 80 object tests, the crash matrix, the fuzz campaign, or
DC-41's integrity-evidence campaign edited the same text file across two *separate, sealed* commits —
the single most ordinary sequence a version-control system exists to support. The gates are
comprehensive for adversarial/structural cases (malformed input, corrupted state, concurrent access)
and for single-shot authoring correctness, but had no representative of sustained, ordinary use: the
same file, edited repeatedly, over time, with real seals between edits.

**What was added.** Three tests specifically targeting *sequences* of sealed generations rather than
single commits: `text_file_edited_across_four_sealed_commits_succeeds` and
`binary_file_replaced_across_four_sealed_commits_succeeds` (store level, via the new
`seal_active_patch` test helper — the store-level equivalent of `prikk seal`, reusable by future tests
needing multi-generation history without driving the CLI binary), and
`editing_the_same_text_file_across_five_sealed_commits_succeeds` (CLI level, end-to-end through the
compiled binary plus independent checkout verification). None of the three existed before this
increment; all three would have caught the original defect immediately.

**Why this matters beyond this one bug.** The gap was structural, not a missing edge case: single-commit
tests can only ever exercise the *first* time any code path runs against a given node. Any defect that
depends on *history* — a second edit, a second delete-then-recreate, a value computed once and reused
incorrectly on the next pass — is invisible to that shape of test by construction. `seal_active_patch`
exists so the next increment that needs multi-generation coverage does not have to solve this problem
again from scratch.

## Identity (criterion 6)

No existing object's bytes or `ObjectId` move. `EditText`'s wire shape, `write_content_blob`'s two
call sites, and every canonical encoding are unchanged. The fix changes *how* a node's current text is
obtained before constructing a new `EditText` operation — it does not change what that operation
contains, how it is encoded, or what object identities result from sealing it. `patch_algebra`'s
`baseline_text` similarly returns the same bytes it always should have; nothing about what merge
evidence certifies changes.

## What did not change

Commit path semantics (node identity, parentage, canonical ordering), `write_content_blob`'s call
sites, `ReplaceBinary`'s handling (confirmed unaffected, not modified), checkout/materialization
(confirmed already correct, not modified), and DC-64's cache-validity trust argument (checksum,
`from_replay`, reanchor bound, `verify`-based divergence) — the fifth fallback trigger extends *when*
the incremental step defers to full replay, it does not touch any of those four defenses.
