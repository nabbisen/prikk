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
- Stale lock cleanup after a crash is manual today, through `prikk unlock` — not automatic, not a
  doctor repair, and not gated on the tool's own PID check, which is advisory only. There is no lock
  timeout or automatic lock stealing.
- The active-session model uses one default active WAL and active ref metadata. It is not a
  multi-active-session model.
- Durability and recovery claims are supported by current unit and integration tests, not by a
  completed crash-matrix or fuzzing campaign.
- Repository *mutation* is exercised by project gates on Linux, macOS, and Windows (DC-87
  Stage 2). Windows' anchoring guarantee is weaker than Linux/macOS in one stated way — see
  [platform support](./platform-support.md) for the exact gap and which of the eight remaining
  durability guarantees (G5 retired in DC-98) are held, weaker, or documented no-ops there.
  Read-only commands are CI-gated on macOS and Windows too — see
  [platform support](./platform-support.md).
- `.prikk/` is not a stable repository format and there is no stable migration policy yet.

## Lock Files and Scope

Prikk currently uses six lock types: the active-session lock, one per-ref lock, and four
container locks (RFC 102 Stage 6 Step 2) — one per compacting container plus the ref log:

```text
active/default/active.lock
refs/locks/<ref-name-storage-key>.lock
refs/containers/pointer-index.lock
refs/containers/log.lock
refs/containers/received-index.lock
trust/policy.lock
```

There is no temporary-candidate mechanism: ref publication writes into a shared, append-only pointer
container directly, and an append-only record has no candidate value to stage before becoming durable.

