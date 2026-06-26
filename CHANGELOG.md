# Changelog

## 0.1.0-pr003

### Added

- Repository layout creation under `.prikk/`.
- `RepositoryLayout` path model in `prikk-store`.
- `FileObjectStore` for persistent object envelope writes and reads.
- Deterministic envelope file codec for storage round-trips.
- Integrity checks when reading file-backed objects.
- Minimal `prikk init [path]` and `prikk status` commands.
- PR-003 implementation handoff document.

### Changed

- CLI banner updated to `0.1.0-pr003`.
- `prikk-store` is now a concrete early storage crate rather than only a trait scaffold.

### Still deferred

- WAL implementation.
- RefState/ref-log implementation.
- Patch algebra implementation.
- Plugin/audit implementation.
- Remote sync.

## 0.1.0-pr002

### Fixed

- Formatting drift reported by `cargo fmt --check`.
- Strict Clippy `indexing_slicing` failures in `prikk-hash`.
- Missing documentation warnings in `prikk-error` and CLI.

## 0.1.0-pr001

### Added

- Initial workspace scaffold.
- Object identity seed.
- Canonical encoding seed.
- Object envelopes and in-memory object store boundary.
