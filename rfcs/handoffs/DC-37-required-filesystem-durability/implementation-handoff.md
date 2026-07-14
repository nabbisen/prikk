# DC-37 Required Filesystem Durability - Implementation Handoff

## Authority and scope

This handoff executes accepted DC-37. It introduces shared required durability primitives and migrates
their classified call sites. It does not implement DC-36 immutable object publication or DC-38 ref
recovery, except for shared APIs and failpoint seams those accepted RFCs require.

## Implementation order

1. Add anchored repository/worktree root handles and relative no-follow directory operations for the
   supported Linux mutation path.
2. Implement required directory validation/creation with one-component creation and parent sync on
   both new and observed entries.
3. Split replace-allowed mutable metadata publication, immutable object publication API scaffolding,
   cross-directory ref promotion, and strict worktree sync into distinct named operations.
4. Migrate repository initialization, WAL, active metadata, ref/log, trust, lock, and worktree call
   sites according to DC-37's table. Do not implement DC-36 winner comparison in this increment.
5. Add failpoints and focused tests for each required open, file sync, directory sync, rename, and
   retained-state boundary.
6. Document the exact tested Linux filesystem and keep unproved mutation environments unsupported.

## Mandatory implementation notes

- Anchored relative operations are required. Reconstructed absolute-path check-then-use logic is not
  an acceptable substitute.
- A destination directory sync is the DC-34/DC-38 ref pointer commit point. Source directory sync is
  later cleanup durability; failure after destination success must not roll back the pointer.
- Lock acquisition failure after creation retains an actionable stale lock. `Drop` removal remains
  explicitly best-effort and is never called durable release.
- Trust key/policy failures must pin effective trust state and retry outcomes after each file boundary.
- Worktree sync failures propagate while leaving partial worktree state; they do not grant repository
  authority to the worktree.
- DC-36 must later add nonblocking no-follow same-handle object validation and observable crash-left
  temp warnings. Shared primitives must support those requirements without silently deleting debris.

## Required evidence before implementation review

- `cargo fmt --check`, workspace build/check, Clippy with warnings denied, and the full workspace tests;
- focused failpoint tests for directory-component parent sync, mutable publication, WAL/log first
  creation and append, trust updates, lock acquisition, strict worktree effects, and ref destination/
  source sync;
- retained-artifact and retry assertions for every injected failure;
- capability evidence naming the Linux filesystem and anchored/no-follow primitives exercised;
- no production, cross-platform mutation, implementation-complete, or release-readiness claim.
