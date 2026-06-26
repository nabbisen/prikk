# PR-006 Verification Handoff

## Scope

PR-006 adds read-only repository verification on top of the PR-005 persistent object store and active WAL layer.

## Implemented

- `verify_repository` in `prikk-store`.
- Object-store scan across persisted object type directories.
- Object file path validation against computed object ID and canonical fanout path.
- Envelope decode and recomputed object ID verification.
- Active WAL replay verification and trailing-partial-byte reporting.
- CLI `prikk verify [path]`.

## Explicitly deferred

- Repair/truncation (`doctor`).
- Ref-state publication and CAS.
- Patch algebra.
- Plugin/audit execution.

## Suggested checks

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
