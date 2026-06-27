# PR-016 Snapshot Path-Safety Handoff

## Scope

PR-016 adds a conservative snapshot-manifest validation boundary for future checkout materialization.
It does not write the worktree and does not replay patches.

## Implemented

- `RepoPath` validates repository-relative paths.
- Path validation rejects absolute paths, `..`, empty components, backslashes, colon characters,
  Windows reserved names, control characters, and non-ASCII paths.
- Snapshot manifests stored in Blob payload bytes can be decoded and validated.
- Snapshot manifests reject duplicate paths and case-insensitive path collisions.
- `prepare_snapshot_checkout_plan()` validates the current block's snapshot blob and returns a
  read-only file-count/byte-count/path summary.
- `prikk checkout --snapshot-plan [path] [--ref REF]` exposes the validation path.

## Deferred

- Real worktree writes.
- Unicode NFC normalization beyond the current conservative ASCII subset.
- Snapshot file materialization.
- Patch replay and algebra.
- Audit plugins and remote sync.

## Acceptance / QA

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Manual smoke:

```sh
cargo run -p prikk -- init ./sample-repo
(cd ./sample-repo && ../target/debug/prikk checkout --plan-only)
```
