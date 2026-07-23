# RFC (accepted) - DC-40 State Merkle Root and Format Transition

**Status.** Accepted with companion FDD after architect re-review on 2026-07-14; implementation
complete at `70c3902` after architect post-commit evidence acceptance on 2026-07-23. Remains accepted
until the 0.18.0 release.
**Target milestone.** M1 - 0.18.0 corrective release.
**Tracks.** Architect review B2.
**Touches.** Canonical clean-tree model, Block schema, repository format gate, seal, replay/verify,
golden vectors, compatibility behavior, and migration documentation.
**Companion FDD.**
`../handoffs/DC-40-state-merkle-format-transition/state-root-format-fdd.md` is required byte and
command compatibility authority. It follows this RFC's lifecycle and must be accepted before coding.

## Problem

The released Block `state_merkle_root` hashes Patch ids under a scaffold domain. It does not commit to
paths, modes, node kinds, file bytes, or symlink targets, and verification does not recompute it.

## Proposed format decision

The companion FDD chooses a flat canonical live-entry set with a binary Merkle reduction. Implicit
directories have no identity. Entries are ordered by canonical repository-path bytes and commit to path,
node id, node kind, normalized mode, and content identity. Files commit to their Blob ObjectId;
symlinks commit to opaque schema-1 UTF-8 target bytes and mode zero. Patch ids are excluded. Different Patch
histories with the same complete live entries produce the same root; changing a node id changes the
state and therefore the root.

New repositories use repository format 2 and new seals write Block envelope schema 2 with the real
root. A Root Block v2 has zero parents; a Normal Block v2 has exactly one Block-v2 parent. Merge,
Repair, and Import Block kinds are rejected in format 2 until a later accepted RFC defines their state
derivation. A Block v2 may not have a Block v1 parent; the first Block in a format-2 repository is a v2
Root. Verification derives every accepted Block state from its singular parent plus ordered Patches and
compares the recomputed root. Snapshot/cache data may accelerate this only after validation; it is not
root authority.

Format-1 repositories are opened in bounded legacy read-only mode. They can be inspected and planned
against as listed in the FDD, but repository mutation, repair, and worktree materialization are refused.
`verify` reports that schema-1 scaffold roots are not state commitments and returns non-zero. 0.18.0
does not provide history-preserving migration: a user obtains a writable format-2 repository by
initializing a new repository and re-authoring the desired worktree state, with new history and
identities. The sole mutation exception is DC-34's exact signer-backed completion of a released
format-1 one-record-ahead interrupted publication; it promotes the already-signed RefState and performs
active-state cleanup without rewriting identity bytes or appending a log record. The implementation
must not otherwise reinterpret or rewrite old Blocks, signatures, or ObjectIds.

## Required tests

- literal cross-implementation vectors for every node kind and tree shape;
- equivalent clean states from different Patch identities produce the same root;
- path, mode, kind, content, node-id, and symlink-target changes alter the root;
- every newly sealed Block is recomputed by repository verification;
- format-1 repositories are never silently written as format 2 or misreported as verified state roots;
- cache/snapshot disagreement cannot override authoritative replay.

## Non-goals

- No in-place identity mutation, transparent migration promise, general GC, remote compatibility, or
  stable repository-format claim.
- No broader patch algebra or merge execution.

## Acceptance criteria

Architect re-review accepted the byte grammar and compatibility matrix on 2026-07-14. Implementation
completion still requires passing golden vectors, root-mismatch verification, format-command tests,
and release documentation that clearly identifies the format transition and migration limit. The
companion FDD's status follows DC-40 under RFC-000.
