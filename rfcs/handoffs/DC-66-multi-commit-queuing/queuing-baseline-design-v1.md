# DC-66 Design — Baseline-for-the-Next-Queued-Patch Rule

Companion to `prerequisite-questions-v1.md` (required reading first). States RFC criterion 2's rule
explicitly and traces both authoring and replay conformance, per criterion 2's own wording.

## 1. The rule

**Queuing is a chain.** The effective baseline for queued commit *k+1* is the sealed baseline
(`resolve_baseline_state`, DC-64-accelerated, entirely unmodified) with queued commits `1..k`'s own
operations folded on top, in WAL append order, via the identical `apply_state_effect` fold every other
replay path in this codebase uses — never a parallel reimplementation.

Concretely, `author_inner` (`crates/prikk-store/src/worktree_patch/node_authoring.rs`) now does:

1. `sealed_state = resolve_baseline_state(...)` — **unchanged**, still DC-64-accelerated, still called
   with the same `(baseline_block_id, horizon_id)` derived from the last **published** ref state.
2. If `active_replay.records` is non-empty (a queue already exists), fold those records' operations on
   top of `sealed_state` via the new `lifecycle_cache::replay::apply_queued_patch_envelopes` — producing
   the *effective* baseline `NodeLifecycleState` this commit actually authors against, plus a
   `queue_text_cache` of any text materialized while folding.
3. `baseline_files`/`baseline_symlinks` (existing-node identity, path, blob_id, mode) are built from
   this effective state exactly as before — no other change to that construction.
4. `plan_edit_text`'s current-text resolution (`current_text_for_node`) now checks `queue_text_cache`
   **first**, before its existing two DC-65 fallbacks (direct stored blob, then
   `materialize_edited_text` over sealed lineage). When no queue exists, `queue_text_cache` is empty and
   every lookup misses, so behaviour is byte-identical to before DC-66 (criterion 9).

## 2. Why "against the last sealed state" is unsound, restated precisely

A worktree file created by queued commit 1 is a live node in commit 1's own patch, but that patch is
not yet a durable object (`persist_wal_patches` only writes queued records as objects at `seal` time —
`crates/prikk-cli/src/seal/support.rs:11-24`) and is not reachable from `resolve_worktree_baseline`,
which reads only `RefStore::read_current_ref_state_id`. Without folding, commit 2's `baseline_files`
would not contain that path, so commit 2 would mint a **second** `node_id` for it via a second
`CreateFile` — two live nodes claiming one path within the same eventual block. Folding commit 1's own
`CreateFile` into the effective state before commit 2 runs is what makes
`generator.mint_fresh(&working_state)` (E1, unchanged) correctly see the path as already occupied.

## 3. Why the fold needs its own DC-65-shaped fallback, and why it differs from DC-64's

`apply_queued_patch_envelopes`'s `text_cache` starts empty for the fold, structurally identical to
`apply_one_block`'s per-block cache. A queued `EditText` whose current content descends from a
**sealed** `EditText` (the node existed and was last edited before this queue started) will miss it,
hitting the exact `MissingBlobForLifecycleEffect` DC-65 named.

**DC-64's fix does not transfer.** DC-64's fifth fallback trigger (`try_incremental_step`) reacts to
this miss by abandoning the incremental step and deferring to an unmodified *full replay* — an
alternative, complete path that always exists because the target is always a **sealed** block. The
unsealed portion of a queue has no such alternative: there is no "full replay" of patches that were
never written as objects. So the fold cannot defer elsewhere; it must resolve the miss itself.

**The resolution reuses DC-65's own materializer, not a new one.** On `MissingBlobForLifecycleEffect`
for an `EditText` operation, the fold calls `materialize_edited_text(reader, baseline_block_id,
horizon_id, node_id)` — the identical function `plan_edit_text`/`current_text_for_node` already call —
against the **sealed** lineage the queue is chained from, seeds `text_cache` with the result, and
retries the same operation once. `apply_edit_text` only mutates `state`/`text_cache` after successfully
resolving the current text (confirmed by reading `effect.rs:158-184`: the resolution happens before any
`state.set_text_blob`/`text_cache.insert` call), so a failed first attempt leaves both untouched and the
retry is safe. The retry is gated on the failing operation actually being `EditText` — any other
operation kind hitting `MissingBlobForLifecycleEffect` is a genuine, unrelated integrity failure and
propagates unchanged.

**This case is unreachable for a `Genesis` baseline by construction**, not by a special check: every
node in a queue chained from `Genesis` was created within that same queue, so its current content is
either a real stored `CreateFile` blob (direct read succeeds) or an earlier queued `EditText`'s result
(already in `text_cache` from folding that earlier operation in the same pass). `sealed_lineage` is
`None` in that branch; if the unreachable case were somehow hit, failing closed is correct.

## 4. Cost, and why it does not reintroduce DC-64's eliminated cost

The fold only runs when `active_replay.records` is non-empty. The overwhelmingly common case — an
empty active WAL, one commit, one seal — takes the exact same path as before DC-66: `resolve_baseline_state`
runs alone, DC-64-accelerated, with zero new work. Queuing's own fold cost is bounded by the number of
operations actually queued (typically small — this is a session's worth of not-yet-sealed commits, not
a lineage), and its `materialize_edited_text` fallback triggers only for a text node whose most recent
change before the queue was itself an unstored edit — the same cost DC-65 already accepted at the
authoring layer for ordinary sequential edits. Per the owner decision's Sequencing correction: this
increment inherits no performance benefit and is not scoped as one; this section exists only to confirm
it does not *regress* the already-accelerated N = 1 path, which it does not, since that path is
untouched.

## 5. Rollback-draft stays excluded from queuing (deliberate non-goal)

`rollback_draft.rs:136`'s `!replay.records.is_empty()` guard is **unchanged**. `append_rollback_draft`
computes its inverse against the *published* tip and asserts `inverse.target_block_id ==
planned_tip.target_block_id` before appending — composing a correct inverse against a queue's chained,
not-yet-sealed effective baseline is an unaddressed correctness question (what does "inverse of the
queue so far" even mean when some of it might still be amended by another commit before seal?) that the
RFC's acceptance criteria never ask this increment to answer. `rollback-draft` continues to require an
empty active WAL; a user who wants a rollback draft simply seals first. This is the RFC's own
"traps" §4 guidance against scope creep, applied to the third guard the handoff names.

## 6. What is explicitly not changed

`resolve_baseline_state` and everything under DC-64's incremental cache — untouched, no new call sites.
`apply_one_block`, `apply_patch_ids`, full-lineage replay (`replay_derived_state`,
`replay_lineage_with_materialized_text`) — untouched; DC-66 adds a sibling fold function, not a
modification to any existing one. `materialize_edited_text` — untouched, reused as-is by both the
authoring call site (unchanged) and the new fold. `plan_edit_text`'s canonical diff/splice construction
— unchanged; only its current-text *source* gains one more, checked-first option.
