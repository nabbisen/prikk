# Concurrency and Locking

This page is the authoritative current-state reference for Prikk's local concurrency and locking
model. It explains what the current lock files protect, how active-session and ref publication writes
are serialized, how ref compare-and-swap checks fail, and where stale-lock recovery remains manual.

For physical paths and authority boundaries, see [repository layout and authority](./repository-layout.md).
For local persistence and crash-recovery behavior, see
[durability and crash recovery](./durability-recovery.md). For verification and doctor diagnostics,
see [integrity and recovery diagnostics](./integrity-recovery.md). For trust and signing boundaries,
see the [trust and threat model](./trust-threat-model.md) and the
[security and signing setup](../guide/security-setup.md) guide.

## Core Caveats

- Prikk is early implementation software and is not a production Git replacement.
- Current locks are local lock files. They are not distributed locks, remote coordination, hosted-forge
  locks, or filesystem leases.
- There is no global repository lock today.
- Lock conflicts and stale-baseline ref publication conflicts both surface as `LockConflict`, but they
  have different causes and operator responses.
- Stale lock cleanup after a crash is manual today. There is no lock timeout, lock stealing, process
  owner validation, or doctor stale-lock repair.
- The active-session model uses one default active WAL and active ref metadata. It is not a
  multi-active-session model.
- Durability and recovery claims are supported by current unit and integration tests, not by a
  completed crash-matrix or fuzzing campaign.
- Linux is the only platform exercised by the current project gates; cross-platform filesystem
  locking, fsync, and rename behavior remain design targets.
- `.prikk/` is not a stable repository format and there is no stable migration policy yet.

## Lock Files and Scope

Prikk currently uses two lock types:

```text
active/default/active.lock
refs/locks/<ref-name-storage-key>.lock
```

It also uses temporary ref pointer candidates during ref publication and repair:

```text
refs/tmp/<ref-name-storage-key>.tmp
```

The lock primitive is the same for active-session and ref locks. The store creates the lock file with
exclusive file creation. If the file already exists, acquisition fails with `LockConflict`. When
acquisition succeeds, the file body records the current process id, lock kind, and a note that stale
lock stealing is not implemented. The lock file is fsynced, and the parent directory is best-effort
synced.

Lock release is best-effort file removal when the lock guard is dropped. If a process exits normally,
that usually removes the lock. If a process dies while holding the lock, the file can remain and later
commands fail closed instead of guessing whether the repository is safe to mutate.

These lock files are local synchronization state. They are not history, trust evidence, publication
evidence, or object identity.

## Active Session Locking

The default active session stores pending Patch envelopes before seal:

```text
active/default/queue.wal
active/default/ref-name
active/default/active.lock
```

The active lock protects writes to this active-session state. Current command paths acquire
`active.lock` before mutating or sealing the default active WAL:

- worktree patch authoring holds the active lock across the active-WAL emptiness/ref-owner guard, patch
  authoring boundary, and final WAL append;
- rollback-draft append acquires the active lock before appending rollback-draft state;
- the active-session append helper acquires the active lock before appending a signed Patch envelope;
- seal acquires the active lock before replaying the WAL, checking active ref metadata, publishing the
  ref, and draining active state after successful publication.

The active WAL is paired with active ref metadata. A non-empty active WAL must have valid metadata
identifying the local branch ref that owns those pending records. Missing, malformed, or mismatched
metadata fails closed; seal does not guess the publication target.

Current worktree authoring is single active-commit-before-seal for the default active WAL. A second
commit before seal either loses the active lock or, after the first commit releases the lock, sees the
non-empty WAL and fails with guidance to seal first.

## Ref Publication Locking and CAS

Ref publication uses a ref-specific lock and repeated expected-current checks. These are related but
distinct mechanisms:

- the per-ref lock serializes Prikk publications and missing-pointer reconstructions for the same ref;
- the expected-current checks reject stale-baseline publication when the caller's expected previous
  RefState no longer matches the current pointer.

A lock conflict such as `active lock already exists` or `ref lock already exists` means another process
may still be holding a local lock, or a stale lock file may remain after a crash. A conflict such as
`ref CAS mismatch` means the ref's current pointer did not match the publication's expected previous
RefState. That is not fixed by deleting a lock file; the caller must re-read the current ref state and
rebuild or retry the publication from the new baseline.

Current ref publication is scoped to one ref:

1. Validate the publication inputs.
2. Acquire `refs/locks/<ref-name-storage-key>.lock`.
3. For unborn-ref creation, require the ref pointer to be absent and the ref log to be empty with no
   trailing partial bytes.
4. Write the signed RefState object.
5. Check the current ref pointer against `expected_previous_ref_state_id`.
6. Append a signed RefUpdate record to the ref log.
7. Check the current ref pointer against `expected_previous_ref_state_id` again.
8. Write a candidate pointer under `refs/tmp/`.
9. Check the current ref pointer against `expected_previous_ref_state_id` again.
10. Rename the candidate pointer into `refs/by-id/` and best-effort sync the pointer parent directory.

Those checks prevent silent overwrite when the on-disk ref pointer has moved away from the caller's
expected baseline. They are not a global repository transaction, a distributed consensus protocol, or a
proof that every crash point has been exhaustively tested.

Seal takes `active.lock` first and then enters ref publication, which acquires the ref lock. Current
code does not acquire those locks in the reverse order.

## Ref Repair Locking

`doctor --repair-main-ref` can reconstruct a missing `heads/main` pointer only from already-valid
evidence. The repair path acquires the same ref-specific lock used by publication. It no-ops if the
pointer already exists. Otherwise it replays the ref log, verifies the chain, checks referenced
RefState and Block evidence, writes a candidate pointer, checks the pointer is still absent, and
promotes the candidate.

