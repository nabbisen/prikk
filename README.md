# Prikk

PRIKK is a next-generation VCS prototype built around content-addressed objects,
block-oriented patch theory, and design-first implementation gates.

This source drop is **PR-003**. It extends the PR-002 object-identity scaffold with
the first persistent repository/storage layer.

## Implemented in PR-003

- Rust workspace scaffold retained from PR-001/PR-002.
- Deterministic object identity and canonical payload types.
- Object envelopes with signatures outside identity.
- Repository layout creation via `RepositoryLayout::init`.
- File-backed object store with object-type directories and hash-prefix fanout.
- Simple envelope file codec for round-tripping signed object envelopes.
- Object read integrity checks: stored envelope ID must match the requested ID.
- Minimal CLI commands:
  - `prikk init [path]`
  - `prikk status`
  - `prikk --version`

## Still intentionally not implemented

- WAL append/replay.
- RefState publication and ref logs.
- Locking protocol.
- Patch apply/inverse/commutation.
- Plugin host and audit publication.
- Remote sync.

Those remain separate PRs so each durability/security boundary can be reviewed independently.

## Expected local checks

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

This environment still does not include `cargo`/`rustc`, so these checks were not executed here.
Please run them on the development machine and return logs for the next fix cycle.
