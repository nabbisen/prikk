# PR-027 Handoff: Non-Mutating Rollback Preview

## Summary

PR-027 adds a read-only rollback preview boundary for the currently supported patch-operation
subset. It combines PR-026 inverse planning with PR-020/PR-024 supported replay validation, then
reports the file-level difference between the current replayed target and the latest snapshot
baseline.

This remains intentionally non-mutating. It does not write inverse Patch objects, append to WAL,
publish rollback refs, or touch worktree files.

## Public API

New `prikk-store` exports:

- `prepare_rollback_preview(layout, ref_name)`
- `RollbackPreviewPlan`
- `RollbackPreviewChange`
- `RollbackPreviewChangeKind`

New CLI command:

```sh
prikk rollback-preview [path] [--ref REF]
```

## Behavior

The preview:

1. derives the unsigned inverse Patch payload using the supported inverse planner;
2. replays the supported single-parent target chain;
3. compares the current replayed state with the latest snapshot baseline;
4. reports deterministic file-level changes:
   - `would-create`
   - `would-delete`
   - `would-replace`

The preview target is the latest snapshot baseline inside the supported replay window. If future
rollback semantics need block-by-block rollback, that should be designed separately before adding a
mutating command.

## Supported Operations

- `CreateFile`
- `DeleteFile`
- `ReplaceBinary`
- full-file `EditText` with `anchor_id = "full-file"`

## Safety Boundaries

PR-027 does not:

- publish refs;
- create rollback branches;
- write inverse Patch objects;
- append active WAL records;
- modify or delete worktree files;
- authorize rollback;
- implement commutation, confluence, or conflict witnesses.

## Validation

Added tests cover:

- file-level rollback previews after create/delete/replace patches;
- full-file text edit rollback previews.

Please run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
