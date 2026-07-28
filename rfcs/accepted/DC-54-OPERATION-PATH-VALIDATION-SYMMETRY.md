# RFC (accepted) - DC-54 Operation Path Validation Symmetry

**Status.** Accepted by the project owner on 2026-07-28, after the author's design-completion
self-critique (`prikk-dc54-design-completion-self-critique-v1.md`) resolved the blocking dependency-cycle
defect in the original draft (§"Sharing the validator requires moving it") and recorded the error-taxonomy,
compatibility, and identity-neutrality decisions below. That document is explicitly not an independent
design review and issued no verdict; this acceptance is the project owner's own call, exercised directly
rather than via a separate architect review round. Opened by a defect discovered during DC-41 stage 4's
property/fuzz campaign, per that RFC's discipline clause ("a discovered behavior defect opens a dedicated
corrective RFC instead of being silently normalized into a test expectation").
**Implementation.** Complete at `e8f780a`. Architect post-commit implementation review v1 (2026-07-28,
`prikk-dc54-post-commit-implementation-review-v1.md`) accepted with no repair required — every scrutiny
point from the review request independently re-verified against the working tree, including the
crate-boundary move the design-completion self-critique had flagged as most deserving of independent
review. One non-blocking hardening note (round-trip property's vacuity signalling) and one process
concern (design and implementation both landed with no independent review before commit) recorded in the
review; neither affects correctness of the committed state.
**Target milestone.** M2 - post-correction assurance milestone.
**Schedule position.** Independent; implementation authorized. Selected ahead of DC-51 because it closes a
live correctness gap in production code rather than a process gap.
**Tracks.** DC-41 stage-4 campaign finding; reproducer committed at
`crates/prikk-store/proptest-regressions/patch_replay/tests/proptest_round_trip.txt`.
**Touches.** Write-side `validate()` for four operation kinds in `prikk-object`. Production behaviour
change — encode becomes stricter.

## Problem

The operation wire codec is asymmetric: **encode accepts paths that decode rejects.**

Write-side `validate()` for every path-carrying operation kind checks only that `node_id` is non-zero:
`CreateFile` (`payload/patch.rs`), `DeleteNode`, `RenamePath` (`:515-525`), and `CreateSymlink`. None
validates its path fields. `encode_canonical` calls `validate()` and then writes the paths through
`field_repo_path`, so an operation carrying a path that violates the `RepoPath` grammar encodes
successfully.

Decode enforces the grammar: `decode_rename_path` calls `RepoPath::parse` on both `old_path` and
`new_path` (`patch_replay/decode.rs:456-457`), which rejects traversal, absolute paths, Windows-reserved
device names, and the other rules in `prikk-replay/src/path.rs`.

The minimized reproducer is a `RenamePath` with `new_path: "com1"` — encodes, then fails to decode with
`InvalidName("Windows reserved path component is not allowed: com1")`.

**This is not a decoder defect.** `decode_patch_operations` returns a clean `Err` and never panics, so
NFR-SEC-04 holds and the decoder's enforcement is the correct behaviour that exposed the gap. The defect is
the missing write-side validation.

**Live exposure.** `RenamePath` is not currently reachable through authoring (`ensure_apply_supported`
returns `UnsupportedObjectType`; application is deferred to a later node-model increment). **`CreateFile`
is wired into authoring today**, so the same class of gap is live for it if a caller ever constructs one
with an unsafe path bypassing worktree-layer validation.

Affected fields: `CreateFile.path`, `DeleteNode.path`, `RenamePath.old_path`, `RenamePath.new_path`,
`CreateSymlink.path`. **Not** affected: `DeleteNode`'s symlink `old_target` and `CreateSymlink.target`,
which decode reads as plain strings without a `RepoPath::parse` call — those are opaque targets by design
under the DC-40 FDD, and this RFC must not tighten them by accident.

## Design

Make encode reject exactly what decode rejects, for the five affected fields:

1. Add path-grammar validation to the write-side `validate()` of `CreateFile`, `DeleteNode`, `RenamePath`,
   and `CreateSymlink`.
2. Use **one** implementation shared with decode, so the two sides cannot drift. A second, independently
   written validator would recreate this defect in a new form.
3. Leave `DeleteNode.old_target` and `CreateSymlink.target` untouched; their opacity is an accepted DC-40
   decision, not an oversight.

### Sharing the validator requires moving it (design amendment, 2026-07-28)

An earlier draft of this RFC said to reuse `prikk-replay`'s `RepoPath` rules directly. **That is
unimplementable:** the `validate()` methods are in `prikk-object`, `validate_repo_path` is in
`prikk-replay`, and `prikk-replay` already depends on `prikk-object` — so `prikk-object → prikk-replay`
would be a dependency cycle.

Resolution: **move `validate_repo_path` and its private helper `validate_component` down into
`prikk-object`** (new `crates/prikk-object/src/path.rs`). Both are pure string logic over
`prikk_error::{PrikkError, Result}` and `std`, and `prikk-object` already depends on `prikk-error`, so no
new dependency is introduced. `prikk-replay::RepoPath::parse` then calls the moved function and
re-exports it, keeping `prikk_replay::validate_repo_path` and `prikk_store::validate_repo_path` working
for every existing caller.

`prikk-replay` retains the `RepoPath` **type** and `validate_no_path_collisions` (set-level, and not
needed here), so this is a narrow downward move of pure lexical validation — not of layout,
materialization, or lifecycle semantics. It touches a boundary DC-19/DC-20 deliberately drew and is the
part of this RFC most deserving of independent scrutiny.

## Decisions recorded

1. **Error taxonomy: propagate `InvalidName` unchanged** — call `validate_repo_path(&self.path)?` with no
   wrapping. Symmetry is the point of the increment, so encode and decode should produce identical error
   text; wrapping would discard the specific rule broken. The resulting difference from the sibling
   `node_id` check (`CanonicalEncoding`) is correct rather than inconsistent: a zero `node_id` is a
   reserved *encoding* value, an unsafe path is an invalid *name*. Apply uniformly across the four kinds.
2. **Existing-repository compatibility: resolved, low risk.** Worktree authoring already validates paths
   before constructing operations (`worktree_patch/node_authoring/worktree_files.rs:72`,
   `node_authoring.rs:340`), and an invalid path could never have round-tripped because decode rejects it.
   Residual exposure is a direct API caller bypassing the worktree layer — exactly the hole this closes.
   No fixture encodes an invalid path. The implementer still evidences this rather than restating it.
3. **Identity neutrality.** Tightening encode rejects previously-encodable values; it does not alter the
   bytes produced for any value that already encodes *and* decodes. No accepted byte sequence changes,
   therefore no existing ObjectId changes. Must be stated explicitly, because DC-39 and DC-40 froze
   identity.

## Non-goals

- No change to the decoder, the `RepoPath` grammar, or the FDD-03 §9.3 wire format.
- No change to the opaque symlink-target fields.
- No new operation kind, and no lifting of `RenamePath`'s deferred application status.
- No relaxation of decode to accept what encode currently produces — that would weaken path safety and is
  the wrong direction.

## Acceptance criteria

Encode and decode accept exactly the same set of path values for the five affected fields. The committed
DC-41 reproducer fails at encode with a clear error rather than at decode. The `path_segment_strategy()`
exclusion in `proptest_round_trip.rs` is **removed**, and the unfiltered campaign budget passes — that
removal is the real proof this is fixed, and the disclosure comment at the exclusion site should be
retired with it. Existing-repository compatibility and identity-neutrality are each evidenced rather than
asserted.
