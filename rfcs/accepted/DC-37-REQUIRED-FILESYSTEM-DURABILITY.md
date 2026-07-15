# RFC (accepted) - DC-37 Required Filesystem Durability

**Status.** Accepted after architect re-review on 2026-07-15; implementation accepted and committed
on 2026-07-15.
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

### Required directory creation and validation

Authoritative directory preparation uses one required primitive rooted at a validated repository
directory. It walks the relative path one component at a time without following symlinks. For every
component:

1. inspect the actual directory entry without following symlinks;
2. if it exists, require a directory, reject symlinks and special files, and required-sync its parent
   before using the component;
3. if it is absent, create only that component, required-sync its parent to establish the new name,
   then inspect the created entry again without following symlinks and require a directory;
4. descend only after the parent sync and no-follow validation succeed.

The primitive never uses an unchecked recursive `create_dir_all` for authoritative paths. A
parent-sync failure retains the observed or newly created child and returns failure. Retry performs the
same no-follow walk and parent syncs again; merely observing an existing entry is not proof that a prior
failed attempt made it durable. This rule applies to repository initialization, dynamic object shards,
and defensive directory preparation before authoritative writes. Supported mutation implementations
must bind initialization to a validated existing worktree-root handle, bind existing repositories to a
validated `.prikk` root handle, and use anchored relative no-follow operations from those handles. A
check-then-use sequence over reconstructed absolute paths does not satisfy this contract. A platform
that cannot enforce the anchored contract is unsupported for mutation.

### Cross-directory ref promotion

Ref candidate promotion is a distinct operation, not an invocation of the same-directory mutable
writer. Before rename, the complete candidate file and `refs/tmp` entry are required-synced. Promotion
then follows this order:

1. atomically rename the validated candidate from `refs/tmp` to its validated destination in
   `refs/by-id`;
2. required-sync `refs/by-id`; this establishes the accepted DC-34/DC-38 pointer commit point;
3. required-sync `refs/tmp`; this establishes durable candidate removal.

If destination sync fails, return interrupted-publication failure immediately. Do not roll back the
destination, sync the source directory, or perform cleanup that can hide the retained state. Retry
classifies both names and validates any destination pointer under DC-38. If destination sync succeeds
but source sync fails, publication is committed but cleanup is incomplete; return failure and let retry
classify the pointer and candidate state. No failure path blindly removes the destination pointer.

### Call-site classification and helper ownership

| Existing boundary | Class | Required 0.18.0 behavior |
|---|---|---|
| Repository initialization directories and `FORMAT` | Authoritative repository metadata | Create one component at a time and required-sync each parent; observed components are no-follow validated and parent-synced again on retry. `FORMAT` publication sync is propagated; failure retains created state. |
| Immutable object publication | Authoritative repository metadata | Required directory preparation establishes every shard entry. DC-36's no-clobber writer owns candidate file sync, final install, and both containing-directory syncs. Exact-existing success also required-syncs the containing directory. |
| WAL first creation, append, and truncation | Authoritative active-session state | File and parent-directory sync errors fail the operation. Written bytes remain for replay classification. |
| Active-ref metadata write and removal | Authoritative active-session state | Publication/session transitions do not report success until required file and directory syncs complete; failed cleanup remains visible and classifiable. |
| Ref candidate, pointer promotion, and ref-log creation/append | Authoritative publication state | Candidate/file and parent-directory sync errors propagate according to the accepted DC-34/DC-38 state machine. No failed post-publication sync triggers blind rollback. |
| Trust key and policy publication | Authoritative local trust state | File and parent-directory sync errors propagate. DC-37 does not claim multi-file transaction atomicity or silently weaken either file's durability. |
| Lock acquisition | Local concurrency-control authority | Acquisition succeeds only after the lock file and parent directory are required-synced. A failure after creation returns failure and retains the lock for explicit stale-lock handling rather than deleting uncertain state. |
| Lock removal from `Drop` | Cleanup durability | Destruction cannot return an error, so deletion and its sync remain explicitly best-effort. Failure may leave a stale lock and must not be described as durable release. |
| Worktree file writes, directory creation, and patch deletion | Worktree side effect | A separately named strict worktree-sync boundary propagates errors and leaves partial worktree state. It does not confer repository authority or reuse a best-effort repository helper. |
| Cache or disposable quarantine cleanup | Non-authoritative cleanup | Best-effort behavior is allowed only at an explicitly named boundary with no authority claim. No current authoritative caller may be reclassified as cache cleanup. |

