# PR-005 CI Fix Handoff

## Purpose

PR-005 responds to the CI logs from PR-004. It is intentionally narrow and does not add new
feature scope.

## Fixed Items

- Formatting drift reported by `cargo fmt --check`.
- Dead `ByteCursor::remaining` method reported by strict Clippy.
- CLI `map_or_else` return-type mismatch in `prikk init [path]`.

## Deferred Items

- RefState publication.
- Ref logs and CAS.
- Seal transaction.
- Patch algebra.
- Plugin and audit execution.

## Verification Commands

Run these from the repository root:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
