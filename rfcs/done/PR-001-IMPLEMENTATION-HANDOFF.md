# PR-001 Implementation Handoff

## Purpose

Start Prikk implementation with the smallest durable foundation: object identity, canonical encoding boundaries, object envelope shape, and crate scaffolding.

## Scope included

- Workspace scaffold.
- `prikk-error` shared error taxonomy.
- `prikk-hash` SHA-256 seed implementation.
- `prikk-object` object identity and canonical payload structures.
- `prikk-store` in-memory test boundary only.
- `prikk` CLI placeholder.
- CI workflow seed.

## Scope deliberately excluded

- Persistent object store.
- WAL and replay.
- Ref locking and publication.
- Patch application/commutation.
- Ed25519 signing implementation.
- Plugin execution.

## Review checklist

- Check object ID formula against FDD-03 v0.3.
- Check that signatures do not affect object ID.
- Check canonical encoder does not use protobuf bytes.
- Check sorted-vs-ordered repeated fields are not accidentally collapsed.
- Check first-party crates forbid unsafe code.
- Run `cargo fmt`, `cargo clippy`, and `cargo test` in a Rust-enabled environment.

## Follow-up PRs

1. Complete canonical field tables and golden vectors.
2. Implement Ed25519 verification/signing behind `prikk-crypto`.
3. Implement object-store file layout and fsync rules.
4. Implement WAL record format and replay.
5. Implement RefState object and ref pointer publication.
