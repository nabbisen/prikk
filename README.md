# Prikk

This package is the first implementation drop for PRIKK after FDD approval was confirmed.
It intentionally starts with the smallest production-quality core slice:

- Rust workspace scaffold.
- `#![forbid(unsafe_code)]` on first-party crates.
- newtype wrappers for identity-bearing values.
- deterministic canonical encoder scaffolding.
- object envelope and object ID computation.
- pure Rust SHA-256 implementation used for ObjectId computation.
- preliminary payload shapes for Patch, Block, RefState, RefUpdate, Tag, Attestation, and Blob.
- CLI placeholder that prints implementation status.
- storage crate placeholder for later WAL/object-store work.

## Current scope

This is **PR-001**, not a full M1 implementation. It should be reviewed as the schema/identity starting point.
The WAL, lock protocol, ref publication, patch algebra, plugin execution, and audit publication policy are intentionally not implemented in this drop.

## Intended validation

In a Rust-enabled environment, run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The current execution container used to prepare this artifact did not have `cargo`/`rustc` installed, so the code was not compiled here. The package includes tests and fixtures for the development environment to run immediately.

## Important invariants implemented here

- Object ID formula uses a single domain string: `PRIKK-OBJECT-ID-v1`.
- Object IDs are computed from unsigned canonical payload bytes.
- Signatures are stored outside the identity preimage.
- Canonical encoding does not use protobuf bytes.
- Repeated fields that are semantic-order-bearing are explicitly ordered; associative lists are sorted by validator helpers.

## Next PRs

Recommended sequence:

1. PR-002: complete FDD-03 canonical field coverage and golden vectors.
2. PR-003: object-store write/read/verify using the ObjectEnvelope from this package.
3. PR-004: WAL record framing and replay.
4. PR-005: ref-state object + ref pointer layout.
