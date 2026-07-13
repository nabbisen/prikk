# Integrity and Recovery Diagnostics

This page is the authoritative current-state reference for Prikk's repository verification and doctor
diagnostics. It describes what `prikk verify` checks, what it does not prove, how `prikk doctor`
interprets verification results, and which repair boundaries are intentionally narrow.

For the storage recovery mechanics behind WAL-tail truncation and ref-pointer reconstruction, see the
[durability and crash recovery](./durability-recovery.md) reference. For trust scope, see the
[trust and threat model](./trust-threat-model.md). For operator key input and local maintainer trust
setup, see the [security and signing setup](../guide/security-setup.md) guide.

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- `prikk verify` is read-only.
- `verify` checks structural integrity and current repository-local publication trust for publication
  objects; it is not a global trust proof.
- There is no repository-wide AUTHOR trust enforcement.
- There is no historical PKI, key revocation, key rotation, hardware signing, remote trust, sync trust,
  or stable migration policy yet.
- `prikk doctor` repairs are opt-in and narrow.
- Doctor recommendations are human guidance, not an automated recovery policy.
- Output fields, counters, severity labels, and issue-code names are current CLI vocabulary, not a
  stable machine-readable schema.

## Verify Scope

`prikk verify` calls the repository verification layer and prints a read-only report. Current
verification covers:

- persisted object placement by object type directory and canonical object path;
- object envelope decoding and recomputed object identity;
- Block payload decoding and references to parent Blocks, Patch objects, and optional snapshot Blobs;
- ref pointer and ref-log consistency;
- signed RefUpdate log record decoding;
- active WAL replay, including trailing partial WAL byte reporting;
- whether active WAL Patch records already exist as persisted Patch objects;
- active WAL ref metadata health;
- active rollback-draft WAL record classification;
- sealed rollback Block and sealed rollback Patch classification;
- repository-local publication trust for Block, RefState, and RefUpdate envelopes.

Publication trust issues are collected separately from hard structural verification errors. This lets
the command print a report while still returning command failure when publication trust is not valid.

## What Verify Does Not Prove

`verify` does not prove that a repository is globally trustworthy. It does not enforce
repository-wide AUTHOR trust, historical PKI semantics, key revocation, key rotation, remote identity,
remote trust, hosted forge policy, or thresholds beyond the current repository-local `required = 1`
maintainer policy.

`verify` also does not prove production readiness, stable repository-format migration, complete
cross-platform filesystem behavior, merge execution safety, semantic conflict resolution, backup
coverage, or successful recovery from every crash shape.

## Verify Output and Exit Behavior

The current CLI prints counters for checked objects, Blocks, rollback Blocks, sealed rollback Patches,
WAL records, persisted WAL Patches, refs, ref-log records, rollback draft WAL records, publication
trust records, publication trust issues, and trailing partial WAL bytes. It also prints the active WAL
metadata state.

The command exits with failure when:

- structural verification returns an error before a report can be produced;
- the report has a non-empty active WAL with missing or malformed active ref metadata; or
- the report has publication-trust issues.

Trailing partial WAL bytes are printed as a warning in the report. The recovery mechanics and safe
truncation boundary are covered by the [durability and crash recovery](./durability-recovery.md)
reference.

## Active WAL Metadata States

`ActiveWalMetadataStatus` currently has six states:

| State | CLI meaning | Doctor issue |
|---|---|---|
| `MissingForEmptyWal` | Empty active WAL with no metadata. | Healthy; no issue by itself. |
| `ValidForEmptyWal` | Empty active WAL with stale but valid metadata. | Warning: `PRIKK-DOCTOR-ACTIVE-REF-METADATA-DEBRIS`. |
| `InvalidForEmptyWal` | Empty active WAL with malformed metadata. | Warning: `PRIKK-DOCTOR-ACTIVE-REF-METADATA-MALFORMED-DEBRIS`. |
| `ValidForNonEmptyWal` | Non-empty active WAL with valid ownership metadata. | Healthy; no issue by itself. |
| `MissingForNonEmptyWal` | Non-empty active WAL without ownership metadata. | Error: `PRIKK-DOCTOR-ACTIVE-REF-METADATA-MISSING`. |
| `InvalidForNonEmptyWal` | Non-empty active WAL with malformed ownership metadata. | Error: `PRIKK-DOCTOR-ACTIVE-REF-METADATA-MALFORMED`. |

Only the non-empty missing/malformed states are active-session integrity issues. Empty-WAL metadata
states are local debris warnings because no WAL records need ownership for publication.

## Doctor Scope

`prikk doctor` is an actionable diagnostic layer over repository verification. When verification
completes, doctor prints the verification report, emits issue lines with severity, code, message, and
recommendation, then prints an issue summary.

When verification fails before a report can be produced, doctor emits a verification-error issue and
recommends preserving the repository before attempting repair.

Doctor output is intended for human diagnostics. The issue-code strings and severity labels are
current CLI vocabulary, not a stable JSON/API contract.

## Doctor Issue Catalog

Current doctor severities are `info`, `warning`, and `error`.

