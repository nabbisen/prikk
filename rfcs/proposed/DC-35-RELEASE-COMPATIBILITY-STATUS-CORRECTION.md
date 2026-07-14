# RFC (proposed) - DC-35 Release Compatibility and Status Correction

**Status.** Proposed; architect design review required.
**Target milestone.** M1 - required before the 0.18.0 release candidate.
**Tracks.** TASK-13 and architect review N3.
**Touches.** Release/compatibility reference, implementation-status correction, README/ROADMAP links,
and release-state bookkeeping. Documentation only.

## Problem

Release rules and compatibility limits are distributed across README, ROADMAP, release reviews, Cargo
metadata, and current-state references. The implementation status also contradicts the released public
merge surface by omitting `merge-plan` and describing public merge evidence as absent.

## Design

Add `docs/src/reference/release-compatibility.md` as the public current-state policy. It must document:

- pre-1.0 semantic-version expectations and the absence of stable Rust API, CLI-output, object-schema,
  or repository-format guarantees unless a later RFC says otherwise;
- unprefixed Git tags such as `0.18.0`, while release archives use `prikk-v0.18.0.tar.gz`;
- the design, implementation review, release-candidate review, final release-state flip, tag, package,
  and publish sequence;
- the requirement that README, CHANGELOG, ROADMAP, RFC status/location, implementation status,
  workspace version, lockfile, and relevant mdBook pages already describe the release at publication;
- immutable release assets: a bad published artifact is superseded, never replaced under the same name;
- current manual gates and the distinction between observed evidence and policy.

Correct the merge-surface statements in `rfcs/IMPLEMENTATION-STATUS.md` against released 0.17.7
behavior. Link the new reference from relevant public and maintainer documentation without turning the
README into an internal development log.

## Non-goals

- No version bump, tag, package, publish, CI, code, schema, or CLI change.
- No compatibility promise, support window, LTS policy, migration tool, or 1.0 commitment.
- No claim that a listed gate passed unless observed for the release under review.

## Dependencies and gates

DC-35 may be implemented after design review independently of storage fixes, but it is held for the
single 0.18.0 corrective release. The final page must reflect DC-34's format and compatibility rulings.
`mdbook build docs` and link/status consistency are required implementation-review evidence.

## Acceptance criteria

The new reference is reviewed and navigable, the N3 contradictions are corrected, release assets and
state transitions are unambiguous, and all limitations remain explicit.
