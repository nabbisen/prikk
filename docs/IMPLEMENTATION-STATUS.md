# Implementation Status — PR-001

Status: initial source drop.

## Implemented

- Workspace layout.
- First-party unsafe policy.
- Error taxonomy seed.
- SHA-256 implementation seed.
- Object ID newtype and display/parse helpers.
- ObjectEnvelope with signatures external to identity.
- Canonical encoder helpers.
- Payload shape seed for core object types.
- Basic tests for SHA-256 and object ID determinism.

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

1. whether the canonical encoder API is safe to extend without accidental protobuf identity bytes;
2. whether the ObjectEnvelope separation is clear enough;
3. whether the newtype boundaries are strict enough;
4. whether crate boundaries match the PRIKK architecture.
