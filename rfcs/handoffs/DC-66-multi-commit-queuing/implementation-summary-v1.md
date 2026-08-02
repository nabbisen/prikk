# DC-66 Implementation Summary

Companion to `prerequisite-questions-v1.md` (§4's four answers) and `queuing-baseline-design-v1.md`
(the baseline-for-the-next-queued-patch rule, criterion 2). This document covers what was built at
each remaining site, the coverage statement (criterion 10), and identity.

## What was built, per site

**Three append guards.** `crates/prikk-store/src/active.rs` (`ActiveSession::append_patch`) and
`crates/prikk-store/src/worktree_patch/node_authoring.rs` (`author_inner`): a non-empty active WAL no
longer returns an error. Both still call `require_active_ref_for_non_empty_wal` — unchanged, since its
ref-ownership check was already record-count-agnostic (prerequisite-questions §2) — and then continue
rather than reject. `crates/prikk-store/src/rollback_draft.rs`'s guard is **unchanged by design**:
composing a correct inverse against a queue's chained, not-yet-sealed baseline is an unaddressed
correctness question outside this increment's acceptance criteria (`queuing-baseline-design-v1.md`
§5); rollback-draft still requires an empty queue.

**The chain fold.** `crates/prikk-store/src/lifecycle_cache/replay.rs`'s new
`apply_queued_patch_envelopes` folds queued (unsealed) WAL records onto the sealed baseline via the
same `apply_state_effect` fold every other replay path uses, operating on in-memory envelopes rather
than object-store reads (queued patches are not yet durable objects — `persist_wal_patches` only
writes them at `seal`). On `MissingBlobForLifecycleEffect` for an `EditText` — a queued edit whose
current content descends from a *sealed* `EditText` — it materializes the sealed text once via the
existing `materialize_edited_text` (DC-65) and retries. `node_authoring.rs::author_inner` calls it
whenever `active_replay.records` is non-empty, producing the effective baseline state and a
`queue_text_cache` that `current_text_for_node` checks before its existing two DC-65 fallbacks.

**Crash recovery reporting.** `WalRepair` (`crates/prikk-store/src/wal.rs`) gained
`preserved_patch_ids: Vec<ObjectId>` — `truncate_trailing_partial`'s mechanism already preserved every
complete record regardless of count (a `while` loop over the whole file, never assuming exactly one),
but only reported a count. `doctor`'s CLI output now prints each preserved patch id.

**`verify` queue health.** Per-patch signature validity (`classify_signature_envelope` over every
`replay.records` entry) and single-ref ownership (`ActiveWalMetadataStatus`) were already
count-agnostic. Ordering was not checked at all — `crates/prikk-store/src/verify.rs` gained
`check_active_wal_ordering`, reporting any record whose sequence does not strictly increase over its
predecessor (reachable only by direct file tampering, since `Wal::append_patch` always assigns
`previous.seq + 1`, but a queue of N gives "ordering" a meaning worth verifying explicitly).

**`status`.** `crates/prikk-cli/src/main.rs::run_status` now prints `queued patches: N targeting
<ref>` (or `<missing metadata>` / `<malformed metadata>` if the active-WAL ownership metadata itself
is unhealthy), reading `read_active_ref_metadata` rather than reusing `heads/main`'s published
`RefState` (the last *sealed* state, not what a live queue is targeting).

**Seal batching.** `crates/prikk-cli/src/seal/support.rs`'s `persist_wal_patches`/
`collect_wal_patch_ids` already looped over `&[WalRecord]` — no code change. `BlockPayload.patch_ids`
was always `Vec<ObjectId>`. DC-64's `apply_one_block`/`apply_patch_ids` already looped over
`block.patch_ids` with one shared `TextCache` spanning the whole block — also no code change. Both
were confirmed, not modified, by prerequisite-questions §4 and re-confirmed by
`text_file_edited_across_a_queue_then_sealed_together_succeeds` sealing two queued edits together.

## Coverage finding (criterion 10)

