# DC-15 FDD-02 Update - Active-Session Integrity Diagnostics

Status: Revised for v0.8.0 design re-review after architect review v1
Related RFC: `../../proposed/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md`
Target FDD: FDD-02 Storage Transaction Model

## Purpose

DC-15 makes active-WAL ref metadata an explicit verification and doctor diagnostic surface. v0.7.0
already uses the metadata to prevent cross-ref seal publication; DC-15 requires repository health checks
to report when that local active-session state is missing or malformed.

## Required FDD-02 Body Updates

### Active-WAL Metadata Status

Verification must classify active-session metadata after replaying the active WAL:

- empty WAL and missing metadata: healthy;
- empty WAL and valid, malformed, or mismatched metadata: local active-session debris;
- non-empty WAL and valid metadata: healthy active-session state;
- non-empty WAL and missing metadata: active-session integrity issue;
- non-empty WAL and malformed metadata: active-session integrity issue;
- non-empty WAL and metadata that fails the local branch-ref validator: active-session integrity issue.

Active-session metadata issues are not sealed-history object corruption. They are local mutable-session
state that blocks safe commit/seal interpretation.

Empty-WAL metadata debris must be visible in `verify` as warning/local-debris state and in `doctor` as an
actionable diagnostic. It must not fail sealed-history verification.

### Doctor Diagnostics

Doctor must surface non-empty-WAL metadata problems as actionable diagnostics. DC-15 does not authorize
automatic repair of non-empty active sessions. Repairing or discarding active WAL contents remains a
separate recovery policy.

Empty-WAL metadata debris may be reported as cleanup-eligible only when active WAL replay has zero
records and zero trailing partial bytes.

### Seal Retry Cleanup

Idempotent seal retry cleanup remains local cleanup, not a fresh publication. It must not require a new
maintainer trust check after the transition is already published. Before draining WAL/ref metadata, seal
must prove that the requested ref already advanced to the expected published state represented by the
active WAL patch IDs. If that state cannot be proven, seal must fail closed and leave the active session
intact.

### Rollback Draft Freshness

Rollback draft append must prove the draft was derived from the same ref tip that is current at append
time:

1. record the target ref state/block identity used for inverse planning;
2. derive inverse and preview without holding the active-session lock;
3. acquire the active-session lock before final WAL emptiness validation and append;
4. re-read the target ref state/block identity while the active-session lock is held;
5. fail closed if the identity changed;
6. write active-WAL ref metadata and append the rollback draft under existing active-session durability
   rules.

The re-read under active lock must not acquire a ref-specific lock or otherwise invert seal's lock order.
Moving the whole inverse derivation under the active lock is not the DC-15 implementation shape unless a
later lock-ordering review explicitly approves it.

This does not add rollback authorization or seal-time rollback approval.

## Required Tests

- `verify` reports non-empty active WAL with missing ref metadata.
- `verify` reports non-empty active WAL with malformed or invalid ref metadata.
- `doctor` reports actionable diagnostics for those states.
- `verify` and `doctor` report empty WAL plus metadata debris as warning/local-debris state, not
  sealed-history corruption.
- Idempotent seal retry cleanup drains only after proving the expected transition is already published,
  without requiring a fresh maintainer trust check.
- Rollback draft append fails when the target ref advances between inverse planning and append.
- Rollback draft append still succeeds when the target ref identity is unchanged.
- Rollback draft freshness re-read does not invert the seal lock order.
