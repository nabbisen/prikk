# PR-007 Ref Publication Handoff

## Summary

PR-007 adds the first durable ref-state/ref-log storage primitives. A `RefState` is persisted as a
content-addressed object before publication, while the ref file is only a flat hashed pointer under
`refs/by-id/<sha256(ref_name)>.ref`. Ref updates are appended as signed inline log records.

## Scope

Implemented:

- `RefLock` for ref-specific exclusive write protection.
- `RefStore::publish` for signed RefState publication with CAS.
- Durable ref pointer candidate write and atomic pointer promotion.
- Inline signed RefUpdate log append/replay with checksums.
- Ref pointer and ref-log verification in `verify_repository`.
- CLI status/verify output for ref-state and ref-log counters.

Not implemented:

- Full seal transaction.
- Policy evaluation or attestation requirements.
- Ref log recovery/repair.
- Branch listing indexes.
- Patch algebra or checkout.

## Design Notes

The PR follows the integrated FDD direction: RefState is a content-addressed object, while the ref
file is an authoritative current pointer. The log is append-only evidence and recovery material, not
the primary source of truth while the ref pointer exists.

## QA Checklist

Run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Manual smoke test:

```sh
cargo run -p prikk -- init ./sample-repo
cd ./sample-repo
cargo run -p prikk -- status
cargo run -p prikk -- verify .
```

Expected current state: `heads/main` is usually unpublished until a later seal/branch command writes
a real RefState through the public workflow.