**What was missing.** Every code path downstream of the active WAL — `seal`'s batching loop, DC-64's
incremental cache, DC-65's text materialization, `verify`'s per-record checks, `doctor`'s repair, and
`status`'s reporting — was written generically (`Vec<ObjectId>`, `for record in &replay.records`,
`while offset < bytes.len()`) but had **never actually run with N > 1**, because the guard being
removed in this increment made that state unreachable. This is exactly the coverage shape DC-65 named:
"single-instance tests only ever exercise the first time a path runs against a given thing" — restated
here at the level of an entire queue rather than a single node.

**What was added.**
- `queued_commits_mint_distinct_node_ids_and_see_each_others_creates` (store): two queued commits
  against a sealed baseline, proving the chain fold (not luck) prevents a duplicate `node_id`.
- `text_file_edited_across_a_queue_then_sealed_together_succeeds` (store, load-bearing): edits a
  sealed-then-queued-then-queued-again text file, forcing the new fold's `materialize_edited_text`
  fallback, the queue-cache-first path in `current_text_for_node`, a two-patch sealed block, and a
  fifth edit that forces DC-64's incremental step to engage against that two-patch block and defer to
  full replay — the first test in this codebase to exercise DC-64/DC-65 at N > 1.
- `crash_during_seal_with_a_queued_pair_preserves_both_and_completes_on_retry` (store): simulates the
  crash point between "patch objects durably written" and "ref publication completes" with a queue of
  two, proving neither is lost and the retry (DC-38's existing machinery) still completes correctly.
- `wal_truncate_preserves_all_complete_records_in_a_torn_queue_and_reports_their_ids` (store): a torn
  queue (two complete records plus a partial third), proving `truncate_trailing_partial` preserves both
  and reports their patch ids, not just a count.
- `verify_repository_reports_active_wal_ordering_violation` (store): a hand-tampered WAL with a
  duplicate sequence, proving the new ordering check actually fires.
- `genesis_second_commit_before_seal_queues` (store, converted from
  `genesis_second_commit_before_seal_fails_closed`): the RFC's own opening scenario, chained straight
  from `Genesis`.
- `active_session_append_queues_distinct_patch_onto_non_empty_wal` (store, converted from
  `active_session_append_rejects_non_empty_wal`): the lower-level `ActiveSession` API (no production
  caller, but a public surface) updated for consistency.
- `two_commits_with_no_seal_between_queue_and_seal_together` (CLI,
  `dc66_multi_commit_queuing.rs`): the full user-facing workflow — `commit; commit; status; status;
  seal; status; verify; checkout --patch-materialize` — driven through the compiled binary.

**Why this matters beyond this one increment.** A guard that has protected a code path from ever
running a certain way is not evidence that path is correct that way — it is evidence that path has
never been checked. Every "already loops," "already `Vec`," "already generic" claim in
prerequisite-questions §4 was true and also previously unproven; this increment is the first proof for
all of them at once.

## Identity

No existing object's bytes or `ObjectId` move, and no wire format changed. `BlockPayload.patch_ids`,
`PatchPayload`, `EditText`'s shape, and every canonical encoding are unchanged — DC-66 is purely a
capability change in *when* `commit` is allowed to run and *what baseline* it computes, never in what
gets written. `WalRepair` and `RepositoryVerification` gained new fields (`preserved_patch_ids`,
`active_wal_ordering_issues`); both are additive, non-breaking to any existing caller that constructs
or matches them positionally-never (both are `#[non_exhaustive]`-shaped in practice — no code outside
`verify.rs`/`wal.rs`/their own tests constructs either struct literal).

## What did not change

Node identity minting order (E1, canonical path order, unchanged), canonical operation ordering, DC-64's
persisted cache format and trust argument (checksum, `from_replay`, reanchor bound — the chain fold is
entirely additive on top, never modifying `resolve_baseline_state` itself), DC-65's `materialize_edited_text`
(reused as-is), `seal`'s block/ref construction, and the one-for-one workflow (existing tests pass
unmodified except the two guard tests directly encoding the old reject behavior, both converted with
inline justification, per criterion 9).
