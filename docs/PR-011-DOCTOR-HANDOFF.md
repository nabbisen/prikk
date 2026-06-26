# PR-011 Doctor Diagnostics Handoff

## Scope

PR-011 adds a read-only `doctor` diagnostic layer over repository verification. The goal is to make repository health results actionable without introducing repair, truncation, or mutation semantics.

## Implementation Summary

- Added `prikk-store::doctor` module.
- Added `DoctorReport`, `DoctorIssue`, and `DoctorSeverity`.
- Added `doctor_repository(&RepositoryLayout)` as a non-mutating diagnostic entry point.
- Added CLI command `prikk doctor [path]`.
- Doctor reports verification success, verification failure, and trailing partial WAL bytes with stable issue codes.
- Added tests for healthy repositories, trailing partial WAL warnings, and verification errors.

## Non-Goals

- No destructive repair.
- No WAL truncation.
- No ref reconstruction.
- No worktree diff capture.
- No patch algebra.
- No audit/plugin execution.

## QA Checklist

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Manual smoke:

```sh
cargo run -p prikk -- init /tmp/prikk-doctor-smoke
cargo run -p prikk -- doctor /tmp/prikk-doctor-smoke
```

Expected: doctor exits successfully and reports no health errors.
