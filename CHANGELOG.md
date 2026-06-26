# Changelog

## 0.1.0 PR-008

Narrow empty-commit scaffold that appends a signed patch envelope to the active WAL under `active.lock`.

- Added `ActiveSession` as the locked default active-session append boundary.
- Added CLI `prikk commit --allow-empty -m <message>` as a narrow commit-path scaffold.
- The commit scaffold writes a signed patch envelope to the active WAL and reports the WAL sequence.
- Updated `status` and documentation to reflect PR-008.
- Kept real diff capture, seal, patch algebra, plugin/audit, and sync deferred.

## 0.1.0 PR-007

- Added `RefStore` for initial RefState publication primitives.
- Added flat `refs/by-id/<sha256(ref_name)>.ref` pointer layout helpers.
- Added ref-specific lock scaffold via `RefLock`.
- Added durable ref pointer candidate write and atomic promotion.
- Added inline signed RefUpdate log append/replay with checksums.
- Extended repository verification to check ref pointers and ref-log records.
- Updated CLI `status` and `verify` output with ref-state/ref-log information.
- Added PR-007 ref publication handoff notes.

## 0.1.0 PR-006

- Added read-only repository verification in `prikk-store`.
- Added object-store scan across persisted object type directories.
- Added verification that object file paths match computed object IDs and canonical fanout paths.
- Added verification that object envelope types match their object directories.
- Added active WAL replay verification summary.
- Added CLI command `prikk verify [path]`.
- Added PR-006 verification handoff notes.

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
