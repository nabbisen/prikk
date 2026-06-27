# PR-026 Handoff: Supported Patch Inverse Planning

## Summary

PR-026 adds a read-only inverse planning boundary for the operation subset currently supported by
patch replay. It does not publish rollback refs and does not mutate the repository.

## Added

- `prikk_store::prepare_patch_inverse_plan()`
- `PatchInversePlan`
- `PatchInverseOperationSummary`
- `PatchInverseOperationKind`
- CLI command: `prikk inverse-plan [path] [--ref REF]`
- mdBook page: `docs/src/patch-inverse.md`

## Supported Inverse Shapes

- `CreateFile` becomes inverse `DeleteFile`.
- `DeleteFile` becomes inverse `CreateFile`.
- `ReplaceBinary` swaps old/new Blob IDs.
- full-file `EditText` becomes an inverse full-file `EditText` whose replacement is the prior text.

## Safety Boundary

The inverse Patch payload is unsigned and not persisted. The reported Patch ID is only a
planning hint. Any future mutating rollback command must have a separate design covering ref
publication, authorization, conflict handling, and recovery.

## Deferred

- Mutating rollback commands.
- Rollback ref publication policy.
- Authorization and audit policy for inverse application.
- Arbitrary-span inverse handling.
- Commutation, confluence, and conflict witnesses.
- Audit plugins and sync.

## Validation Requested

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
