# RFC (done) - DC-33 Concurrency and Locking Reference

**Status.** Release candidate for 0.17.7.
**Target release.** 0.17.7.
**Tracks.** TASK-12 concurrency and locking model.
**Touches.** mdBook reference documentation, durability/recovery and repository-layout cross-links,
roadmap/status docs.
**Companion handoff.** None. This is a current-state concurrency reference and does not create a
gating FDD.

## Context

DC-24 established the current data model and trust/threat references. DC-26 moved current-state
references into the published mdBook. DC-28 documented durability and crash-recovery behavior, but it
intentionally left stale-lock policy to a future concurrency/locking reference. DC-31 documented the
physical `.prikk/` layout, including active and ref lock paths. DC-32 documented path and worktree
safety.

The next documentation gap is the local concurrency model. Users can encounter `active.lock` conflicts
after concurrent commands or a crash, and ref publication has compare-and-swap semantics that are
important for understanding failed seals and recovery. Those facts are currently spread across
durability, repository-layout, data-model, and code-level comments.

DC-33 should provide one current-state reference for local locks, active-session discipline,
ref-specific publication locks, compare-and-swap checks, narrow ref repair locking, and stale-lock
limits. It should not change locking behavior or promise distributed, remote, or production-grade
concurrency semantics.

## Problem

1. **Lock errors are user-visible.** `active.lock` and ref lock conflicts can surface during normal
   command use, but there is no single user-facing page explaining what those locks protect.
2. **Stale lock cleanup is manual.** If a process dies while holding `active.lock` or a ref lock, the
   current implementation has no lock stealing, timeout, owner validation, or automatic stale-lock
   cleanup.
3. **Active-session discipline is intentionally narrow.** The current model uses the default active
   WAL and active ref metadata. Worktree authoring prevents a second active commit before seal, and
   seal owns the publication/drain path.
4. **Ref publication combines locks and optimistic checks.** Ref publication uses a ref-specific lock,
   signed RefState and RefUpdate evidence, repeated expected-current checks, a temporary candidate
   pointer, and rename promotion. That sequence is easy to overclaim if described as a general
   transaction model.
5. **Ref repair has its own lock boundary.** Missing pointer reconstruction uses the ref lock and
   writes only a pointer from already-valid log, RefState, and Block evidence.
6. **The current scope is local-only.** There is no global repository lock, distributed lock, remote
   synchronization, multi-active-session model, or complete crash-matrix proof.

## Design Goals

1. Add a current-state reference page at `docs/src/reference/concurrency-locking.md`.
2. Document lock files and their storage locations:
   `active/default/active.lock`, `refs/locks/<ref-name-storage-key>.lock`, and
   `refs/tmp/<ref-name-storage-key>.tmp`.
3. Document the current lock primitive: exclusive file creation with `create_new(true)`, a small lock
   body containing process id/kind/note, fsync of the lock file, best-effort parent-directory sync, and
   best-effort file removal on drop.
4. Explain that lock acquisition fails closed with `LockConflict` when the file already exists.
5. Document stale-lock limits honestly: no timeout, no process-owner validation, no automatic lock
   stealing, no doctor repair for unsafe active sessions, and manual cleanup only after the operator
   confirms no Prikk process is writing the repository.
6. Document active-session locking:
   - `commit` / worktree patch authoring holds `active.lock` across the active-WAL emptiness/ref-owner
     guard, patch construction boundary, and WAL append;
   - rollback-draft append uses the active lock before appending rollback-draft state;
   - seal holds `active.lock` while replaying the active WAL, validating active ref metadata, publishing
     the ref, and draining active state;
   - non-empty active WALs require valid active ref metadata;
   - current worktree authoring remains single active-commit-before-seal for a given active WAL.
7. Document ref-specific locking:
   - publication and missing-pointer reconstruction acquire the lock for the affected ref;
   - locks are per ref storage key, not a global repository lock;
   - unrelated refs are not serialized by a single global lock in current storage code.
8. Document current ref publication and CAS behavior precisely:
   - publication validates inputs and acquires the ref lock;
   - unborn creation requires the ref pointer to be absent and the ref log to be empty with no trailing
     partial bytes;
   - the signed RefState object is written before publication;
   - the current ref pointer is checked against the expected previous RefState before and after ref-log
     append, and again before pointer promotion;
   - a signed RefUpdate record is appended to the ref log;
   - a candidate pointer is written under `refs/tmp/`;
   - the candidate pointer is renamed into `refs/by-id/` and the parent directory is best-effort synced;
   - CAS mismatch surfaces as `LockConflict`, not silent overwrite.
9. Document ref repair locking: missing `heads/main` pointer repair uses the ref lock, reconstructs only
   from valid ref-log/RefState/Block evidence, checks the pointer is still absent before promotion, and
   does not repair malformed logs, missing objects, signatures, trust policy, or key material.
10. Cross-link repository layout, data model, durability/recovery, integrity/recovery diagnostics, and
    signing setup where appropriate.
11. Include visible claim-to-source anchors for each lock, active-session, ref publication, repair, and
    caveat claim.

## Non-goals

DC-33 does not add:

- code, schema, CLI behavior, repository behavior, lock behavior, seal behavior, commit behavior,
  verification behavior, doctor behavior, trust behavior, release semantics, or repository-format
  stability guarantees;
- automatic stale-lock detection, stale-lock stealing, lock expiry, PID validation, or doctor stale-lock
  repair;
- distributed locking, remote synchronization, hosted-forge locking semantics, or filesystem lease
  semantics;
