# PR-013 Ref Recovery Handoff

## Scope

PR-013 adds a conservative ref-pointer recovery path on top of the PR-007 ref primitives and
PR-011/PR-012 doctor workflow.

Implemented:

- `RefStore::recoverable_missing_ref("heads/main")` analysis.
- `RefStore::reconstruct_missing_ref_from_log("heads/main")` guarded repair.
- `DoctorRepairOptions::reconstruct_main_ref()`.
- CLI command `prikk doctor [path] --repair-main-ref`.
- Doctor warning when `heads/main` pointer is missing but the ref log and RefState object are valid.
- Tests for direct RefStore reconstruction and doctor-driven reconstruction.

## Safety Rules

The repair is intentionally narrow. It only writes the missing ref pointer file when all of the
following are true:

1. the current pointer for `heads/main` is missing;
2. the ref log exists and has no trailing partial record;
3. every decoded RefUpdate forms a valid chain;
4. the latest RefUpdate points to an existing signed RefState object;
5. the RefState points to an existing Block object;
6. the ref-specific lock can be acquired.

The repair does not create missing objects, repair malformed logs, repair checksum mismatches,
rewrite ref logs, or synthesize publication policy evidence.

## Deferred Work

- generic ref-name recovery UI;
- ref-log truncation or repair;
- object quarantine and GC;
- missing-object recovery;
- real worktree diff capture;
- patch algebra;
- audit plugins and sync.

## Validation Commands

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