The current replace-allowed `write_file_atomically` and DC-36's immutable writer become separate APIs.
The mutable metadata writer uses a unique same-directory `create_new` temp, complete file sync,
replace-allowed atomic rename, and required parent-directory sync. A post-rename sync failure returns
failure while retaining the final name for verification/retry. The immutable writer follows DC-36 and
never replaces a final object. Redundant caller-side syncs after either writer are removed so each
durability boundary has one documented owner.

### 0.18.0 mutation support matrix

| Environment | Repository mutation support |
|---|---|
| Linux local filesystem providing anchored relative no-follow operations, nonblocking final-entry open, regular-file and directory sync, atomic rename, and no-clobber hard-link/install | Supported experimentally when capability probes and focused failure tests pass. |
| Linux network, pseudo, FUSE, or other filesystem missing any required primitive | Unsupported for mutation; fail before publication where capability is known, otherwise fail at the primitive and retain state. |
| macOS | Read-only/diagnostic use only for 0.18.0 unless the same required primitive and crash tests pass before RC review. |
| Windows | Read-only/diagnostic use only for 0.18.0 unless a reviewed equivalent primitive and the same tests pass before RC review. |

Platform name alone is not proof of filesystem capability. The release docs must state the actually
observed environments and keep untested targets unsupported for mutation.

### Retained-artifact rules

| Failed boundary | Required retained state and result |
|---|---|
| Directory component creation followed by parent-sync failure | Created child remains untrusted for durability; return failure. Retry no-follow validates it and required-syncs its parent before use. |
| Temp/candidate write or file sync | No authoritative final-name claim; unique temp may remain; return failure. |
| No-clobber install | Existing winner remains untouched; candidate temp may remain; return failure or compare winner on `AlreadyExists`. |
| Pointer/object rename or install followed by directory-sync failure | Final name may exist; never roll it back blindly; return failure and require verify/retry classification. |
| WAL/ref-log append file-sync failure | Written prefix/tail remains; return failure; replay classifies complete record versus incomplete tail. |
| First-create directory-sync failure | Created file remains; return failure; retry revalidates exact bytes/state. |
| Cross-directory ref rename followed by destination-sync failure | Destination may exist and source may be absent; return interrupted-publication failure immediately without source sync or rollback. Retry classifies both names. |
| Ref destination sync succeeds but source-sync fails | Pointer is committed and candidate removal is not proven durable; return cleanup-incomplete failure. Retry validates the committed pointer and classifies source state. |
| Temp/debris unlink or cleanup directory-sync failure | Authoritative final state remains; return failure or explicit cleanup-incomplete state; never report fully durable cleanup. |

## Required analysis and tests

- inventory every current helper call and classify it as authoritative, local/session, worktree, or
  cleanup durability;
- inject existing-component validation, component creation, and component-parent sync failures at
  every authoritative directory class;
- inject directory open and sync failures at each authoritative class;
- inject file sync, no-clobber install, rename, and cleanup failures as separate boundaries;
- inject ref-promotion destination-sync and source-sync failures separately and verify the DC-38 state
  classification;
- pin effective trust state and retry behavior after each key-file and policy-file publication failure;
- prove command failure is propagated and later verification/retry classifies retained state;
- document the exercised platform/filesystem boundary without claiming cross-platform proof.

## Non-goals

- No claim that fsync alone proves all hardware/filesystem durability.
- No distributed storage, journaling replacement, backup, or cross-platform certification.
- No ref publication reordering; DC-38 owns that state machine and depends on this policy.

## Dependencies and acceptance

DC-37 can begin after design review and is the first M1 storage implementation. DC-36 and DC-38
implementation must use its accepted required-sync API and failure semantics. Completion requires the
call-site inventory, injected-failure tests, and corrected durability documentation.
