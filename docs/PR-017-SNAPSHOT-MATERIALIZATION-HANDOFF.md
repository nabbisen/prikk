# PR-017 Snapshot Materialization Handoff

## Scope

PR-017 adds a conservative, explicit snapshot materialization path for snapshot-backed blocks.

## Implementation Notes

- Adds `prikk_store::materialize_snapshot_checkout`.
- Adds `SnapshotMaterializationReport`.
- Adds CLI mode `prikk checkout --snapshot-materialize`.
- Writes only regular file entries from a validated `SnapshotManifest`.
- Refuses conflicting existing files, symlinked parents, symlink targets, and non-file targets.
- Rejects snapshot paths under `.prikk/`.

## Deferred

- Patch replay and patch algebra.
- Unicode NFC path normalization beyond conservative ASCII.
- Deleting extra worktree files.
- Snapshot directories, symlinks, executable bits, and binary asset policies.

## QA

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected focused coverage:

- snapshot materialization writes new files;
- materialization is idempotent for identical bytes;
- conflicting existing files are refused;
- `.prikk/` snapshot paths are rejected.
