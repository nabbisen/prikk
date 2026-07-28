# DC-41 Stage 1 - Crash Matrix Coverage Evidence

**Scope.** DC-41 stage 1 only (crash-matrix audit). Stages 2-4 are separate candidates. The platform
matrix is descoped from DC-41 entirely (see the RFC's Follow-up section).
**Snapshot baseline.** `crates/prikk-store/src/fsutil/anchored/failpoints.rs` `Point` enum, 24 variants at
implementation time. This count is a snapshot, not a target to preserve — re-enumerate from the enum, not
from this table, if it drifts.
**Layer contract.** Every variant requires a primitive-level (`P`) test asserting specific durable
post-failure state. Variants on the identity-or-publication-critical path additionally require a
repository-level (`R`) test asserting repository post-state (pointer value, log record count, verify
issue code, retry outcome) — a mis-sequenced failure there produces the split-brain class of defect DC-38
exists to prevent. Applying that rule, `PromotionRename` and `PromotionSourceSync` require `R` in addition
to `P`; all other variants are satisfied by `P` alone, though most already have supplementary `R`
coverage from unrelated caller tests.
**Supersedes** `.git-exclude/reviewed/prikk-dc41-stage1-crash-matrix-developer-handoff-v1.md`, the
reviewer-produced starting inventory. That document is explicitly a review-time snapshot, not authority,
and one of its findings did not hold under verification: it stated `PromotionRename`/`PromotionSourceSync`
have "no repository-level coverage." An existing test,
`refs/tests/publication_recovery/failpoints.rs::pointer_promotion_failures_retry_to_one_log_record`,
already exercised both at the repository level via `RefStore::publish`. What was actually missing was
assertion *strength* at that layer (no pointer-value or verify-issue-code assertion, only `is_err()` plus
a final log-record count) — so this stage strengthens that coverage rather than adding it from scratch.
The exact expected pointer value and verify issue code per variant were confirmed empirically (a
throwaway probe against the real code, not derived from reading the verify-logic match arms by hand,
which would have produced the wrong code for `PromotionSourceSync` — see Notes). Root cause per the
implementation review's own disclosure: the inventory's search excluded the enum-definition file by
**basename** (`grep -v "failpoints.rs:"`), which also silently dropped the repository-level test file
`refs/tests/publication_recovery/failpoints.rs` (same basename, different path) — understating the
"Layer" column for this pair and, less consequentially, for `ImmutableFileSync`, `ImmutableInstallSync`,
`MutableFileSync`, `MutableRename`, `AppendWrite`, and `Truncate`, whose repository-level rows below are
independently verified against this table rather than that inventory. A future reader should treat this
table, not the superseded inventory, as authoritative.

## Evidence Table

| # | `Point` variant | Layer | Test(s) | Asserted post-state | Disposition |
|---|---|---|---|---|---|
| 1 | `CreatedDirectoryParentSync` | P | `fsutil::tests::directory::failed_directory_parent_sync_retains_created_component` | created component retained as a directory; retry succeeds | already-met |
| 2 | `ObservedDirectoryParentSync` | P | `fsutil::tests::directory::observed_component_parent_sync_failure_is_retryable` | child path does not exist; retry succeeds | already-met |
| 3 | `DirectoryCreate` | P | `fsutil::tests::directory::directory_create_failure_has_no_side_effect_and_is_retryable` | child path does not exist; retry succeeds | already-met |
| 4 | `MutableFileSync` | P+R | `fsutil::tests::failed_mutable_file_sync_keeps_only_non_authoritative_temp` (P); `trust::tests::trust_file_sync_and_rename_failures_pin_effective_state_and_retry` (R) | P: target absent, exactly one temp debris file. R: prior effective key/policy retained; retry installs the new state | already-met |
| 5 | `MutableRename` | P+R | `fsutil::tests::failed_mutable_rename_keeps_previous_authoritative_state` (P); `trust::tests::trust_file_sync_and_rename_failures_pin_effective_state_and_retry` (R, same loop as #4) | P: previous authoritative bytes retained. R: prior effective policy retained; retry installs new state | already-met |
| 6 | `MutableParentSync` | P+R | `fsutil::tests::failed_mutable_parent_sync_retains_replaced_final_name` (P); `refs::tests::publication_recovery::candidate_failure_warns_and_retry_publishes_once` (R) | P: replaced final name retained. R: verify issue `PRIKK-VERIFY-REF-CANDIDATE-DEBRIS`, dependent commit/trust-add operations blocked, retry publishes exactly once | already-met (exceeds bar) |
| 7 | `PromotionDestinationSync` | P+R | `fsutil::tests::promotion_destination_sync_failure_retains_destination_state` (P); `refs::tests::publication_recovery::pointer_sync_failure_is_blocking_and_repeated_retry_appends_once` (R) | P: destination holds committed bytes, source empty (rename already completed; this syncs the destination directory). R: pointer already `Some(state_id)`, log 0 records, verify `PRIKK-VERIFY-REF-DIVERGENCE` (blocking), retry idempotent to exactly 1 log record | already-met (exceeds bar) |
| 8 | `PromotionRename` | P+R | `fsutil::tests::promotion_rename_failure_retains_source_only` (P); `refs::tests::publication_recovery::failpoints::pointer_rename_failure_leaves_unmoved_pointer_with_candidate_debris` (R, **strengthened**) | P: source retains candidate bytes, destination absent (rename itself never happened). R (new): pointer `None` (unmoved), verify issue `PRIKK-VERIFY-REF-CANDIDATE-DEBRIS` and **non-blocking** (`issue.blocking == false`, confirmed empirically); retry promotes to exactly 1 log record, pointer becomes `Some(state_id)`, issues empty | **strengthened** |
| 9 | `PromotionSourceSync` | P+R | `fsutil::tests::promotion_source_sync_failure_reports_committed_destination` (P); `refs::tests::publication_recovery::failpoints::pointer_source_sync_failure_leaves_committed_pointer_ahead_of_log` (R, **strengthened**) | P: source gone, destination holds committed bytes (rename completed; only the post-rename source-directory sync failed). R (new): pointer already `Some(state_id)`, log 0 records, verify issue `PRIKK-VERIFY-REF-DIVERGENCE` and **blocking** (confirmed empirically — same code as #7's `PromotionDestinationSync`, since both are post-rename directory-durability syncs), retry to exactly 1 log record, no blocking issues after | **strengthened** |
| 10 | `RequiredDirectorySync` | P+R | `fsutil::caller_tests::sync_matrix::{wal,active,ref,lock}_*` (multiple callers); `refs::tests::ref_log_file_and_first_directory_sync_failures_retry_without_duplication`; `wal::tests::first_wal_directory_sync_failure_retains_replayable_record`; `lock::tests::failed_lock_directory_sync_retains_stale_lock` | exact log/WAL record counts before and after retry; stale lock file retained until explicit cleanup | already-met |
| 11 | `RequiredFileSync` | P+R | `refs::tests::publication_recovery::complete_record_sync_failure_retries_without_duplicate`; `wal::tests::wal_file_sync_failure_retains_replayable_record`; `lock::tests::failed_lock_file_sync_retains_stale_lock` | exact record counts, no duplicate on retry; stale lock file retained | already-met |
| 12 | `RequiredOpen` | P | `fsutil::tests::directory::required_open_failure_has_no_side_effect_and_is_retryable` | candidate file does not exist; retry succeeds. (Caller-propagation tests in `fsutil::caller_tests::{caller_tests,validation_matrix}` exist for WAL/active/object/trust/ref/worktree callers but only assert `is_err()`/`is_ok()` without a durable-state check — supplementary propagation evidence, not the primary bar-meeting test, since the primitive test already proves no side effect) | already-met |
| 13 | `AppendWrite` | P+R | `fsutil::tests::failed_append_write_is_retryable` (P); `refs::tests::publication_recovery::pointer_lead_with_partial_tail_is_truncated_then_completed` (R); `wal::tests::existing_wal_append_write_failure_is_retryable` | P: log content unchanged on failure, exact content after retry. R: verify `PRIKK-VERIFY-REF-DIVERGENCE`, retry, `trailing_partial_bytes == 0`; WAL record counts exact | already-met (exceeds bar) |
| 14 | `Truncate` | P+R | `fsutil::tests::failed_truncate_retains_previous_state_and_is_retryable`; `wal::tests::wal_truncate_failure_retains_partial_tail_and_retry_repairs_it`; `active::tests::active_publication_cleanup_failures_preserve_retryable_states` | exact byte content retained on failure; `trailing_partial_bytes` exact values before/after; byte-exact WAL/metadata comparison in the active-cleanup case | already-met (exceeds bar) |
| 15 | `Unlink` | P+R | `fsutil::tests::failed_unlink_retains_file_and_cleanup_sync_reports_removed_state`; `active::tests::active_publication_cleanup_failures_preserve_retryable_states` | file retained on failure; queue WAL emptied per the disposition table in the active-cleanup test | already-met |
| 16 | `CleanupDirectorySync` | P+R | `fsutil::tests::failed_unlink_retains_file_and_cleanup_sync_reports_removed_state`; `active::tests::active_publication_cleanup_failures_preserve_retryable_states`; `patch_checkout::tests::patch_deletion_retry_resyncs_observed_absent_parent` | file absence after sync failure; `deleted_files`/`already_absent_deleted_files` counts exact across retry | already-met (exceeds bar) |
| 17 | `ImmutableCleanupSync` | R | `object_store::tests::immutable::exact_existing_bytes_are_synced_and_accepted`; `::immutable_failpoints_retain_required_artifacts_and_retry` (table-driven); `races::fresh_process_retry_resyncs_installed_final_without_cleaning_old_temp` | object id equality after retry; table-driven case: installed=true, temp files empty; cross-process retained-temp count exact | already-met |
| 18 | `ImmutableFileSync` | R | `object_store::tests::immutable::immutable_failpoints_retain_required_artifacts_and_retry` (table-driven); `::crash_left_temp_is_ignored_by_reads_and_warned_without_cleanup` | table-driven: installed=false, exactly 1 temp. Second test: `verify_repository().checked_objects == 0`, `object_temp_paths` matches exactly, doctor issue `PRIKK-DOCTOR-OBJECT-TEMP-DEBRIS` (warning) | already-met (exceeds bar) |
| 19 | `ImmutableInstall` | R | `object_store::tests::immutable::immutable_failpoints_retain_required_artifacts_and_retry` (table-driven) | installed=false, exactly 1 temp | already-met |
| 20 | `ImmutableInstallUnsupported` | R | same (table-driven) | error message contains "unsupported by filesystem or policy"; installed=false, exactly 1 temp | already-met |
| 21 | `ImmutableInstallNoSys` | R | same (table-driven) | same shape as #20 | already-met |
| 22 | `ImmutableInstallPermission` | R | same (table-driven) | same shape as #20 | already-met |
| 23 | `ImmutableInstallSync` | R | same (table-driven); `races::fresh_process_retry_resyncs_installed_final_without_cleaning_old_temp`; `fsutil::caller_tests::sync_matrix::immutable_object_install_sync_failure_retains_and_classifies` | table-driven: installed=true, exactly 1 temp. Cross-process: retained temp count exact after two induced failures. Caller test: final object path is a file after retry | already-met (exceeds bar) |
| 24 | `ImmutableTempUnlink` | R | `object_store::tests::immutable::immutable_failpoints_retain_required_artifacts_and_retry` (table-driven) | installed=true, exactly 1 temp | already-met |

## Summary

- **24/24 variants covered** at the time of this audit (24 at design time, matching the RFC's snapshot).
- **22 already-met** the bar with no change.
- **2 strengthened** (`PromotionRename`, `PromotionSourceSync`) — repository-level assertions added for
  pointer value and verify issue code; the underlying coverage already existed, only assertion strength
  was added.
- **0 added from scratch**, **0 unexercisable**.
- No variant was silently omitted.

## Notes

- The five `ImmutableInstall*`/`ImmutableTempUnlink` variants are passed as **loop elements** in a
  table-driven test (`object_store::tests::immutable::immutable_failpoints_retain_required_artifacts_and_retry`,
  loop at lines ~167-175, per-variant expected outcomes in the `match` at ~192-217), not as literal
  `fail_once_for_test(TestFailPoint::X)` call-site arguments. A search keyed on literal call sites misses
  them; this table was built by enumerating the `Point` enum and searching for each variant name as a
  match arm or array element, not just as a direct call argument.
- The `PromotionSourceSync` verify issue code was **not** derived by reading `refs/verify.rs`'s match
  arms by hand — that reading suggested `PRIKK-VERIFY-REF-POINTER-LEADS-LOG`, which is wrong. The actual
  code path emits `PRIKK-VERIFY-REF-DIVERGENCE` (confirmed by a throwaway `eprintln!` probe against the
  real repository state, then removed before writing the permanent assertion). Manual trace-through of
  branch logic is not a substitute for running the code when the exact string a test should assert is at
  stake.
- `RequiredOpen`'s caller-propagation tests (`fsutil::caller_tests::caller_tests`,
  `fsutil::caller_tests::validation_matrix`) assert only `is_err()` then `is_ok()` on retry, without a
  durable-state check in between. They are not listed as the bar-meeting evidence for variant 12 — the
  primitive test at `fsutil::tests::directory` carries that — but they remain valid supplementary evidence
  that the failpoint propagates correctly through every storage caller (WAL, active, object, trust, ref,
  worktree). Not strengthened, since the bar is already met by the primitive test and the RFC's stage-1
  scope is per-*variant* assertion strength, not per-*call-site* uniformity.

## Diff Summary

One file changed: `crates/prikk-store/src/refs/tests/publication_recovery/failpoints.rs`. The single loop
test `pointer_promotion_failures_retry_to_one_log_record` (covering both `PromotionRename` and
`PromotionSourceSync` with only `is_err()` + final log-count assertions) was replaced with two separate
named tests — `pointer_rename_failure_leaves_unmoved_pointer_with_candidate_debris` and
`pointer_source_sync_failure_leaves_committed_pointer_ahead_of_log` — because the two variants have
different expected pointer/verify-issue outcomes and no longer share one assertion body. Net test count:
+1 (one test replaced by two). No other test was modified, added, or removed. No production code,
dependency, or CI file changed.

## Gate Evidence

- `prikk-store --lib` test count: 530 before, 531 after (net +1, accounted for above; no silent loss).
- `Cargo.lock`, `Cargo.toml`, package manifests, both command inventories, the oracle manifest, and
  `release-signers.toml`: unchanged.
- Full gate output recorded in the implementation-review request (fmt, clippy, workspace test, MSRV
  1.85 test, `git diff --check`, release-policy `check`/`boundary-check`/`reference-check`).
