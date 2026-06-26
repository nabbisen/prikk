# PR-004 WAL Handoff

## Scope

PR-004 adds the first active-session WAL implementation after the PR-003 persistent object store.
It follows the approved design direction: commit-time WAL records store the full signed
`ObjectEnvelope`, so recovery can reconstruct exact patch objects without re-signing.

## Implemented

- `Wal::append_patch` appends signed patch envelopes and fsyncs the WAL file.
- `Wal::replay` validates record magic, version, length, checksum, and envelope bytes.
- Incomplete trailing records are reported as trailing partial bytes.
- Non-final checksum mismatch is treated as integrity failure.
- First WAL creation fsyncs the parent directory.
- `ActiveLock` uses exclusive file creation for the default active session.

## Intentional Limits

- No stale-lock stealing yet.
- No WAL truncation command yet.
- No seal transaction yet.
- No ref update or ref log mutation yet.
- No real Ed25519 signing yet; signatures are structurally validated only.

## Review Checklist

- Confirm the WAL record format matches FDD-02 v0.3 direction.
- Confirm signed patch envelope requirement is appropriate for this increment.
- Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
