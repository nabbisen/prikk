# Implementation Status — PR-002

Status: CI feedback fix for the initial source drop.

## Implemented

- Workspace layout.
- First-party unsafe policy.
- Error taxonomy seed.
- SHA-256 implementation seed, revised to satisfy strict Clippy indexing policy.
- Object ID newtype and display/parse helpers.
- ObjectEnvelope with signatures external to identity.
- Canonical encoder helpers.
- Payload shape seed for core object types.
- Basic tests for SHA-256 and object ID determinism.

## Fixed from PR-001 feedback

- Formatting drift reported by `cargo fmt --check`.
- `clippy::indexing_slicing` violations in `prikk-hash`.
- Missing documentation on `ObjectTypeMismatch` fields.
- Missing crate-level documentation for the CLI binary.

## Not implemented yet

- Ed25519 signing/verification.
- Trust store.
- WAL and fsync transaction logic.
- Object-store persistence.
- Ref locking and ref publication.
- Patch algebra and commutation.
- Worktree materialization.
- Plugin host.

## Review focus

Reviewers should focus on:

1. whether the revised SHA-256 implementation still passes the known vectors;
2. whether strict Clippy now remains clean under `-D warnings`;
3. whether the canonical encoder API is safe to extend without accidental protobuf identity bytes;
4. whether the ObjectEnvelope separation is clear enough;
5. whether crate boundaries match the PRIKK architecture.
