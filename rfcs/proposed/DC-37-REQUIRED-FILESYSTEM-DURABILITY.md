# RFC (proposed) - DC-37 Required Filesystem Durability

**Status.** Proposed; architect design review required.
**Target milestone.** M1 - 0.18.0 corrective release.
**Tracks.** Architect review B3.
**Touches.** Repository metadata fsync policy, `fsutil`, object/WAL/ref/lock/trust/worktree call sites,
error taxonomy, capability documentation, and injected-failure tests.

## Problem

`sync_directory_best_effort` suppresses directory open and `sync_all` failures at boundaries that
current durability claims treat as required. A command can report success even when repository
metadata durability was not established.

## Design

Replace the single ambiguous helper with explicit policy boundaries:

- `sync_directory_required` propagates directory open and sync failures as operation failures;
- an optional/best-effort helper may exist only for a named non-authoritative boundary whose contract
  explicitly permits it;
- object finalization, first WAL creation, ref candidate/pointer promotion, first ref-log creation,
  trust-policy/key publication, and other authoritative repository metadata use required sync;
- worktree materialization and lock-file cleanup are classified separately rather than inheriting a
  blanket repository rule.

Supported-platform exceptions must be represented by an explicit capability or policy decision. Error
suppression based only on an OS error kind is forbidden at required boundaries. A failed required sync
must leave recoverable artifacts in place and must not trigger cleanup that hides the interruption.

For object, WAL, ref, log, trust, and repository-format metadata, the invariant is absolute: required
file and directory sync succeeds, or the command returns failure and retains state needed for
verification/retry. A weaker durability mode may exist only after a separate RFC, explicit user
selection, persistent repository marking, and visible command output. There is no automatic downgrade,
permission-denied exception, or success-on-unsupported behavior.

### 0.18.0 mutation support matrix

| Environment | Repository mutation support |
|---|---|
| Linux local filesystem providing regular-file sync, directory sync, atomic rename, and no-clobber hard-link/install | Supported experimentally when capability probes and focused failure tests pass. |
| Linux network, pseudo, FUSE, or other filesystem missing any required primitive | Unsupported for mutation; fail before publication where capability is known, otherwise fail at the primitive and retain state. |
| macOS | Read-only/diagnostic use only for 0.18.0 unless the same required primitive and crash tests pass before RC review. |
| Windows | Read-only/diagnostic use only for 0.18.0 unless a reviewed equivalent primitive and the same tests pass before RC review. |

Platform name alone is not proof of filesystem capability. The release docs must state the actually
observed environments and keep untested targets unsupported for mutation.

### Retained-artifact rules

| Failed boundary | Required retained state and result |
|---|---|
| Temp/candidate write or file sync | No authoritative final-name claim; unique temp may remain; return failure. |
| No-clobber install | Existing winner remains untouched; candidate temp may remain; return failure or compare winner on `AlreadyExists`. |
| Pointer/object rename or install followed by directory-sync failure | Final name may exist; never roll it back blindly; return failure and require verify/retry classification. |
| WAL/ref-log append file-sync failure | Written prefix/tail remains; return failure; replay classifies complete record versus incomplete tail. |
| First-create directory-sync failure | Created file remains; return failure; retry revalidates exact bytes/state. |
| Temp/debris unlink or cleanup directory-sync failure | Authoritative final state remains; return failure or explicit cleanup-incomplete state; never report fully durable cleanup. |

## Required analysis and tests

- inventory every current helper call and classify it as authoritative, local/session, worktree, or
  cleanup durability;
- inject directory open and sync failures at each authoritative class;
- inject file sync, no-clobber install, rename, and cleanup failures as separate boundaries;
- prove command failure is propagated and later verification/retry classifies retained state;
- document the exercised platform/filesystem boundary without claiming cross-platform proof.

## Non-goals

- No claim that fsync alone proves all hardware/filesystem durability.
- No distributed storage, journaling replacement, backup, or cross-platform certification.
- No ref publication reordering; DC-38 owns that state machine and depends on this policy.

## Dependencies and acceptance

DC-37 can begin after design review. DC-38 implementation must use its accepted required-sync API and
failure semantics. Completion requires the call-site inventory, injected-failure tests, and corrected
durability documentation.
