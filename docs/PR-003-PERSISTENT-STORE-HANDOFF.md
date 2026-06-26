# PRIKK PR-003 Persistent Store Handoff

## Purpose

PR-003 adds the first persistent repository/storage layer without implementing WAL, refs, locking,
or patch algebra. The goal is to validate the object-store boundary before adding transactionality.

## Implementation scope

- Add `.prikk/` layout creation.
- Add object-type directories for patch, block, ref-state, tag, attestation, blob, and ref-update.
- Add hash-prefix fanout for object files.
- Add a file-backed object store that writes and reads `ObjectEnvelope` values.
- Verify object identity on read.
- Add minimal CLI support for repository initialization.

## Non-goals

- Do not implement WAL.
- Do not implement ref updates.
- Do not implement locks.
- Do not implement patch application or commutation.
- Do not implement plugins or audit policy.

## Review focus

1. Confirm object paths are deterministic and type-separated.
2. Confirm object file reads verify computed IDs.
3. Confirm signed envelopes round-trip without re-signing.
4. Confirm CLI behavior is intentionally minimal.
5. Confirm no future FDD semantics are frozen accidentally.

## Expected checks

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
