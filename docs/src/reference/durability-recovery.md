# Durability and Crash Recovery

This page is the authoritative current-state reference for Prikk's local persistence and
crash-recovery model. It describes the current implementation behavior without adding storage,
verification, doctor, or command semantics.

For related concepts, see the [repository layout and authority](./repository-layout.md) reference, the
[data model](./data-model.md), the [trust and threat model](./trust-threat-model.md), and the command
guides for `verify` and `doctor` through the
[integrity and recovery diagnostics](./integrity-recovery.md) reference.
Release-transaction durability, artifact identity, and evidence limits are documented separately in
[release, versioning, and compatibility](./release-compatibility.md).

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- Durability and recovery claims are supported by current unit and integration tests, not by a
  completed crash-matrix or fuzzing campaign.
- Repository mutation currently requires Linux anchored relative no-follow operations, strict regular
  file and directory sync, atomic rename, and the required install primitives. macOS, Windows, and
  filesystems without those proved capabilities remain read-only/diagnostic targets — see
  [platform support](./platform-support.md) for exactly which commands that covers and how it is
  CI-verified.
- `.prikk/` is not a stable repository format and there is no stable migration policy yet.
- Ref pointer files are mutable convenience pointers, not roots of trust.
- `doctor` repairs are opt-in and narrow; they do not synthesize missing objects, signatures, trust
  policy, or key material.
- Stale `active.lock` cleanup after a crash is manual today; the current lock/CAS boundary is covered
  by the [concurrency and locking](./concurrency-locking.md) reference.

## Commit Persistence Boundary

A successful `commit` appends an exact signed Patch envelope to the active WAL. The WAL append path
rejects non-Patch envelopes and unsigned Patch envelopes, writes a checksummed record, required-syncs
the WAL file, and required-syncs the parent directory after every append. Any required
file or directory sync failure returns an operation failure while retaining written state for replay.

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
9. Construct the deterministic signed RefUpdate and durably write a pointer candidate.
10. Promote and required-sync the authoritative ref pointer as the publication commit point.
11. Append and required-sync exactly one signed RefUpdate log record.
12. Confirm pointer/log agreement, then drain the active WAL and remove active ref metadata.

The implementation is designed so interruption recovery lands on a checkable previous ref state or a
checkable new published state. That statement is bounded by the current evidence: unit/integration
tests, no completed crash-matrix or fuzzing campaign, and Linux-only exercised gates.

If the active WAL's Patch IDs already match the current published tip, seal reconstructs the expected
no-clock RefUpdate and finishes any exact one-record pointer lead before cleanup. An existing complete
matching record is not duplicated. If the already-published transition cannot be checked exactly,
seal fails closed.

## Required Filesystem Boundaries

Authoritative directories are traversed through anchored no-follow handles on the supported Linux
mutation path. Missing directories are created one component at a time, and each new name is
established by syncing its parent before descent. Retry also re-syncs the parent of an observed
component instead of treating presence as proof of earlier durability.

Reads, metadata checks, and directory listings that authorize a mutation use the same retained root
as the mutation. Replacing the visible worktree or `.prikk` path therefore cannot redirect a
check-then-mutate workflow to a different tree. Append retries classify an exact retained complete
record without duplicating it and re-sync the file and parent; required removal re-syncs its retained
parent even when the final entry is already absent.

Mutable metadata publication uses a unique same-directory exclusive temp, complete file sync, atomic
replace rename, and required parent sync. An error after rename leaves the final name in place and
returns failure for verification or retry; it does not blindly roll back visible state.

Immutable object publication uses a separate no-clobber operation. It syncs a unique same-shard temp,
installs the final name without replacement, syncs the shard, removes only its invocation-owned temp,
and syncs the shard again. If another publisher wins, success requires same-handle validation and
exact persisted-byte equality; malformed, wrong-identity, wrong-type, or byte-different winners fail
without replacement. Crash-left object temps are warning-only debris and are never object authority.

Worktree writes and removals use separately named strict operations. Their errors propagate and may
leave partial worktree effects, but the worktree does not become repository authority. Lock removal
from a guard destructor remains explicitly best-effort because destruction cannot return an error.

## Ref Pointer and Ref Log Recovery

Ref publication uses a signed RefState object, a signed inline RefUpdate log record, and a mutable ref
pointer file. The pointer is useful for fast lookup, but it is not trusted by itself.

The ref store validates branch ref names, holds a ref-specific lock, rechecks the expected current
RefState ID, writes and syncs a candidate pointer, and promotes it before appending committed log
evidence. It required-syncs `refs/by-id/` to establish the pointer commit point and then required-syncs
`refs/tmp/` to establish candidate removal. Only after pointer promotion does it append and sync the
already-constructed signed RefUpdate.

Verification jointly classifies pointer and log state. A candidate left before promotion is warning
debris but blocks unrelated mutation. Signer-backed retry may replace the complete candidate and
removes only uniquely named candidate-write temps belonging to that ref under its lock. Cleanup
requires the generated `.tmp.<decimal-pid>.<32-lowercase-hex>` suffix; malformed same-prefix or
unrelated names remain visible and mutation-blocking. A pointer
leading the log by exactly one expected transition is an interrupted publication and makes `verify`
fail. Signer-backed `seal` retry may append the exact deterministic RefUpdate after revalidating
retained WAL and trust. If the final log frame is structurally incomplete, that same path may truncate
and sync only the incomplete suffix before the append. Fully framed checksum-invalid or malformed
records are never truncation-safe.

For released format-1 repositories, one exact already-signed log-ahead transition may be completed by
signer-backed seal without another append when retained active state proves the transition. Other
ahead-log states fail closed. A missing format-1 pointer with log history is diagnosed but is not
reconstructed by doctor in 0.18.0; preserve the repository and restore from backup or retain it for
later migration/recovery tooling.

