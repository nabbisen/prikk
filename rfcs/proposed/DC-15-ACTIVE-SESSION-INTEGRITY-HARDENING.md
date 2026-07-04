# RFC (proposed) - DC-15 Active-Session Integrity and Verification Hardening

**Status.** Revised for design re-review after architect review v1.
**Target release.** v0.8.0.
**Tracks.** Closing accepted non-blocking hardening backlog before broad M2+ patch algebra.
**Touches.** Active-WAL metadata verification/doctor diagnostics, rollback draft append freshness,
lower-level ref publication boundaries, signature key-id/preimage validation, and legacy placeholder
wording.
**Companion FDD updates.** `../handoffs/DC-15-active-session-integrity-hardening/fdd-02-update.md`,
`../handoffs/DC-15-active-session-integrity-hardening/fdd-03-update.md`,
`../handoffs/DC-15-active-session-integrity-hardening/fdd-04-update.md`.

## Context

v0.7.0 completed the arbitrary-span text direct-inverse bridge and the focused P1 repair pass. The final
repair review accepted the release but left several non-blocking hardening notes visible:

- active-WAL ref metadata is command-critical but not reported by repository `verify`;
- `append_rollback_draft` derives inverse/preview before it holds the active lock;
- the lower-level ref publication API has only minimal ref-name validation;
- signing key-id and signature preimage length handling is looser than trust-policy key-id validation;
- documentation still needs clearer wording around legacy placeholder names and rejection guards.

DC-15 is a narrow foundation-hardening increment. It intentionally does not start M2+ patch algebra.

Architect review v1 accepted the design direction with one required erratum and three open-question
rulings folded into this revision:

- key-id and length guards must live on the shared signature preimage construction path used by both
  signing and verification;
- empty-WAL metadata debris is reported by both `verify` and `doctor` as warning/local-debris state;
- idempotent seal retry cleanup remains local cleanup without maintainer re-check, but only after the
  already-published transition is verified;
- `Signature::signed_bytes` becomes fallible rather than relying on a signer-only wrapper.

## Design Goals

1. Make active-WAL ref metadata integrity visible to `verify` and `doctor`.
2. Close the rollback-draft stale-target append window without adding rollback authorization.
3. Clarify and enforce the lower-level ref publication boundary for branch refs.
4. Tighten key-id/preimage length validation for production signing paths and signature preimage helpers.
5. Clean legacy placeholder wording so docs distinguish "rejected compatibility artifact" from
   "accepted authority".
6. Preserve all v0.7.0 object schemas and release boundaries.

## Non-goals

DC-15 does not add:

- rollback refs;
- rollback authorization or audit approval;
- AUTHOR trust-store enforcement;
- branch switching, branch copy/fork, merge-base semantics, branch deletion/rename, tags, or remote refs;
- queued multi-commit active sessions or per-ref active WALs;
- commutation, confluence, conflict witnesses, semantic merge, or worktree rollback mutation;
- key rotation, revocation, expiration, threshold policy, hardware signing, or remote trust.

## Proposed Design

### Active-WAL metadata verification

Repository verification should classify active-session metadata state alongside active-WAL replay:

- empty WAL plus missing metadata is healthy;
- empty WAL plus valid, malformed, or mismatched metadata is local debris, not structural corruption;
- non-empty WAL plus valid metadata is healthy active-session state;
- non-empty WAL plus missing metadata is an active-session integrity issue;
- non-empty WAL plus malformed metadata is an active-session integrity issue;
- non-empty WAL plus metadata that fails the local branch-ref validator is an active-session integrity
  issue.

`verify` must expose these as explicit report counters or issue details instead of silently ignoring
metadata. Non-empty-WAL metadata issues are integrity issues. Empty-WAL metadata debris is a distinct
warning-class field, visible but not conflated with sealed-history corruption. `doctor` should translate
both classes into actionable diagnostics. DC-15 should not repair non-empty WAL metadata problems
automatically; clearing or reconstructing active sessions needs a separate recovery policy.

Empty-WAL metadata debris may remain cleaned by command paths under the active lock. If `doctor` reports
empty-WAL debris, it may classify it as cleanup-eligible only when no WAL records and no trailing partial
bytes exist.

Idempotent seal retry cleanup remains local cleanup, not a new publication authority decision. It must
not require a new maintainer trust check, but it must first verify that the requested ref already advanced
to the expected published state represented by the active WAL patch IDs. If that already-published
transition cannot be proven, seal must fail closed rather than drain.

### Rollback draft append freshness

`append_rollback_draft` currently derives inverse/preview before acquiring the active lock and then
appends if the active WAL is empty. DC-15 must use the compare shape accepted by architect review:

1. read the target ref state/block identity used for inverse planning;
2. derive inverse/preview without holding the active lock;
3. acquire the active lock before the final empty-WAL check and append;
4. re-read the target ref state/block identity under the active lock using a path that does not acquire a
   ref-specific lock or otherwise invert the seal lock order;
5. fail closed if the target changed between planning and append;
6. write active-WAL ref metadata and append the rollback draft under the same existing lock discipline.

Moving the entire inverse derivation under the active lock is rejected for DC-15 implementation unless a
later lock-ordering review proves it cannot invert with seal. The compare shape keeps expensive sealed
history reads lock-free; sealed history is immutable, so an unchanged tip identity proves the derived
inverse is still fresh.

