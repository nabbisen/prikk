# Changelog

## 0.1.0-pr002

CI feedback fix for the initial implementation source drop.

- Fixed `cargo fmt --check` formatting drift.
- Removed Clippy `indexing_slicing` failures from `prikk-hash`.
- Added missing documentation for structured error fields.
- Added crate-level documentation for the CLI binary.
- Kept implementation scope unchanged: no WAL, refs, patch algebra, plugins, or sync yet.

## 0.1.0-pr001

Initial implementation source drop.

- Added Rust workspace.
- Added object identity seed.
- Added deterministic canonical encoder seed.
- Added ObjectEnvelope with external signatures.
- Added payload structures for core object types.
- Added in-memory object-store boundary.
