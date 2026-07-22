# Data Model

This page is the authoritative current-state reference for Prikk's data model. It describes the
released implementation through 0.16.0 and is grounded in the code, released RFCs, and implementation
status records listed in the anchor table at the foot of the page.

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- `.prikk/` is Prikk's native repository format and is not Git-compatible storage.
- Ref files are mutable pointers for convenience and recovery, not roots of trust.
- Durability and recovery claims are supported by current unit and integration tests, not by a
  completed crash-matrix or fuzzing campaign.
- Linux is the only platform exercised by the current project gates; cross-platform fsync and path
  semantics remain design targets.
- Stable repository-format migration, complete branch management, tags/remotes, sync, hosted forge
  trust, plugin execution, and production merge execution remain deferred.

Trust, signature, and threat-boundary caveats live in the
[trust and threat model](./trust-threat-model.md). The local persistence and crash-recovery boundary
lives in the [durability and crash recovery](./durability-recovery.md) reference. The physical
`.prikk/` layout and authority-vs-pointer/cache boundary lives in the
[repository layout and authority](./repository-layout.md) reference. Local lock and ref
compare-and-swap behavior lives in the [concurrency and locking](./concurrency-locking.md) reference.

## Object Identity

Prikk objects are typed, versioned envelopes. An object id is SHA-256 over a domain-separated preimage
containing the object type, schema version, payload length, and unsigned canonical payload bytes.
Signatures live outside that identity preimage, so adding or sorting signatures does not change the
object id.

New envelope serialization and repository writes require a strict signature sequence. Ed25519
signatures must be 64 bytes, duplicate signature tuples are rejected, and signatures are ordered by
key-id bytes, signer-role code, algorithm code, then signature bytes. Advisory signature timestamps
do not affect that order. Format-1 verification preserves older structurally readable bytes and
reports malformed shape, duplicate, or non-canonical ordering as warnings instead of rewriting them.

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
ids, target Block id, update sequence, a schema-1 no-clock sentinel, and maintainer key id. The
`created_at` field is exactly zero for current writes and is not a trusted creation or event timestamp.

Publication is guarded by ref-specific locking and compare-and-swap checks. The
[concurrency and locking](./concurrency-locking.md) reference owns the detailed lock/CAS behavior.
Seal persists WAL Patch envelopes, creates a signed Block and RefState, promotes the authoritative ref
pointer as the publication commit point, appends exactly one signed RefUpdate log entry, confirms
pointer/log agreement, then drains the active WAL and active ref metadata.

## Active WAL and Recovery Boundary

The active WAL stores exact signed Patch envelopes before sealing. WAL append requires a Patch
envelope with at least one signature, writes a checksummed record, and fsyncs the WAL file. WAL replay
reads valid records from the start and reports incomplete trailing bytes separately from checksum
failures.

The detailed persistence, seal-publication, and recovery framing lives in the
[durability and crash recovery](./durability-recovery.md) reference.

The current active-session model is single-commit-per-active-WAL. Active ref metadata records which
branch ref owns a non-empty active WAL. Missing or malformed active ref metadata on a non-empty WAL is
an integrity issue; stale metadata on an empty WAL is local debris.

Doctor repair is intentionally narrow. It can truncate an incomplete trailing active-WAL record after
the preceding records verify. It does not reconstruct missing ref pointers, sign or append RefUpdates,
synthesize missing objects, repair malformed logs, or prove crash behavior beyond current test
evidence. Exact interrupted ref publication completion belongs to signer-backed `seal` retry.

## Replay, Checkout, Verify, and Doctor

Replay and lifecycle semantics live in the internally scoped `prikk-replay` crate, while `prikk-store`
remains the repository integration crate for layout, refs, WAL, active sessions, object storage,
verification, doctor, and worktree integration. `prikk-replay` is not a stable external Rust API.

Repository verification is read-only. It checks object placement, envelope decoding, object identity,
Block references, ref pointer and log consistency, active WAL checksums, active WAL metadata health,
sealed rollback Patch classification, and publication trust for publication envelopes. Doctor converts
verification results into actionable diagnostics and exposes only the narrow repairs described above.
The diagnostic catalog lives in the
[integrity and recovery diagnostics](./integrity-recovery.md) reference.

