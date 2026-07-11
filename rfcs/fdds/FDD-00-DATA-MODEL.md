# FDD-00 - Current Data Model Reference

Status: Current-state reference created by accepted DC-24
Scope: Released implementation through 0.16.0 plus accepted DC-24 documentation rules

## Numbering and Scope

`FDD-00` is a consolidation reference. The original FDD scheme split data-model material across
storage, canonical identity, schema, lifecycle, and patch-algebra records. This file collects the
current released data-model facts in one place so public docs can link to a stable reference.

This file does not create FDD-01, FDD-02, FDD-03, or FDD-05. Those references remain unconsolidated or
deferred unless a later RFC creates them. `FDD-04-TRUST-THREAT-MODEL.md` is the companion security and
trust reference.

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- `.prikk/` is Prikk's native repository format and is not Git-compatible storage.
- Ref files are pointers, not roots of trust.
- Maintainer trust is repository-local with the current minimal `required = 1` policy.
- `verify` is not a global trust proof.
- There is no key rotation, revocation, hardware signing, remote trust, sync trust, or stable migration
  policy yet.
- Durability and recovery claims are supported by current unit and integration tests, not by a
  completed crash-matrix or fuzzing campaign.
- Linux is the only platform exercised by the current project gates; cross-platform fsync and path
  semantics remain design targets.

## Object Identity

Prikk objects are typed, versioned envelopes. An object id is SHA-256 over a domain-separated preimage
containing the object type, schema version, payload length, and unsigned canonical payload bytes.
Signatures live outside that identity preimage, so adding or sorting signatures does not change the
object id.

The current object model includes persistent Patch, Block, RefState, and Blob object directories. Tag
and Attestation object types and directories are defined, but current public command surfaces do not
produce Tag or Attestation objects. RefUpdate is an object-envelope type stored inline in ref logs
rather than as a persistent object-store directory. BlockSummaryCache and RecoveryNote are explicitly
not roots of trust.

## Patch and Operation Model

A Patch is the identity-bearing unit of logical change. Its payload contains one or more ordered
operations, sorted parent Patch ids, optional intent, optional preconditions, and an identity-bearing
purpose. `PatchPurpose::Normal` is the default by omission. `PatchPurpose::RollbackDraft` is encoded
explicitly and survives WAL-to-object persistence for rollback classification.

Current production authoring creates node-addressed patches from the worktree. It derives the baseline
from authoritative replay of the published branch tip, or from an empty genesis baseline for an unborn
branch ref. It rejects snapshot-only baselines without node identity for worktree authoring.

## Blocks

A Block is an immutable sealed history unit. Its payload records sorted parent Block ids, Block kind,
Patch ids in canonical Block order, a state Merkle root, and an optional snapshot Blob reference.
Current seal creates Root Blocks for unborn refs and Normal Blocks for refs with an existing published
tip. Merge, Repair, and Import Block kinds are defined in the payload enum, but current public command
surfaces do not publish merge execution or import behavior.

## Refs and Publication

RefState is the content-addressed state for a branch or tag ref. A ref pointer file stores the current
RefState id for convenience and recovery, but the pointer file is not itself the root of trust.
RefUpdate records are signed envelope entries in append-only ref logs and link old and new RefState
ids, target Block id, update sequence, creation timestamp, and maintainer key id.

Publication is guarded by ref-specific locking and compare-and-swap checks. Seal persists WAL Patch
envelopes, creates a signed Block, creates a signed RefState, appends a signed RefUpdate log entry,
promotes the ref pointer, then drains the active WAL and active ref metadata after successful
publication.

## Active WAL and Recovery Boundary

The active WAL stores exact signed Patch envelopes before sealing. WAL append requires a Patch envelope
with at least one signature, writes a checksummed record, and fsyncs the WAL file. WAL replay reads
valid records from the start and reports incomplete trailing bytes separately from checksum failures.

The current active-session model is single-commit-per-active-WAL. Active ref metadata records which
branch ref owns a non-empty active WAL. Missing or malformed active ref metadata on a non-empty WAL is
an integrity issue; stale metadata on an empty WAL is local debris.