- a global repository lock;
- multi-active-session support;
- branch switching, branch copy/fork, tags/remotes, sync, or concurrent multi-ref transaction support;
- a formal transaction model or complete crash-matrix proof;
- new diagnostics, error-code taxonomy, JSON output, or public Rust API stabilization;
- a new current-state FDD under `rfcs/fdds/`.

## Proposed Documentation Shape

Create:

```text
docs/src/reference/concurrency-locking.md
```

Add it under the mdBook `# Reference` section near repository layout and durability:

```md
- [Concurrency and Locking](reference/concurrency-locking.md)
```

The page should be a current-state reference. It should describe current behavior and current gaps, not
future lock policy.

### Required Sections

The implemented page should contain at least:

1. **Core Caveats.** Early implementation, local-only locks, no stale-lock stealing/timeout, no
   distributed lock, no global repository lock, no complete crash-matrix proof, Linux-only exercised
   gates, and no stable repository-format guarantee.
2. **Lock Files and Scope.** `active.lock`, `refs/locks/*.lock`, `refs/tmp/*.tmp`, exclusive create,
   lock body, fsync behavior, release-on-drop best effort, and `LockConflict` on existing files.
3. **Active Session Locking.** Active WAL ownership, active ref metadata, worktree authoring, rollback
   draft append, seal, single active WAL, and single active-commit-before-seal behavior.
4. **Ref Publication Locking and CAS.** Per-ref lock scope, expected previous RefState, unborn-ref
   checks, RefState object write, ref-log append, repeated expected-current checks, candidate pointer,
   rename promotion, and CAS mismatch failure.
5. **Ref Repair Locking.** Missing pointer reconstruction boundary and what repair refuses to do.
6. **Stale Locks and Manual Cleanup.** Operator-facing guidance that cleanup is manual and safe only
   after checking no Prikk process is writing the repository; no doctor auto-clear claim.
7. **Concurrent Operations Supported Today.** What can run concurrently in principle versus what the
   current default active WAL serializes or rejects.
8. **Deferred and Not Promised.** Multi-active sessions, distributed locks, remote sync, branch
   transactions, lock expiry, PID checks, broad active-session recovery, crash-matrix/fuzzing, and
   cross-platform filesystem validation remain deferred.
9. **Claim-to-Source Anchors.** Code/docs/RFC anchors for lock primitive, active session, ref
   publication, ref repair, doctor/verify limits, and caveats.

### Stale-Lock Wording Guard

The page must not instruct users to delete a lock blindly. It should state that a stale lock may remain
after a crashed process and that manual cleanup is currently the only option, but it must frame cleanup
as safe only after the operator has confirmed no Prikk process is still running against that
repository. It should also state that `doctor` does not currently clear unsafe active sessions or
implement stale-lock repair.

### CAS Wording Guard

The page must describe current CAS as repeated expected-current checks around publication steps under a
ref-specific lock. It must not claim a complete multi-file transaction, global serializability,
distributed consensus, or a proof that every crash point is harmless beyond the current durability and
repair evidence.

## Required Source Audit

The implementation must check the final page against:

- `crates/prikk-store/src/lock.rs`
- `crates/prikk-store/src/active.rs`
- `crates/prikk-store/src/wal.rs`
- `crates/prikk-store/src/refs.rs`
- `crates/prikk-store/src/refs/log.rs`
- `crates/prikk-store/src/refs/pointer.rs`
- `crates/prikk-store/src/doctor.rs`
- `crates/prikk-store/src/verify.rs`
- `crates/prikk-store/src/rollback_draft.rs`
- `crates/prikk-store/src/worktree_patch/node_authoring.rs`
- `crates/prikk-cli/src/seal.rs`
- `docs/src/reference/data-model.md`
- `docs/src/reference/durability-recovery.md`
- `docs/src/reference/integrity-recovery.md`
- `docs/src/reference/repository-layout.md`
- `docs/src/reference/path-safety.md`
- `rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md`
- `rfcs/done/DC-28-DURABILITY-CRASH-RECOVERY-REFERENCE.md`
- `rfcs/done/DC-29-VERIFY-DOCTOR-INTEGRITY-RECOVERY-REFERENCE.md`
- `rfcs/done/DC-31-REPOSITORY-LAYOUT-AUTHORITY-REFERENCE.md`
- `rfcs/done/DC-32-PATH-WORKTREE-SAFETY-REFERENCE.md`
- `rfcs/done/PR-007-REF-PUBLICATION-HANDOFF.md`
- `rfcs/done/PR-013-REF-RECOVERY-HANDOFF.md`

## Review Requirements

Architect review should verify:

1. The proposed scope is documentation-only and does not imply lock, CLI, schema, verification, doctor,
   or repository behavior changes.
2. The lock primitive description matches current `ActiveLock` and `RefLock` behavior.
3. The stale-lock language is honest and does not imply timeout, PID validation, lock stealing, or
   doctor auto-repair.
4. The active-session model is described as the current default active WAL plus active ref metadata,
   not as multi-session support.
5. The ref publication sequence and CAS language match `RefStore::publish` precisely enough for user
   documentation.
6. The repair boundary matches `reconstruct_missing_ref_from_log` and current doctor limits.
7. The design avoids distributed, global-lock, stable-format, production-readiness, and complete
   crash-proof claims.
8. The required source audit is sufficient for implementation review.

## Acceptance Criteria

DC-33 is ready for implementation only after architect design review accepts this RFC or accepts a
repaired version. Implementation is complete when:

- the reference page exists and is linked in mdBook navigation;
- relevant current guide/reference pages link to it without duplicating the full page;
- claim-to-source anchors are included;
- `ROADMAP.md`, `rfcs/README.md`, and `rfcs/IMPLEMENTATION-STATUS.md` are updated consistently;
- documentation build/check commands pass in the implementing thread.
