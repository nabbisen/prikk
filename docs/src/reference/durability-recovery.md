# Durability and Crash Recovery

This page is the authoritative current-state reference for Prikk's local persistence and
crash-recovery model. It describes the current implementation behavior without adding storage,
verification, doctor, or command semantics.

For related concepts, see the [data model](./data-model.md), the
[trust and threat model](./trust-threat-model.md), and the command guides for `verify` and `doctor`
through the [integrity and recovery diagnostics](./integrity-recovery.md) reference.

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- Durability and recovery claims are supported by current unit and integration tests, not by a
  completed crash-matrix or fuzzing campaign.
- Linux is the only platform exercised by the current project gates; macOS, Windows, and other
  filesystem semantics remain unverified release targets.
- `.prikk/` is not a stable repository format and there is no stable migration policy yet.
- Ref pointer files are mutable convenience pointers, not roots of trust.
- `doctor` repairs are opt-in and narrow; they do not synthesize missing objects, signatures, trust
  policy, or key material.
- Stale `active.lock` cleanup after a crash is manual today and belongs to the future
  concurrency/locking reference.

## Commit Persistence Boundary

A successful `commit` appends an exact signed Patch envelope to the active WAL. The WAL append path
rejects non-Patch envelopes and unsigned Patch envelopes, writes a checksummed record, fsyncs the WAL
file, and best-effort syncs the parent directory when the WAL file is first created.

That is the active-session persistence boundary. It does not mean the Patch is sealed into a Block, a
RefState has been published, a ref pointer moved, or the active WAL has been drained. Sealed history is
created later by `seal`.

## WAL Replay and Tail Handling

WAL replay reads valid records from the start of the file. Each complete record carries magic,
version, sequence, body length, checksum, and the encoded signed envelope bytes.

Incomplete trailing bytes are reported separately as trailing partial bytes. They represent the only
current WAL truncation case that `doctor --repair-wal-tail` handles. A complete record with a checksum
mismatch, malformed header, unsupported version, or malformed envelope is an integrity failure and is
not a safe automatic truncation candidate.

## Active Ref Metadata

The active WAL is paired with active ref metadata that records which local branch ref owns the
non-empty WAL. A non-empty active WAL with missing or malformed active ref metadata is an
active-session integrity issue. Seal refuses that state rather than guessing which ref should receive
the WAL records.

An empty active WAL with leftover active ref metadata is local debris. Verification and doctor report
that distinction so empty-WAL cleanup does not get confused with sealed-history corruption.

## Seal Publication Flow

`seal --allow-no-audit` publishes the active WAL through the repository storage layers in a fixed
order:

1. Acquire the active lock.
2. Replay the active WAL and reject trailing partial bytes.
3. Reject an empty active WAL, or clean empty-WAL metadata debris where the command path permits it.
4. Require active ref metadata to match the requested local branch ref.
5. Verify the configured MAINTAINER signer against the repository-local trust policy.
6. Persist the signed Patch envelopes from the WAL into the object store.
7. Create a signed Block envelope.
8. Create a signed RefState envelope.
9. Create and append a signed RefUpdate log record.
10. Promote the ref pointer through the ref store.
11. Drain the active WAL and remove active ref metadata after successful publication.

The implementation is designed so interruption recovery lands on a checkable previous ref state or a
checkable new published state. That statement is bounded by the current evidence: unit/integration
tests, no completed crash-matrix or fuzzing campaign, and Linux-only exercised gates.

If the active WAL's Patch IDs already match the current published tip, seal treats that as an
idempotent retry case and drains the active WAL/ref metadata instead of appending another publication.
If the already-published transition cannot be checked, seal fails closed.

## Ref Pointer and Ref Log Recovery

Ref publication uses a signed RefState object, a signed inline RefUpdate log record, and a mutable ref
pointer file. The pointer is useful for fast lookup, but it is not trusted by itself.

The ref store validates branch ref names, holds a ref-specific lock, checks the expected current
RefState ID before and after log append, writes a candidate pointer file, renames that candidate into
place, and best-effort syncs the parent directory after promotion.

If the `heads/main` pointer is missing, `doctor --repair-main-ref` may reconstruct it only from
already-valid evidence:

- the current pointer is absent;
- the ref log exists and has no trailing partial record;
- decoded RefUpdates form a valid chain;
- the latest RefUpdate points to an existing signed RefState object;
- that RefState points to an existing Block object;
- the ref-specific lock can be acquired.

The repair writes only the missing pointer file. It does not create missing objects, rewrite ref logs,
repair malformed logs, truncate ref-log tails, or synthesize publication policy evidence.

