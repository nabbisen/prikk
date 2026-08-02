# DC-57 Implementation Summary

Companion to `implementation-handoff-v2.md` and `rfcs/accepted/DC-57-ACTIVE-PATCH-THRESHOLDS.md`'s
acceptance criteria.

## 1. The definition (criterion 1)

**"Active patches" is the active WAL's record count** — the same count DC-66 already reports via
`status`, `Wal::replay().records.len()`. One computation site: a shared comparison,
`worktree_patch::active_patch_limit_exceeded(current_count, active_patch_limit) -> bool`
(`current_count >= active_patch_limit`), called by both authoring paths that can grow the queue:
`node_authoring.rs::author_inner` (the production path) and `active.rs::ActiveSession::append_patch`
(a lower-level, currently uncalled-in-production API DC-66 already updated for its own guard, kept
consistent here). `rollback_draft.rs::append_rollback_draft` needs no check: DC-66 left its guard
requiring an empty active WAL, so it can only ever move the count from 0 to 1 — never near either
threshold.

## 2. Thresholds and enforcement (criteria 2, 3)

- **`WorktreePatchCommitOptions`** gained `active_patch_limit: usize`, defaulting to
  `DEFAULT_ACTIVE_PATCH_LIMIT` (1000) in both `file_level()`/`prefer_text_edits()`, overridable via
  `.with_active_patch_limit(limit)`. No existing call site needed to change — the default keeps every
  pre-DC-57 test's behavior identical.
- **`author_inner`** checks the limit immediately after reading `active_replay` (the WAL replay) and
  before the empty/non-empty branch that follows — before any ref-metadata write, baseline resolution,
  blob write, or WAL append. A blocked commit's error names `seal` as the remedy.
- **`ActiveSession::append_patch`** gained an `active_patch_limit: usize` parameter with the identical
  check in the identical position.
- Neither check is reachable via any path that has already started writing — both fire on data read
  before the function's first mutation.

## 3. Configuration (criteria 2, 6)

Read once in `prikk-cli/src/main.rs`, at the CLI boundary — following the precedent already set by
`prikk-store` never reading environment variables itself (`PRIKK_AUTHOR_KEY_ID`/`PRIKK_AUTHOR_SEED` are
likewise read only in `main.rs` and threaded down as constructed values). `ActivePatchThresholds::from_env()`
parses `PRIKK_ACTIVE_PATCH_WARN` (default 800) and `PRIKK_ACTIVE_PATCH_LIMIT` (default 1000,
`prikk_store::DEFAULT_ACTIVE_PATCH_LIMIT`) together, failing closed — never silently keeping the
default — on: a value present but non-numeric, `warn > limit`, or either equal to zero. No config
file, no parser dependency, no persistence: the setting is per-invocation, exactly as the RFC requires.

## 4. `status` extension, not a second surface (criterion 7)

`run_status` already prints `queued patches: N targeting <ref>` (DC-66). This increment adds, only
when the queue is non-empty: a hard-limit warning when `N >= limit`, else a recommend-sealing warning
when `N >= warn`. No new command, no new output block — the RFC's own instruction ("`status` already
reports queue health... extend it, don't invent a surface") is met literally.

## 5. `seal` availability at and above the bound (criterion 4)

`seal` (`crates/prikk-cli/src/seal.rs::seal_active_no_audit`) takes no `WorktreePatchCommitOptions` and
never consults an active-patch limit — it only drains whatever the queue already holds. Confirmed, not
just argued: `seal_remains_available_at_and_above_the_hard_bound` queues to a scaled limit, then seals
successfully and verifies the resulting block matches what was queued.

## 6. Tests (criterion 5)

- `worktree_patch::threshold_tests::boundary_values_match_the_rfc` — the RFC's literal 799/800/999/
  1000/1001 tested directly against the one shared comparison (pure arithmetic, no repository).
- `active_patch_hard_block_fires_before_any_write_and_leaves_no_partial_state` — a scaled limit (2)
  proves the *wiring*: two ordinary commits succeed, a third is refused, the WAL is byte-identical to
  before the refusal, and no new object was written for the refused attempt's content.
- `seal_remains_available_at_and_above_the_hard_bound` — seal succeeds and drains the queue once the
  limit is reached.
- `active_session_append_patch_enforces_its_own_limit` — the second authoring path, same proof.
- CLI (`dc57_active_patch_thresholds.rs`, driving the compiled binary):
  `scaled_thresholds_warn_then_hard_block_then_seal_recovers` (the full lifecycle: warn hint, hard-limit
  hint, refusal naming `seal`, seal recovers), `defaults_apply_when_unset`, and
  `malformed_thresholds_fail_closed_rather_than_defaulting` (non-numeric, warn-above-limit, and
  zero-for-either, each refused, none leaving a queued patch beyond what was already there).

Why a scaled limit rather than literally queuing 799+ patches: the comparison itself is the entire
runtime logic (`>=`), already proven correct at the literal RFC values by the pure unit test above;
what the integration tests need to additionally prove is that the check is *wired into the real path*
at the right point, which a scaled limit demonstrates identically to a limit of 1000, in milliseconds
instead of the cost of authoring, signing, and replaying 800+ real patches — the same "install the
boundary directly rather than perform the operation N times" technique DC-64 used for its 64-step
reanchor bound.

## 7. Identity and what did not change

No existing object's bytes or `ObjectId` move, no wire format changed — this is purely a policy check
on the commit path, not a change to what gets written. `commit_worktree_changes_signed`'s public
signature is unchanged (the new field lives inside `WorktreePatchCommitOptions`, already a parameter).
`ActiveSession::append_patch`'s signature changed (gained `active_patch_limit: usize`) — it has no
production caller, so this is source-breaking only for its own seven test call sites (all updated) and
any external consumer of the crate's public API, which does not exist inside this repository.
`seal`'s block/ref construction, DC-66's chain fold, and DC-64/DC-65's materialization are all
untouched — the check happens strictly before any of them run.

## 8. Test counts before/after

`prikk-store` 568 → 572 (+4: the boundary unit test plus three integration tests above); `prikk-cli`
gained a new 3-test file (`dc57_active_patch_thresholds.rs`); `prikk-object`/`prikk-replay`/
`prikk-hash`/`prikk-crypto`/`prikk-release-policy` unchanged at 80/44/14/5/59; locked package count
unchanged at 180 (no new dependency — the RFC's own non-goal).