The lock primitive is the same for every lock kind above. The store creates the lock file with
exclusive file creation. If the file already exists, acquisition fails with `LockConflict`. When
acquisition succeeds, the file body records the current process id, lock kind, and a note that stale
lock stealing is not implemented — see [Stale Locks and Manual Cleanup](#stale-locks-and-manual-cleanup)
below for what that note no longer fully describes. The lock file and parent directory are
required-synced before acquisition succeeds. A post-create sync failure returns failure and
deliberately retains the lock as an actionable stale-lock state.

Lock release is best-effort file removal when the lock guard is dropped. If a process exits normally,
that usually removes the lock. If a process dies while holding the lock, the file can remain and later
commands fail closed instead of guessing whether the repository is safe to mutate.

The four container locks are acquired by whichever operation is touching that container — the writer
(ref publication, trust add/remove, bundle import) or `prikk compact`/`prikk compact --plan-only` — and
held for that operation's whole critical section, never just the final write. Multi-container
operations (ref publication touches the pointer-index and ref-log locks together; trust add touches
only the policy lock, since the key container is not lockable) acquire their whole set through one
internal helper that sorts it into a single fixed order first, so no call site can express an inverted
acquisition order.

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
- doctor WAL-tail repair acquires the active lock before its final publication guard and holds it
  through verification, truncation, and the post-repair report;
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

- the per-ref lock serializes Prikk publications and signer-backed completion for the same ref;
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
3. Persist the signed RefState object into its container.
4. Validate pointer/log state, including the empty state required for unborn-ref creation.
5. Check the current ref pointer against `expected_previous_ref_state_id`.
6. Append and required-sync the new pointer entry to the shared pointer-index container — the
   publication commit point. An append-only record has no candidate value to stage first, so this one
   durable append is both the check-then-write step and the promotion step the pre-container design
   needed two for.
7. Append and required-sync exactly one signed RefUpdate record to the shared log container.
8. Confirm pointer/log agreement before active state is removed.

Those checks prevent silent overwrite when the on-disk ref pointer has moved away from the caller's
expected baseline. They are not a global repository transaction, a distributed consensus protocol, or a
proof that every crash point has been exhaustively tested.

Seal takes `active.lock` first and then enters ref publication, which acquires the ref lock. Current
code does not acquire those locks in the reverse order.

## Interrupted Publication Locking

The pointer-index append is the publication commit point. If interruption leaves the pointer exactly
one transition ahead of the log, only signer-backed `seal` retry may finish publication. It takes the
active lock and the same ref-specific lock, revalidates retained WAL, RefState, Block, sequence,
old/new ids, and maintainer trust, then appends the exact deterministic RefUpdate. A structurally
incomplete final log frame may be truncated only by that path after the complete prefix verifies; the
shared log container has no pre-append refusal on an existing incomplete tail (unlike the pointer-first
check above), since a torn tail belonging to one ref never enters any other ref's own filtered
subsequence and so cannot block a different ref's publish.

Doctor diagnoses interrupted publication but does not sign, append, promote, or reconstruct a
missing pointer. `--repair-main-ref` is a recognized input for this and performs no repair — it is
always refused, regardless of repository state.

## Container Locking and Compaction

Four containers — the ref-pointer index, the ref log, the received-ref index, and the trust policy
container — each have their own lock, held for the whole critical section by whichever operation is
touching that container: an ordinary writer (ref publication, trust add/remove, bundle import) or
`prikk compact`/`prikk compact --plan-only`. This excludes a compaction run and an ordinary write from
interleaving; it is not about protecting the container's *content* the way CAS protects a ref's
baseline, but about protecting *which physical slot* is currently authoritative while it is being
read, written, or switched. For what a container lock actually protects against and how compaction
itself works, see [repository layout — Compaction](./repository-layout.md#compaction).

Ref publication acquires the ref-pointer-index and ref-log container locks together, in that order, in
addition to (not instead of) the per-ref lock above. Trust add/remove acquires the trust-policy
container lock in addition to `active.lock`. Bundle import acquires only the received-ref-index
container lock — it previously acquired no lock at all for this write, which is what surfaced the
container-locking work in the first place.

## Object Container Writes Are Not Among the Four Locked Containers

The content-addressed object containers (`write_object_to_container`, `index.rs`) are not one of
the four containers above — there is no `LockableContainer` variant for them, and no dedicated lock
file protects a write into one. An object write is safe under concurrency only when something else
already holds a lock across it:

- **`seal`** (`crates/prikk-cli/src/seal.rs`) acquires `ActiveLock` before persisting the WAL's
  Patch envelopes and building a Block, and holds it for the whole operation — object writes there
  are incidentally serialized against any other session by the same lock that serializes everything
  else `seal` does.
- **`import_bundle`** (`bundle.rs`) does not. Its object-writing loop runs before any lock is
  acquired at all; the received-ref-index container lock above is taken only afterward, to protect
  the received-pointer write, and covers none of the object writes that already happened.

This matters because `write_object_to_container`'s own write path (`index.rs`) reads the target
container's current length, then appends at that offset and records it in the index — a
read-then-append that is not atomic across two unsynchronized writers. Content-addressed
idempotency (RFC 102 Stage 3's same-id-same-bytes no-op, preserving the old `publish_immutable`
contract) makes writing the *same* object twice from two racing writers safe, but does not cover
two *different* concurrent unprotected appends into the same container computing offsets against
the same stale length.

**Known and accepted, not fixed here.** This predates DC-98 — it was previously recorded only as a
comment pointing at `FINDINGS.md` and a `DurabilityContract::publish_immutable` row, both since
removed, leaving it unregistered anywhere. Re-registered here in its own terms rather than by
reference to either. The mechanism that would close it (a dedicated object-container lock, or
extending `import_bundle` to hold `ActiveLock` across its object writes) is out of scope for this
page to design; it is deferred, tracked follow-up scope, not a defect this document is claiming is
fixed.

## Stale Locks and Manual Cleanup

If a process dies while holding any lock — `active.lock`, a ref lock, or one of the four container
locks — the lock file can remain. Current Prikk does not steal stale locks, expire them, or use doctor
to clear them, and this is deliberate, not merely unimplemented: automatically clearing a lock whose
process turns out to still be running would let two writers hold the same container simultaneously,
the exact race locking exists to prevent.

`prikk unlock` is the supported recovery path. A bare invocation lists every currently held lock, its
recorded process id, and a best-effort advisory on whether that process still appears to be running
(checked with `kill(pid, 0)` on Linux and macOS; unknown on other platforms). This check is
asymmetric on purpose: a positive result — the process still appears to be running — is reliable
evidence to refuse, because the check actually found it. A negative or unknown result is *not*
evidence the lock is safe to clear, since PID reuse after a reboot or PID-namespace isolation inside a
container can both make a genuinely running process appear absent. `prikk unlock --lock <path>` clears
one specific lock, after printing its details, and requires typing `yes` at an interactive prompt
unless `--yes` is passed for scripting — the tool never decides a lock is stale on its own; it reports
what it can check and lets the operator supply the fact it cannot.

Manual cleanup remains an operator decision, now made through a supported command rather than deleting
the file directly. It is only safe after confirming that no Prikk process is still writing the
repository. If the active WAL is non-empty, preserve the repository state and use `verify` / `doctor`
diagnostics before deciding whether clearing any lock is appropriate. Do not clear a lock to work
around a `ref CAS mismatch`; that error means the publication baseline is stale, not that a lock file
is blocking progress.

## Concurrent Operations Supported Today

The current model is conservative:

- one writer can hold the default active-session lock;
- one writer can hold the lock for a specific ref;
- different ref locks are separate files, so current storage code does not serialize all refs through a
  single global lock — but ref publication to *any* ref also acquires the shared ref-pointer-index and
  ref-log container locks, so two publications to different refs still serialize against each other on
  those, even though their per-ref locks differ;
- one writer or one `prikk compact` run can hold each of the four container locks;
- the default active WAL still serializes public command flows that author then seal active state;
- read-only verification, doctor analysis, history inspection, checkout planning, merge evidence, and
  merge planning do not create these lock files, though they still read mutable repository state.

This does not mean Prikk supports multi-user concurrent repository mutation, branch transactions,
remote synchronization, or race-free behavior under arbitrary concurrent filesystem modification.

## Deferred and Not Promised

Still deferred: multi-active sessions, distributed locking, remote sync, hosted-forge lock semantics,
branch transactions, lock expiry, **automatic** stale-lock recovery, broad active-session recovery,
complete crash-matrix testing, filesystem fault injection, fuzzing for WAL/ref-log recovery, macOS and
Windows filesystem validation, stable repository-format migration, multi-ref backup export, a
rehearsed repository-format-migration restore, and production-readiness claims. Single-ref
backup/restore tooling is no longer deferred — `prikk bundle export`/`verify`/`import`, see
[Backup and Restore](../guide/backup-restore.md). A best-effort, advisory PID check now exists
(`prikk unlock`, see
[Stale Locks and Manual Cleanup](#stale-locks-and-manual-cleanup) above) — what remains deferred is
*automatic* recovery, not the check itself; the tool still requires an explicit operator decision for
every lock it clears.

## Claim-to-Source Anchors

| Claim | Source anchors |
|---|---|
| Active and ref locks use exclusive anchored file creation, required-sync the lock file and parent directory, retain a stale lock on acquisition-sync failure, and attempt best-effort removal on drop. | [`lock.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/lock.rs), [DC-37](https://github.com/prikk-vcs/prikk/blob/main/rfcs/accepted/DC-37-REQUIRED-FILESYSTEM-DURABILITY.md) |
| Existing lock files fail closed as `LockConflict`, and current locks have no stale-lock stealing. | [`lock.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/lock.rs), [durability and crash recovery](./durability-recovery.md) |
| Active-session append holds `active.lock` before appending to the active WAL. | [`active.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/active.rs), [`wal.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/wal.rs) |
| Worktree patch authoring holds `active.lock` across the active-WAL guard and final WAL append, enforcing the current seal-before-second-commit behavior. | [`node_authoring.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/worktree_patch/node_authoring.rs), [DC-15](https://github.com/prikk-vcs/prikk/blob/main/rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md) |
| Rollback-draft append acquires `active.lock` before appending rollback-draft state. | [`rollback_draft.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/rollback_draft.rs), [DC-10](https://github.com/prikk-vcs/prikk/blob/main/rfcs/done/DC-10-ROLLBACK-DRAFT-SIGNING.md) |
| Seal acquires `active.lock`, validates active ref metadata, publishes through the ref store, then drains active state after successful publication. | [`seal.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/seal.rs), [`refs.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/refs.rs), [durability and crash recovery](./durability-recovery.md) |
| Non-empty active WALs require valid active ref metadata; missing or malformed metadata is an integrity issue. | [`active.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/active.rs), [`verify.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/verify.rs), [integrity and recovery diagnostics](./integrity-recovery.md) |
| Ref publication uses a per-ref lock, expected-current checks, signed RefState persistence, a durable pointer-index append as the commit point, then exactly one signed RefUpdate append to the shared log container. | [`refs/publication.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/refs/publication.rs), [`refs/pointer_index.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/refs/pointer_index.rs), [`refs/container.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/refs/container.rs), [DC-38](https://github.com/prikk-vcs/prikk/blob/main/rfcs/accepted/DC-38-REF-PUBLICATION-CRASH-RECOVERY.md) |
| Ref CAS mismatch returns `LockConflict` and is distinct from an existing lock-file conflict. | [`refs.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/refs.rs), [`lock.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/lock.rs) |
| Unborn ref publication is allowed only when the pointer is absent and the ref log is empty with no trailing partial bytes. | [`refs.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/refs.rs), [`seal.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/seal.rs), [DC-13](https://github.com/prikk-vcs/prikk/blob/main/rfcs/done/DC-13-NONDEFAULT-REF-GENESIS.md) |
| Doctor's `--repair-main-ref` input is recognized but always refused and performs no repair; exact interrupted publication completion requires signer-backed seal under the active and ref locks. | [`seal.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/seal.rs), [`doctor.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/doctor.rs), [DC-38](https://github.com/prikk-vcs/prikk/blob/main/rfcs/accepted/DC-38-REF-PUBLICATION-CRASH-RECOVERY.md) |
| Doctor repairs are opt-in and do not clear unsafe active sessions or define stale-lock cleanup. | [`doctor.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/doctor.rs), [integrity and recovery diagnostics](./integrity-recovery.md), [DC-29](https://github.com/prikk-vcs/prikk/blob/main/rfcs/done/DC-29-VERIFY-DOCTOR-INTEGRITY-RECOVERY-REFERENCE.md) |
| Four container locks (ref-pointer index, ref log, received-ref index, trust policy) are acquired by writers and `prikk compact` alike, sorted into one fixed order by a single acquisition helper before any lock is taken. | [`lock.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/lock.rs), [`compact.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/compact.rs) |
| `prikk unlock` lists every held lock with an advisory (not authoritative) liveness check of its recorded process id, and clears one named lock only after explicit confirmation or `--yes`. | [`unlock.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-store/src/unlock.rs), [`prikk-cli/src/unlock.rs`](https://github.com/prikk-vcs/prikk/blob/main/crates/prikk-cli/src/unlock.rs) |
| Repository path and durability claims for *mutation* remain limited by current test evidence and gates exercised on Linux, macOS, and Windows (DC-87 Stage 2); read-only commands are CI-gated cross-platform as of DC-71. | [durability and crash recovery](./durability-recovery.md), [path and worktree safety](./path-safety.md), [platform support](./platform-support.md), [DC-28](https://github.com/prikk-vcs/prikk/blob/main/rfcs/done/DC-28-DURABILITY-CRASH-RECOVERY-REFERENCE.md), [DC-32](https://github.com/prikk-vcs/prikk/blob/main/rfcs/done/DC-32-PATH-WORKTREE-SAFETY-REFERENCE.md) |

## Provenance

This reference implements DC-33 as a documentation-only extension of the DC-24 current-state
reference series. It adds no code, schema, CLI behavior, lock behavior, commit behavior, seal behavior,
verification behavior, doctor behavior, trust behavior, repository behavior, release semantics, or
repository-format stability guarantee.