Doctor repair is intentionally narrow. It can truncate an incomplete trailing WAL record after the
preceding records verify, and can reconstruct a missing ref pointer from an already-valid ref log and
RefState. It does not synthesize missing objects, repair malformed logs, or prove crash behavior beyond
the current test evidence.

## Replay, Checkout, Verify, and Doctor

Replay and lifecycle semantics live in the internally scoped `prikk-replay` crate, while
`prikk-store` remains the repository integration crate for layout, refs, WAL, active sessions, object
storage, verification, doctor, and worktree integration. `prikk-replay` is not a stable external Rust
API.

Repository verification is read-only. It checks object placement, envelope decoding, object identity,
Block references, ref pointer and log consistency, active WAL checksums, active WAL metadata health,
sealed rollback Patch classification, and publication trust for publication envelopes. Doctor converts
verification results into actionable diagnostics and exposes only the narrow repairs described above.

## Deferred

Still deferred: stable repository-format migration, complete branch management, tags/remotes, sync,
hosted forge trust, audit/plugin execution, production merge execution, persisted proof or witness
objects, general rollback authorization, multi-maintainer publication policy, and full cross-platform
filesystem validation.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| Object ids derive from type, schema version, payload length, and unsigned canonical payload. | `crates/prikk-object/src/id.rs`; `crates/prikk-object/src/envelope.rs`; `rfcs/archive/DC-09-PHASE-4-NODE-MODEL.md` |
| Signatures are outside object identity. | `crates/prikk-object/src/envelope.rs`; `crates/prikk-object/src/signature.rs`; `rfcs/done/DC-10-ROLLBACK-DRAFT-SIGNING.md` |
| Current persistent object directories exclude RefUpdate. | `crates/prikk-store/src/layout.rs`; `crates/prikk-object/src/id.rs` |
| Patch payloads require non-empty contiguous operations and carry identity-bearing purpose. | `crates/prikk-object/src/payload/patch.rs`; `rfcs/done/DC-10-ROLLBACK-DRAFT-SIGNING.md` |
| Worktree authoring derives baselines from authoritative replay or valid genesis. | `crates/prikk-store/src/worktree_patch/node_authoring.rs`; `rfcs/done/DC-13-NONDEFAULT-REF-GENESIS.md`; `rfcs/IMPLEMENTATION-STATUS.md` |
| Blocks contain parent ids, kind, Patch ids, state root, and optional snapshot Blob ref. | `crates/prikk-object/src/payload/block.rs`; `crates/prikk-cli/src/seal.rs` |
| RefState is content-addressed state and ref pointer files are mutable pointers. | `crates/prikk-object/src/payload/refs.rs`; `crates/prikk-store/src/refs.rs`; `rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md` |
| RefUpdate is append-only publication evidence stored inline in ref logs. | `crates/prikk-object/src/payload/refs.rs`; `crates/prikk-store/src/refs.rs`; `crates/prikk-cli/src/seal.rs` |
| Active WAL records exact signed Patch envelopes and detects trailing partial bytes. | `crates/prikk-store/src/wal.rs`; `crates/prikk-store/src/verify.rs`; `rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md` |
| Verification is read-only and bounded to structural, WAL, ref, rollback, and publication-trust checks. | `crates/prikk-store/src/verify.rs`; `crates/prikk-store/src/doctor.rs`; `rfcs/IMPLEMENTATION-STATUS.md` |
| `prikk-replay` is internally scoped and not a stable external API. | `rfcs/done/DC-19-REPLAY-LIFECYCLE-CRATE-BOUNDARY.md`; `rfcs/done/DC-20-REPLAY-BOUNDARY-STABILIZATION.md`; `rfcs/IMPLEMENTATION-STATUS.md` |
| Durability and platform claims remain limited by current test evidence. | `rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md`; `rfcs/accepted/DC-24-DATA-MODEL-TRUST-THREAT-DOCS.md` |

## Provenance

This reference consolidates released records through DC-23 and accepted DC-24. It uses
`rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md` only as a tracked recap of older
non-VCS baseline inputs; current code, released RFCs, and `rfcs/IMPLEMENTATION-STATUS.md` remain the
durable authorities.
