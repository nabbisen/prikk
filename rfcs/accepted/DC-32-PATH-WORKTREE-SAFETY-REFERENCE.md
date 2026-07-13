# RFC (accepted) - DC-32 Path and Worktree Safety Reference

**Status.** Accepted after architect design review.
**Target release.** 0.17.6.
**Tracks.** TASK-11 path and worktree safety rules.
**Touches.** mdBook reference documentation, checkout/worktree guide cross-links, roadmap/status docs.
**Companion handoff.** None. This is a current-state safety reference and does not create a gating FDD.

## Context

DC-24 established the current data model and trust/threat references. DC-26 moved current-state
references into the published mdBook. DC-31 documented the physical `.prikk/` repository layout and
authority boundaries.

The next documentation gap is path and worktree safety. Users can hit path rejection errors through
snapshot manifests, patch replay, worktree status, worktree authoring, snapshot materialization, and
patch materialization. The rules are intentionally conservative, but they are scattered across code and
guide pages. DC-32 should give users and reviewers one current-state reference for what is accepted,
what is rejected, what worktree writes refuse, and which path/platform cases remain deferred.

DC-32 should close that gap without changing validators, checkout behavior, worktree authoring,
materialization behavior, object schema, repository format, or CLI behavior.

## Problem

1. **The validator is stricter than users may expect.** Current repository paths are ASCII-only,
   slash-separated, repository-relative strings. Non-ASCII paths, backslashes, colons, control
   characters, trailing spaces/dots, Windows reserved names, `.prikk/`, empty components, and dot
   components are rejected.
2. **Cross-platform safety rules are enforced even on Unix.** Windows reserved names and
   case-insensitive collisions are rejected today to avoid portable-history ambiguity.
3. **Write-safety behavior is spread across multiple pages.** Snapshot and patch materializers refuse
   symlinked parents/targets, conflicting existing files, non-file targets, and arbitrary deletion, but
   there is no single page that explains the shared boundary.
4. **Worktree authoring has its own conservative edge.** It skips `.prikk/`, rejects symlinks and
   non-regular files, rejects non-UTF-8 worktree paths, validates paths through `RepoPath`, and
   currently does not author symlink nodes.
5. **Deferred Unicode and platform coverage can be overread.** The current implementation rejects
   non-ASCII paths until Unicode NFC normalization is designed and tested. Full cross-platform path
   semantics remain an exercised-gate gap.

## Design Goals

1. Add a current-state reference page at `docs/src/reference/path-safety.md`.
2. Document the current `RepoPath` acceptance shape: non-empty, ASCII, repository-relative,
   slash-separated path text.
3. Document the current rejection set:
   absolute paths, backslashes, colon characters, non-ASCII paths, control characters, empty
   components, `.`/`..` components, first component `.prikk` case-insensitively, components ending in
   space or dot, Windows reserved component basenames, duplicate paths, and case-insensitive
   collisions.
4. Explain that cross-platform conservative rules are intentional and are enforced even on Unix.
5. Document snapshot manifest path safety: UTF-8 path bytes, `RepoPath` validation, sorted paths,
   duplicate/case-fold collision rejection, and content length framing.
6. Document snapshot materialization safety: opt-in only, validates snapshot manifests, refuses
   conflicting existing files, refuses symlinked parents and symlink targets, refuses non-file targets,
   and never removes extra worktree files.
7. Document patch materialization safety: uses the shared safe materializer for writes; deletion is
   opt-in, limited to explicit patch-deleted files, requires current bytes to match delete precondition
   bytes, refuses symlink targets and non-regular targets, and never removes arbitrary untracked files.
8. Document worktree authoring safety: `.prikk/` is skipped, symlink authoring is out of scope,
   non-regular entries are rejected, non-UTF-8 paths fail closed, paths are validated through
   `RepoPath`, file modes are normalized, and snapshot-only baselines are not node identity authority.
9. Include honest caveats for incomplete Unicode normalization, symlink authoring, full
   cross-platform path semantics, no Git path compatibility promise, no complete or atomic
   symlink-escape protection claim, no protection under concurrent worktree modification, and no stable
   repository-format or migration guarantee.
10. Cross-link the repository layout, data model, trust/threat, checkout, worktree status, worktree
    patch authoring, and materialization guide pages where appropriate.
11. Include visible claim-to-source anchors for every path and materialization safety claim.

## Non-goals

DC-32 does not add:

- code, schema, CLI behavior, repository behavior, checkout behavior, materialization behavior,
  worktree authoring behavior, trust behavior, verification behavior, or release semantics;
- Unicode NFC normalization;
- non-ASCII path support;
- symlink authoring or symlink materialization;
- Git path compatibility;
- platform-specific path policy negotiation;
- case-sensitive/case-insensitive mode switches;
- new diagnostics, error-code taxonomy, or machine-readable output;
- stable repository-format or migration guarantees;
- a new current-state FDD under `rfcs/fdds/`.

## Proposed Documentation Shape

Create:

```text
docs/src/reference/path-safety.md
```

Add it under the mdBook `# Reference` section near the repository layout reference:

```md
- [Path and Worktree Safety](reference/path-safety.md)
```

The page should be a current-state reference. It should describe current behavior and current gaps, not
future path policy.

### Required Sections

The implemented page should contain at least:

