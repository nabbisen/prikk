# Changelog

## 0.1.0 PR-005

- Fixed `cargo fmt --check` drift reported against PR-004.
- Removed unused `ByteCursor::remaining` so strict Clippy dead-code checks pass.
- Fixed `prikk init [path]` argument handling so CLI tests compile.
- Updated implementation status and handoff notes for the CI feedback round.

## 0.1.0 PR-004

- Aligned workspace metadata with project Rust instructions: Rust 2024, Apache-2.0, author nabbisen.
- Split large `prikk-store` and `prikk-object` payload files by logical boundaries.
- Added `LICENSE`, `NOTICE`, and mdBook-compatible `docs/src` seed.
- Fixed PR-003 storage transcription defects before extending storage work.
- Added active-session file lock scaffold.
- Added file-backed WAL append/replay for signed patch envelopes.
- Added WAL tests for signed patch round-trip and unsigned-patch rejection.
- Updated CLI status output to report active WAL replay state.

## 0.1.0 PR-003

- Added `.prikk/` repository layout creation.
- Added file-backed object store.
- Added persistent object envelope file codec.
- Added minimal `prikk init` and `prikk status` commands.

## 0.1.0 PR-002

- Fixed formatting and strict Clippy warnings from PR-001 logs.
- Added missing documentation in `prikk-error` and CLI crate.

## 0.1.0 PR-001

- Added initial Rust workspace scaffold.
- Added object ID formula seed.
- Added deterministic canonical encoding seed.
- Added object envelope and in-memory object store.