| Code | Severity | Meaning |
|---|---|---|
| `PRIKK-DOCTOR-VERIFY-OK` | `info` | Repository verification completed without integrity errors. |
| `PRIKK-DOCTOR-WAL-TRAILING-PARTIAL` | `warning` | Active WAL has trailing bytes that look like an incomplete final record. |
| `PRIKK-DOCTOR-REF-POINTER-MISSING` | `warning` | `heads/main` pointer is missing but the ref log can recover a RefState. |
| `PRIKK-DOCTOR-REF-RECOVERY-ERROR` | `error` | `heads/main` recovery analysis failed. |
| `PRIKK-DOCTOR-ACTIVE-REF-METADATA-MISSING` | `error` | Active WAL has records but active ref metadata is missing. |
| `PRIKK-DOCTOR-ACTIVE-REF-METADATA-MALFORMED` | `error` | Active WAL has records but active ref metadata is malformed. |
| `PRIKK-DOCTOR-ACTIVE-REF-METADATA-DEBRIS` | `warning` | Active WAL is empty but stale valid ref metadata remains. |
| `PRIKK-DOCTOR-ACTIVE-REF-METADATA-MALFORMED-DEBRIS` | `warning` | Active WAL is empty but malformed ref metadata remains. |
| `PRIKK-DOCTOR-VERIFY-ERROR` | `error` | Repository verification failed before doctor could produce a healthy report. |

Publication-trust issues can also appear in doctor output as error-severity diagnostics using the
trust issue code and message from publication-trust verification. They are not part of the nine
doctor-owned issue codes above.

`MissingForEmptyWal` and `ValidForNonEmptyWal` are healthy metadata states and do not produce doctor
issues by themselves.

## Doctor Repair Boundary

Doctor currently exposes two repair switches:

- `--repair-wal-tail`;
- `--repair-main-ref`.

Repair refuses to run when repository health has error-severity issues. The detailed recovery
mechanics and safety preconditions for those repairs live in the
[durability and crash recovery](./durability-recovery.md) reference.

Doctor does not synthesize missing objects, repair malformed logs, repair checksum mismatches, repair
signatures, auto-trust keys, reconstruct trust policy, recover key material, clear unsafe active
sessions, or define stale-lock cleanup.

## Relationship to Rollback Verification

Repository `verify` counts active rollback-draft WAL records after classifying and decoding
rollback-marked Patch payloads under the supported replay subset. It also counts sealed rollback
Blocks and sealed rollback Patch references.

`prikk rollback-draft-verify` is a stronger selected-ref pre-seal check for one active rollback draft.
It verifies that the active WAL contains exactly one rollback draft and that the draft payload matches
the inverse Patch derived from the selected ref. See the
[rollback draft verification](../guide/rollback/rollback-draft-verify.md) guide for the command-level
boundary.

## Deferred Work

Still deferred: broader repair policy, stale-lock policy, missing-object recovery, malformed-log
repair, checksum-mismatch repair, object quarantine and garbage collection, repository-wide AUTHOR
trust policy, key rotation, revocation, hardware signing, remote trust, hosted identity, JSON output,
stable diagnostic schema, backup/restore tooling, stable repository-format migration, and production
readiness.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| Repository verification reports counters for objects, WAL records, Blocks, refs, ref logs, rollback material, publication trust, trailing partial WAL bytes, and active WAL metadata state. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [`output.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/output.rs), [PR-006](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-006-VERIFY-HANDOFF.md), [PR-010](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-010-VERIFY-HARDENING-HANDOFF.md) |
| Verification checks object placement, envelope decoding, object identity, Block references, ref pointer/log consistency, WAL replay, rollback classification, and publication trust. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [data model](./data-model.md) |
| Publication trust checks Block, RefState, and RefUpdate envelopes against repository-local maintainer trust and reports issues separately. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [`trust.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/trust.rs), [DC-11](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md), [trust and threat model](./trust-threat-model.md) |
| `verify` command failure occurs for active-WAL metadata integrity issues or publication-trust issues after printing the report. | [`main.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/main.rs), [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs) |
| Active WAL metadata has six states, with two healthy states, two empty-WAL warning states, and two non-empty-WAL integrity states. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [DC-15](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md) |
| Doctor is a diagnostic layer over verification with issue severities, issue codes, messages, recommendations, and an issue summary. | [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [`output.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/output.rs), [PR-011](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-011-DOCTOR-HANDOFF.md) |
| Doctor currently owns nine issue codes; publication-trust issues can also be surfaced by doctor using trust issue codes. | [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [`trust.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/trust.rs) |
| Doctor repairs are opt-in and limited to WAL-tail truncation plus guarded `heads/main` pointer reconstruction. | [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [`args.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/args.rs), [PR-012](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-012-DOCTOR-REPAIR-HANDOFF.md), [PR-013](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-013-REF-RECOVERY-HANDOFF.md), [durability and crash recovery](./durability-recovery.md) |
| Repository verification classifies rollback draft WAL records and sealed rollback material, while `rollback-draft-verify` performs a stronger selected-ref check. | [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [`rollback_verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/rollback_verify.rs), [PR-029](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-029-ROLLBACK-DRAFT-VERIFY-HANDOFF.md), [PR-030](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-030-SEALED-ROLLBACK-HISTORY-HANDOFF.md), [rollback draft verification guide](../guide/rollback/rollback-draft-verify.md) |
| Verify/doctor output is current CLI vocabulary, not a stable machine-readable schema. | [`output.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/output.rs), [DC-29](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-29-VERIFY-DOCTOR-INTEGRITY-RECOVERY-REFERENCE.md) |

## Provenance

This reference consolidates current released records through DC-29. It
follows the DC-26 documentation-home model: current-state references live in the published mdBook, not
under `rfcs/fdds/`. It is documentation-only and does not change verification, doctor, repair, trust,
CLI, object schema, repository format, or repository behavior.