Pointer/log agreement with the matching active WAL and metadata still retained is incomplete cleanup,
not a healthy repository state. Verification returns non-zero and unrelated mutation remains blocked
until signer-backed seal revalidates the transition, appends nothing, and removes active state.

## Doctor Repair Boundary

The current doctor mutation is `doctor --repair-wal-tail`, which acquires the active lock and truncates
incomplete trailing active-WAL bytes after an under-lock publication guard and verification have
accepted the preceding WAL prefix. Doctor diagnoses ref-publication
states but does not sign, append, promote, or reconstruct ref authority.

The [integrity and recovery diagnostics](./integrity-recovery.md) reference owns the full diagnostic
catalog: verification checks, `DoctorIssue` codes, severities, and diagnostic interpretation. This
page intentionally does not duplicate that catalog.

Doctor repair refuses to modify the repository when verification has error-severity issues. It also
does not auto-trust keys, repair signatures, repair checksum mismatches, rebuild missing objects,
recover missing key material, or clear unsafe active sessions.

## Stale Locks and Manual Repair

`active.lock` is acquired with exclusive file creation. If a process dies while holding it, stale lock
cleanup is manual today. DC-28 does not define lock stealing, lock expiry, process ownership checks, or
automatic stale-lock repair. The current lock and compare-and-swap behavior is covered by the
[concurrency and locking](./concurrency-locking.md) reference.

## Deferred Work

Still deferred: the broad crash-matrix campaign, fuzzing for WAL/ref-log recovery,
macOS and Windows filesystem validation, stale-lock policy, broad active-session recovery, ref-log
repair, missing-object recovery, object quarantine or garbage collection, backup/restore tooling,
stable repository-format migration, and production-readiness claims.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| Commit persistence appends exact signed Patch envelopes to the active WAL, required-syncs the WAL file, and required-syncs the parent directory after every append. | [`wal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/wal.rs), [DC-37](https://github.com/nabbisen/prikk/blob/main/rfcs/accepted/DC-37-REQUIRED-FILESYSTEM-DURABILITY.md) |
| WAL replay reports incomplete trailing bytes separately from complete-record checksum failures. | [`wal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/wal.rs), [PR-004](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-004-WAL-HANDOFF.md), [PR-006](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-006-VERIFY-HANDOFF.md) |
| WAL-tail repair truncates only incomplete trailing bytes and refuses complete-record integrity failures. | [`wal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/wal.rs), [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [PR-012](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-012-DOCTOR-REPAIR-HANDOFF.md) |
| Non-empty active WALs require valid active-ref ownership metadata; empty-WAL metadata debris is separate local debris. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [DC-15](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md) |
| Seal rejects trailing partial WAL bytes, missing/malformed active ref metadata, and mismatched active ref ownership before publication. | [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [DC-15](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md) |
| Seal persists WAL Patches, creates signed Block and RefState objects, promotes the pointer commit point, appends exactly one signed RefUpdate, confirms agreement, then drains active state. | [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [`refs/publication.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/publication.rs), [DC-38](https://github.com/nabbisen/prikk/blob/main/rfcs/accepted/DC-38-REF-PUBLICATION-CRASH-RECOVERY.md) |
| Seal verifies the configured MAINTAINER signer against repository-local trust before publication. | [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [`trust.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/trust.rs), [DC-11](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md) |
| Ref publication uses ref-specific locking, compare-and-swap checks, signed RefState/RefUpdate envelopes, pointer-first commit, and an idempotent exact log append. | [`refs/publication.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/publication.rs), [`pointer.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/pointer.rs), [`log.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/log.rs), [DC-38](https://github.com/nabbisen/prikk/blob/main/rfcs/accepted/DC-38-REF-PUBLICATION-CRASH-RECOVERY.md) |
| Immutable object publication never replaces an existing final name; existing or concurrent winners require valid identity/type and exact persisted-byte equality, while recognized crash-left temps remain warning-only debris. | [`object_store.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/object_store.rs), [`immutable.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/fsutil/anchored/immutable.rs), [DC-36](https://github.com/nabbisen/prikk/blob/main/rfcs/accepted/DC-36-EXISTING-OBJECT-PUBLICATION-INTEGRITY.md) |
| Doctor refuses format-1 missing-pointer reconstruction; exact interrupted ref publication completion requires retained active evidence and a trusted signer. | [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [DC-38](https://github.com/nabbisen/prikk/blob/main/rfcs/accepted/DC-38-REF-PUBLICATION-CRASH-RECOVERY.md) |
| Doctor began as read-only diagnostics, and current mutating repairs remain opt-in and narrow. | [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [PR-011](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-011-DOCTOR-HANDOFF.md), [PR-012](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-012-DOCTOR-REPAIR-HANDOFF.md), [PR-013](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-013-REF-RECOVERY-HANDOFF.md) |
| Ref pointer files are mutable pointers, not roots of trust. | [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [`pointer.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/pointer.rs), [data model](./data-model.md) |
| Durability/platform claims remain limited by current test evidence and Linux-only exercised gates. | [DC-24 baseline recap](https://github.com/nabbisen/prikk/blob/main/rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md), [DC-24](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-24-DATA-MODEL-TRUST-THREAT-DOCS.md), [DC-28](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-28-DURABILITY-CRASH-RECOVERY-REFERENCE.md) |

## Provenance

This reference follows the DC-26 documentation-home model: current-state references live in the
published mdBook, not under `rfcs/fdds/`. Its required-sync and ref-publication sections are updated
with the DC-37 and DC-38 implementations and remain subject to the combined 0.18.0 implementation and
release reviews.