1. **Core Caveats.** Early implementation, conservative path subset, ASCII-only, no Unicode NFC
   normalization yet, symlink authoring deferred, Linux-only exercised gates, and no stable path-format
   or migration guarantee.
2. **Repository Path Shape.** Current `RepoPath` representation and acceptance shape.
3. **Rejected Path Forms.** The full validator rejection set, including cross-platform reserved names
   and case-insensitive collisions.
4. **Snapshot Manifest Paths.** UTF-8 decode, `RepoPath` parsing, sorted entries, duplicate/collision
   rejection, and manifest framing.
5. **Materialization Safety.** Snapshot and patch write rules, symlink parent/target refusal,
   conflicting-file refusal, non-file refusal, and no arbitrary deletion.
6. **Deletion Safety.** Explicit patch-deletion path, old-byte precondition check, symlink/non-file
   refusal, and already-absent handling.
7. **Worktree Authoring Safety.** `.prikk/` skip, symlink/non-regular rejection, non-UTF-8 failure,
   `RepoPath` validation, normalized file mode, and snapshot-only baseline rejection.
8. **Deferred and Not Promised.** Unicode normalization, symlink support, platform matrix, Git path
   compatibility, stable repository-format migration, and path-policy configuration remain deferred.
9. **Claim-to-Source Anchors.** Code/docs/RFC anchors for path validation, snapshot manifests,
   materialization, patch deletion, worktree authoring, and caveats.

### Write-Safety Precision

The materialization section must describe the current mechanism accurately:

- `RepoPath` lexically forbids absolute paths, traversal, backslashes, and other rejected path forms;
- materialization joins the validated repository path to the repository root and checks lexical
  root-containment;
- each existing parent directory is checked with symlink-aware metadata and symlink parents are
  refused;
- the final target is checked with symlink-aware metadata when it already exists, and symlink or
  non-file targets are refused;
- writes use the current atomic file-write helper.

The page must also state what this does not prove. Current materialization is check-then-write; it is
not an `openat`/`O_NOFOLLOW` design, the containment check is lexical rather than canonical realpath
proof, and the docs must not promise race-free protection against concurrent worktree modification or a
concurrent process swapping an ancestor for a symlink between checks and writes.

### Implementation Review Guards

Implementation review must also verify:

1. `.prikk` is documented as rejected only when it is the first component, case-insensitively. A
   non-leading `.prikk` component must not be described as rejected by the `RepoPath` validator.
2. Control-character rejection is documented as bytes `0x00` through `0x1F` and `0x7F`, after the
   ASCII-only gate.
3. Windows reserved component matching is documented as exactly `CON`, `PRN`, `AUX`, `NUL`, `COM1`
   through `COM9`, and `LPT1` through `LPT9`, matched on the component basename before the first dot.
4. Claim-to-source anchors split validator, manifest, snapshot materializer, patch materialization/
   deletion, worktree authoring, and atomic-write claims to their owning files.

## Required Source Audit

The implementation must check the final page against:

- `crates/prikk-replay/src/path.rs`
- `crates/prikk-replay/src/path/tests.rs`
- `crates/prikk-store/src/path.rs`
- `crates/prikk-store/src/snapshot.rs`
- `crates/prikk-store/src/worktree.rs`
- `crates/prikk-store/src/patch_checkout.rs`
- `crates/prikk-store/src/patch_replay/decode.rs`
- `crates/prikk-store/src/worktree_patch/node_authoring.rs`
- `crates/prikk-store/src/fsutil.rs`
- `crates/prikk-store/src/snapshot/tests.rs`
- `crates/prikk-store/src/worktree_patch/tests.rs`
- `docs/src/guide/checkout/snapshot-checkout.md`
- `docs/src/guide/checkout/snapshot-materialization.md`
- `docs/src/guide/patches/worktree-patch.md`
- `docs/src/guide/patches/patch-materialization.md`
- `docs/src/guide/patches/patch-deletions.md`
- `docs/src/guide/worktree-status.md`
- `docs/src/reference/repository-layout.md`
- `docs/src/reference/trust-threat-model.md`

## Review Requirements

Architect review should verify:

1. The proposed scope is documentation-only and does not imply validator, checkout, or worktree
   behavior changes.
2. The rejection set matches current `RepoPath` code exactly and does not omit `.prikk/`, trailing
   space/dot, colon, backslash, or non-ASCII behavior.
3. The design honestly states that non-ASCII/NFC normalization and symlink authoring remain deferred.
4. The write-safety description states the current lstat/lexical-containment/check-then-write
   mechanism and does not overclaim complete, atomic, race-free symlink escape protection beyond
   current code paths and platform evidence.
5. The page plan correctly separates read-only planning, opt-in writes, and opt-in explicit deletion.
6. The required source audit is sufficient for implementation review.

## Acceptance Criteria

DC-32 is ready for implementation only after architect design review accepts this RFC or accepts a
repaired version. Implementation is complete when:

- the reference page exists and is linked in mdBook navigation;
- relevant current guide/reference pages link to it without duplicating the full page;
- claim-to-source anchors are included;
- `ROADMAP.md`, `rfcs/README.md`, and `rfcs/IMPLEMENTATION-STATUS.md` are updated consistently;
- documentation build/check commands pass in the implementing thread.