This does not make `seal` enforce rollback-draft verification or authorization. That remains a separate
rollback policy design.

### Ref publication boundary

DC-13 intentionally kept object decoding compatibility-aware, but production branch publication should
not depend on CLI-only validation. DC-15 should make the lower-level boundary explicit:

- production branch publication must validate `RefPublication.ref_name` with the shared local branch
  validator before it writes objects or ref logs;
- compatibility-oriented decoding of historical `RefState` / `RefUpdate` payloads remains unchanged;
- if an internal non-branch publication escape hatch is needed later, it must be explicitly named and not
  used by current branch CLI paths.

This preserves the v0.7.0 branch-only production surface and avoids accidentally publishing tags,
remotes, rollback refs, or malformed branch names through a lower-level caller.

### Key-id and signature preimage validation

DC-15 should align production signing key-id handling with the existing trust-policy key-id discipline:

- `Ed25519AuthorSigner` and `Ed25519MaintainerSigner` reject empty, control-character, path-like,
  traversal-like, or overlong key ids before signing;
- maintainer trust policy and signing boundaries use the same key-id policy where practical;
- `Signature::signed_bytes` must be fallible and must not silently truncate key-id length metadata through
  unchecked `u16` casts.

Preferred implementation shape:

- add a shared key-id validation helper in the object or store layer;
- make signature preimage construction return `Result<Vec<u8>>`;
- update callers and tests to handle the result explicitly.

The guard belongs on the shared preimage function because verification also reconstructs the signature
preimage from signature records. Signer-only validation is insufficient: it leaves verification paths
able to reconstruct preimages from unchecked key ids.

### Legacy placeholder wording

Documentation should distinguish three cases:

- old placeholder signatures or key ids from development-era artifacts;
- rejection guards that detect and reject those placeholders;
- accepted production authority.

Production documentation should not imply that legacy placeholder strings are still valid rollback or
publication authority. It is acceptable for production code to contain constants used solely to reject
legacy marker values.

## Implementation Outline

1. Add active-WAL metadata status representation to verification results without changing object schema.
2. Extend `verify` CLI output and doctor diagnostics to report non-empty-WAL metadata issues.
3. Add rollback-draft append freshness check against the target ref state/block id under the active lock.
4. Move branch-ref validation into the production `RefStore::publish` boundary, preserving historical
   object decoding compatibility.
5. Add or reuse a shared signing key-id validator and prevent unchecked preimage length truncation.
6. Sweep release docs and RFC/FDD handoff wording for legacy placeholder authority ambiguity.
7. Keep M2+ patch algebra and rollback policy out of this implementation.

## Required Tests

- `verify` reports non-empty active WAL with missing ref metadata.
- `verify` reports non-empty active WAL with malformed ref metadata.
- `doctor` reports actionable active-session metadata issues for the same cases.
- `verify` and `doctor` report empty WAL plus stale or malformed metadata as warning/local-debris state,
  not sealed-history corruption.
- Rollback draft append fails if the target ref changes between inverse planning and append.
- Rollback draft append still succeeds for unchanged target ref and empty active WAL.
- Rollback draft freshness re-read under the active lock does not acquire a ref-specific lock or invert
  the seal lock order.
- `RefStore::publish` rejects invalid branch refs before object writes or log appends.
- Historical object decoding tests continue to accept compatibility-aware `RefState` / `RefUpdate`
  payloads as currently allowed.
- AUTHOR and MAINTAINER signer constructors reject invalid or overlong key ids.
- Signature preimage construction is fallible and rejects invalid or overlong key ids on both signing and
  verification paths.
- Idempotent seal retry cleanup drains only after proving the expected transition is already published and
  does not require a fresh maintainer trust check.
- Legacy placeholder docs and code comments state "rejection guard" rather than "accepted authority".

## Compatibility

No object schema migration is required. DC-15 changes validation, diagnostics, and command behavior around
mutable local session state and signing inputs. Existing valid repositories remain valid.

Repositories with non-empty active WALs and missing or malformed active ref metadata were already
unsealable under v0.7.0 command paths. DC-15 makes that condition visible to `verify` / `doctor`; it does
not reinterpret it as valid history.

## Review Rulings

Architect review v1 resolved the previously open questions:

1. Empty-WAL metadata debris is visible in both `verify` and `doctor`, as warning/local-debris state.
2. Idempotent seal retry cleanup remains local cleanup without a maintainer re-check, but only after the
   already-published transition is proven.
3. `Signature::signed_bytes` becomes fallible because the shared path is used by signing and verification.

It also recommended sequencing the independently applicable hardening items early if maintaining an older
line. In the current post-0.7.0 line, DC-15 keeps the full set together for v0.8.0; the signing preimage
guard and rollback freshness work remain implementation-priority items inside that increment.

## Rejected Alternatives

### Start M2+ patch algebra now

Rejected for DC-15. The remaining review backlog affects active-session integrity, publication
boundaries, and signing input validation. These are safer to harden before commutation and conflict
witnesses depend on them.

### Add rollback authorization as part of stale-draft prevention

Rejected. A freshness check prevents appending a draft derived from a stale ref tip. Authorization,
approval, and rollback-specific publication policy require a separate design.

### Treat active-WAL metadata issues as object-store corruption

Rejected. Active-WAL metadata is local mutable session state. It should be visible and actionable, but it
is not content-addressed sealed history.
