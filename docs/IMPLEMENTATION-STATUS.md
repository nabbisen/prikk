# PRIKK Implementation Status

## Current source drop

`0.1.0-pr003`

## Implemented

- Workspace scaffold.
- Object ID formula.
- Canonical encoding seed.
- Object envelope shape.
- Signature metadata shape.
- Payload shape seed.
- In-memory object store.
- File-backed object store.
- Repository layout initialization.
- Minimal CLI `init` and `status` commands.

## Open implementation gates

- Safe scaffolding.
- Object identity and storage foundation increments.

## Still gated / intentionally deferred

- WAL durability.
- RefState/ref-log/CAS.
- Patch algebra and commutation.
- Plugin ABI/runtime.
- Audit publication policy.
- Remote sync.

## Next likely PR

PR-004 should introduce the WAL record format and append/replay tests, but it should not advance refs
or seal publication until the WAL behavior is reviewed independently.
