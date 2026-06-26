# PR-015 Checkout Plan Handoff

## Scope

PR-015 adds a read-only checkout planning boundary. It does not write the worktree and does not
apply patch algebra. The goal is to validate the current publication target before later worktree
materialization work begins.

## Implemented

- `prikk_store::prepare_checkout_plan()`.
- `CheckoutPlan` and `CheckoutMaterialization`.
- Current RefState loading and target Block decoding.
- Validation of parent Block, Patch, and optional snapshot Blob references.
- CLI `prikk checkout --plan-only [path] [--ref REF]`.
- Tests for unpublished refs, valid block/patch references, and missing patch detection.

## Acceptance Commands

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Deferred

- Worktree writes.
- Snapshot Blob materialization.
- Patch apply/inverse/commutation.
- Path-level checkout filters.
- Conflict-state checkout.