## Deferred

Still deferred: stable repository-format migration, complete branch management, tags/remotes, sync,
hosted forge trust, audit/plugin execution, production merge execution, persisted proof or witness
objects, general rollback authorization, multi-maintainer publication policy, and full cross-platform
filesystem validation.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| Object ids derive from type, schema version, payload length, and unsigned canonical payload. | [`id.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-object/src/id.rs), [`envelope.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-object/src/envelope.rs), [DC-09](https://github.com/nabbisen/prikk/blob/main/rfcs/archive/DC-09-PHASE-4-NODE-MODEL.md) |
| Signatures are outside object identity; strict new envelopes enforce Ed25519 shape, tuple uniqueness, and canonical order. | [`envelope.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-object/src/envelope.rs), [`signature.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-object/src/signature.rs), [DC-39](https://github.com/nabbisen/prikk/blob/main/rfcs/accepted/DC-39-SIGNATURE-ENVELOPE-AUTHORITY.md) |
| Current persistent object directories exclude RefUpdate. | [`layout.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/layout.rs), [`id.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-object/src/id.rs) |
| Patch payloads require non-empty contiguous operations and carry identity-bearing purpose. | [`patch.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-object/src/payload/patch.rs), [DC-10](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-10-ROLLBACK-DRAFT-SIGNING.md) |
| Worktree authoring derives baselines from authoritative replay or valid genesis. | [`node_authoring.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/worktree_patch/node_authoring.rs), [DC-13](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-13-NONDEFAULT-REF-GENESIS.md), [implementation status](https://github.com/nabbisen/prikk/blob/main/rfcs/IMPLEMENTATION-STATUS.md) |
| Blocks contain parent ids, kind, Patch ids, state root, and optional snapshot Blob ref. | [`block.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-object/src/payload/block.rs), [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs) |
| RefState is content-addressed state and ref pointer files are mutable pointers. | [`refs.rs` payload](https://github.com/nabbisen/prikk/blob/main/crates/prikk-object/src/payload/refs.rs), [`refs.rs` store](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [DC-11](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md) |
| RefUpdate is append-only publication evidence stored inline in ref logs; schema-1 writes use zero as a no-clock sentinel. | [`refs.rs` payload](https://github.com/nabbisen/prikk/blob/main/crates/prikk-object/src/payload/refs.rs), [`refs.rs` store](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [DC-39](https://github.com/nabbisen/prikk/blob/main/rfcs/accepted/DC-39-SIGNATURE-ENVELOPE-AUTHORITY.md) |
| Active WAL records exact signed Patch envelopes and detects trailing partial bytes. | [`wal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/wal.rs), [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [DC-15](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md) |
| Verification is read-only and bounded to structural, WAL, ref, rollback, and publication-trust checks. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [implementation status](https://github.com/nabbisen/prikk/blob/main/rfcs/IMPLEMENTATION-STATUS.md) |
| `prikk-replay` is internally scoped and not a stable external API. | [DC-19](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-19-REPLAY-LIFECYCLE-CRATE-BOUNDARY.md), [DC-20](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-20-REPLAY-BOUNDARY-STABILIZATION.md), [implementation status](https://github.com/nabbisen/prikk/blob/main/rfcs/IMPLEMENTATION-STATUS.md) |
| Durability and platform claims remain limited by current test evidence. | [DC-24 baseline recap](https://github.com/nabbisen/prikk/blob/main/rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md), [DC-24](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-24-DATA-MODEL-TRUST-THREAT-DOCS.md) |

## Provenance

This reference consolidates released records through DC-23 and DC-24. It uses
[`baseline-recap.md`](https://github.com/nabbisen/prikk/blob/main/rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md)
only as a tracked recap of older non-VCS baseline inputs; current code, released RFCs, and
[`IMPLEMENTATION-STATUS.md`](https://github.com/nabbisen/prikk/blob/main/rfcs/IMPLEMENTATION-STATUS.md)
remain the durable authorities. DC-26 moved this current-state reference from `rfcs/fdds/` into the
published book without changing code, schema, trust, or CLI behavior.
