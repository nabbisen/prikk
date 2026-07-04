# DC-13 FDD-02 Update - Active Ref Ownership and Unborn Branch Publication

Status: Implemented for v0.6.0
Related RFC: `../../done/DC-13-NONDEFAULT-REF-GENESIS.md`
Target FDD: FDD-02 Storage Transaction Model

## Purpose

DC-13 allows first-commit genesis on explicit non-default branch refs. The storage transaction model
must therefore record which ref owns the current active WAL and must preserve the existing ref-log
recovery distinction for every branch ref.

## Required FDD-02 Body Updates

### Active-Session Metadata

Add active-session metadata:

```text
.prikk/active/default/ref-name
```

The file stores the exact UTF-8 ref name that owns the current active WAL. It is local mutable session
metadata, not a content-addressed object and not part of Patch identity.

Rules:

- the first commit into an empty active WAL writes and fsyncs the file under the active lock before
  appending the first WAL record;
- metadata creation or replacement must use durable file creation semantics before the first WAL append:
  write a temporary file under the active directory, fsync it, atomically rename it to
  `active/default/ref-name`, and fsync the active-session directory;
- the file content is the exact validated canonical ref string, with no trailing newline and no lossy
  conversion;
- a crash after metadata fsync but before WAL append leaves empty WAL plus stale metadata, which is
  safe and non-authoritative;
- any non-empty active WAL with missing or malformed metadata fails closed because DC-13 has no reliable
  legacy session discriminator;
- later commits fail closed when the active WAL is non-empty, even when metadata matches;
- seal reads it under the active lock and must match the requested `--ref`;
- empty WAL plus missing metadata is valid;
- empty WAL plus stale, malformed, or ref-mismatched metadata is non-authoritative and is removed after
  the lock is held, followed by an active-directory fsync;
- successful seal removes `active/default/ref-name` after draining the active WAL and fsyncs the
  active-session directory.

Seal must hold the active-session lock across metadata read/validation, WAL replay and partial-tail
validation, requested-ref comparison, ref publication, WAL drain, metadata removal, and active-directory
fsync. Ref-specific publication locking is still required inside that active-lock scope; the acquisition
order is active-session lock first, then ref-specific lock.

### Ref Publication

For an unborn branch ref, publication uses the existing `RefPublication` CAS contract:

- `expected_previous_ref_state_id = None`;
- `RefState.previous_ref_state_id = None`;
- `RefState.update_seq = 1`;
- `RefUpdate.old_ref_state_id = None`;
- `RefUpdate.update_seq = 1`.

DC-13 uses the existing signed ref publication semantics with `expected_previous_ref_state_id = None`.
The transaction must preserve the FDD-02 crash-safety discipline: the signed update event is durably
staged or journaled before pointer promotion. DC-13 must not define pointer-before-log as a normative
ordering.

The safe outline is:

1. acquire the ref-specific lock;
2. construct and sign the `RefState` and `RefUpdate` for the same transition;
3. write the signed RefState object;
4. durably stage or journal the signed RefUpdate transition according to FDD-02;
5. verify current pointer equals the expected previous state;
6. promote the ref pointer candidate atomically;
7. finalize the log entry and fsync files/directories according to the existing crash matrix.

### Genesis Eligibility

An absent ref pointer is eligible for genesis only when the corresponding ref log is absent, or exists
and is readable with zero records and zero trailing partial bytes. Any existing log history means
recovery/corruption, not unborn genesis. Any unreadable, malformed, or partial log is corruption, not
genesis.

This rule applies equally to `heads/main` and non-default `heads/*` refs.

### Recovery and Doctor

`doctor --repair-main-ref` remains limited to `heads/main`. DC-13 may diagnose recoverable non-default
refs from logs, but it must not add generalized repair without a separate design.

## Required Tests

- active-WAL ref metadata is written on first commit and checked on later commit;
- active-WAL ref metadata creation includes active-directory fsync before the first WAL append;
- non-empty WAL with missing or malformed metadata fails closed;
- a second commit before seal fails closed even when metadata matches;
- seal rejects a requested ref that differs from active-WAL ref metadata;
- successful seal removes active-WAL ref metadata after draining the WAL;
- empty WAL plus stale, malformed, or ref-mismatched metadata cleanup is deterministic;
- unborn non-default branch publication uses update sequence 1 and no previous ref state;
- absent pointer plus absent log is eligible genesis for a valid unborn branch ref;
- missing pointer with non-empty log blocks genesis for non-default refs;
- `doctor --repair-main-ref` does not repair `heads/topic`.