This repair writes only the missing pointer file. It does not synthesize objects, repair malformed
logs, truncate ref-log tails, repair signatures, auto-trust keys, reconstruct trust policy, recover key
material, or clear unsafe active sessions.

A crash after appending a valid RefUpdate log record but before pointer promotion can leave a missing
pointer with valid log evidence. That is the narrow shape this repair path is designed to handle,
subject to the verification and repair limits documented in the
[durability and crash recovery](./durability-recovery.md) and
[integrity and recovery diagnostics](./integrity-recovery.md) references.

## Stale Locks and Manual Cleanup

If a process dies while holding `active.lock` or a ref lock, the lock file can remain. Current Prikk
does not steal stale locks, expire them, validate the recorded process id, or use doctor to clear them.

Manual cleanup is therefore an operator decision. It is only safe after confirming that no Prikk process
is still writing the repository. If the active WAL is non-empty, preserve the repository state and use
`verify` / `doctor` diagnostics before deciding whether any manual lock removal is appropriate. Do not
delete a lock file to work around a `ref CAS mismatch`; that error means the publication baseline is
stale, not that a lock file is blocking progress.

## Concurrent Operations Supported Today

The current model is conservative:

- one writer can hold the default active-session lock;
- one writer can hold the lock for a specific ref;
- different ref locks are separate files, so current storage code does not serialize all refs through a
  single global lock;
- the default active WAL still serializes public command flows that author then seal active state;
- read-only verification, doctor analysis, history inspection, checkout planning, merge evidence, and
  merge planning do not create these lock files, though they still read mutable repository state.

This does not mean Prikk supports multi-user concurrent repository mutation, branch transactions,
remote synchronization, or race-free behavior under arbitrary concurrent filesystem modification.

## Deferred and Not Promised

Still deferred: multi-active sessions, distributed locking, remote sync, hosted-forge lock semantics,
branch transactions, lock expiry, PID checks, automatic stale-lock recovery, broad active-session
recovery, complete crash-matrix testing, filesystem fault injection, fuzzing for WAL/ref-log recovery,
macOS and Windows filesystem validation, stable repository-format migration, backup/restore tooling,
and production-readiness claims.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| Active and ref locks use exclusive file creation, fsync the lock file, best-effort sync the parent directory, and remove the lock file on drop. | [`lock.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/lock.rs), [PR-007](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-007-REF-PUBLICATION-HANDOFF.md) |
| Existing lock files fail closed as `LockConflict`, and current locks have no stale-lock stealing. | [`lock.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/lock.rs), [durability and crash recovery](./durability-recovery.md) |
| Active-session append holds `active.lock` before appending to the active WAL. | [`active.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/active.rs), [`wal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/wal.rs) |
| Worktree patch authoring holds `active.lock` across the active-WAL guard and final WAL append, enforcing the current seal-before-second-commit behavior. | [`node_authoring.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/worktree_patch/node_authoring.rs), [DC-15](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md) |
| Rollback-draft append acquires `active.lock` before appending rollback-draft state. | [`rollback_draft.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/rollback_draft.rs), [DC-10](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-10-ROLLBACK-DRAFT-SIGNING.md) |
| Seal acquires `active.lock`, validates active ref metadata, publishes through the ref store, then drains active state after successful publication. | [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [durability and crash recovery](./durability-recovery.md) |
| Non-empty active WALs require valid active ref metadata; missing or malformed metadata is an integrity issue. | [`active.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/active.rs), [`verify.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/verify.rs), [integrity and recovery diagnostics](./integrity-recovery.md) |
| Ref publication uses a per-ref lock, expected-current checks, signed RefState persistence, signed RefUpdate log append, candidate pointer write, rename promotion, and parent-directory sync. | [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [`refs/log.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/log.rs), [`refs/pointer.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs/pointer.rs), [PR-007](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-007-REF-PUBLICATION-HANDOFF.md) |
| Ref CAS mismatch returns `LockConflict` and is distinct from an existing lock-file conflict. | [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [`lock.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/lock.rs) |
| Unborn ref publication is allowed only when the pointer is absent and the ref log is empty with no trailing partial bytes. | [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [`seal.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-cli/src/seal.rs), [DC-13](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-13-NONDEFAULT-REF-GENESIS.md) |
| Missing `heads/main` pointer reconstruction is guarded by the ref lock and writes only a missing pointer from already-valid ref-log, RefState, and Block evidence. | [`refs.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/refs.rs), [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [PR-013](https://github.com/nabbisen/prikk/blob/main/rfcs/done/PR-013-REF-RECOVERY-HANDOFF.md) |
| Doctor repairs are opt-in and do not clear unsafe active sessions or define stale-lock cleanup. | [`doctor.rs`](https://github.com/nabbisen/prikk/blob/main/crates/prikk-store/src/doctor.rs), [integrity and recovery diagnostics](./integrity-recovery.md), [DC-29](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-29-VERIFY-DOCTOR-INTEGRITY-RECOVERY-REFERENCE.md) |
| Repository path, durability, and platform claims remain limited by current test evidence and Linux-only exercised gates. | [durability and crash recovery](./durability-recovery.md), [path and worktree safety](./path-safety.md), [DC-28](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-28-DURABILITY-CRASH-RECOVERY-REFERENCE.md), [DC-32](https://github.com/nabbisen/prikk/blob/main/rfcs/done/DC-32-PATH-WORKTREE-SAFETY-REFERENCE.md) |

## Provenance

This reference implements DC-33 as a documentation-only extension of the DC-24 current-state
reference series. It adds no code, schema, CLI behavior, lock behavior, commit behavior, seal behavior,
verification behavior, doctor behavior, trust behavior, repository behavior, release semantics, or
repository-format stability guarantee.