## Doctor Repair Boundary

DC-28 owns the durability and crash-recovery framing for `doctor`. The current recovery actions are:

- `doctor --repair-wal-tail`, which truncates incomplete trailing bytes after verification has accepted
  the preceding WAL prefix; and
- `doctor --repair-main-ref`, which reconstructs a missing `heads/main` pointer from already-valid
  ref-log and RefState evidence.

The [integrity and recovery diagnostics](./integrity-recovery.md) reference owns the full diagnostic
catalog: verification checks, `DoctorIssue` codes, severities, and diagnostic interpretation. This
page intentionally does not duplicate that catalog.

Doctor repair refuses to modify the repository when verification has error-severity issues. It also
does not auto-trust keys, repair signatures, repair checksum mismatches, rebuild missing objects,
recover missing key material, or clear unsafe active sessions.

## Stale Locks and Manual Repair

`active.lock` is acquired with exclusive file creation. If a process dies while holding it, stale lock
cleanup is manual today. DC-28 does not define lock stealing, lock expiry, process ownership checks, or
automatic stale-lock repair. That policy belongs with the future concurrency/locking reference.

## Deferred Work

Still deferred: crash-matrix testing, filesystem fault injection, fuzzing for WAL/ref-log recovery,
macOS and Windows filesystem validation, stale-lock policy, broad active-session recovery, ref-log
repair, missing-object recovery, object quarantine or garbage collection, backup/restore tooling,
stable repository-format migration, and production-readiness claims.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| Commit persistence appends exact signed Patch envelopes to the active WAL, fsyncs the WAL file, and best-effort syncs the parent directory only when the WAL is first created. | [`wal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/wal.rs), [PR-004](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-004-WAL-HANDOFF.md) |
| WAL replay reports incomplete trailing bytes separately from complete-record checksum failures. | [`wal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/wal.rs), [PR-004](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-004-WAL-HANDOFF.md), [PR-006](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-006-VERIFY-HANDOFF.md) |
| WAL-tail repair truncates only incomplete trailing bytes and refuses complete-record integrity failures. | [`wal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/wal.rs), [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [PR-012](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-012-DOCTOR-REPAIR-HANDOFF.md) |
| Non-empty active WALs require valid active-ref ownership metadata; empty-WAL metadata debris is separate local debris. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [DC-15](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md) |
| Seal rejects trailing partial WAL bytes, missing/malformed active ref metadata, and mismatched active ref ownership before publication. | [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [DC-15](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md) |
| Seal persists WAL Patches, creates signed Block and RefState objects, appends signed RefUpdate evidence, publishes the ref, then drains active state after successful publication. | [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [PR-009](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-009-SEAL-SCAFFOLD-HANDOFF.md) |
| Seal verifies the configured MAINTAINER signer against repository-local trust before publication. | [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [`trust.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/trust.rs), [DC-11](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md) |
| Ref publication uses ref-specific locking, compare-and-swap checks, signed RefState/RefUpdate envelopes, candidate pointer write, rename promotion, and parent-directory sync. | [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [`pointer.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/pointer.rs), [`fsutil.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/fsutil.rs), [PR-007](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-007-REF-PUBLICATION-HANDOFF.md) |
| Missing `heads/main` pointer reconstruction is limited to already-valid ref-log, RefState, and Block evidence. | [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [PR-013](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-013-REF-RECOVERY-HANDOFF.md) |
| Doctor began as read-only diagnostics, and current mutating repairs remain opt-in and narrow. | [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [PR-011](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-011-DOCTOR-HANDOFF.md), [PR-012](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-012-DOCTOR-REPAIR-HANDOFF.md), [PR-013](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-013-REF-RECOVERY-HANDOFF.md) |
| Ref pointer files are mutable pointers, not roots of trust. | [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [`pointer.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/pointer.rs), [data model](./data-model.md) |
| Durability/platform claims remain limited by current test evidence and Linux-only exercised gates. | [DC-24 baseline recap](https://github.com/nabbisen/prikk/blob/main/rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md), [DC-24](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-24-DATA-MODEL-TRUST-THREAT-DOCS.md), [DC-28](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-28-DURABILITY-CRASH-RECOVERY-REFERENCE.md) |

## Provenance

This reference consolidates current released records through DC-27 plus the done DC-28 design prepared
for the 0.17.2 release. It
follows the DC-26 documentation-home model: current-state references live in the published mdBook, not
under `rfcs/fdds/`. It is documentation-only and does not change WAL, ref, seal, verification, doctor,
object schema, CLI, trust, or repository behavior.
