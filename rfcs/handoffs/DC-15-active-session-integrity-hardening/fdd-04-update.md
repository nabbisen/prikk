# DC-15 FDD-04 Update - Active-Session and Legacy Placeholder Threat Notes

Status: Revised for v0.8.0 design re-review after architect review v1
Related RFC: `../../done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md`
Target FDD: FDD-04 Threat Model

## Purpose

DC-15 clarifies the threat boundary around mutable active-session state, production signing inputs, and
legacy placeholder artifacts. It is a hardening increment, not a new trust or authorization system.

## Required FDD-04 Body Updates

### Active-Session Metadata Tampering

Missing, malformed, or invalid active-WAL ref metadata with a non-empty active WAL is an integrity issue
because the repository cannot safely determine which ref owns the queued patch records. Commit and seal
paths must fail closed, and `verify` / `doctor` must report the condition.

The condition is local mutable-session state, not content-addressed object corruption. Diagnostics must
avoid implying sealed history was rewritten unless object/ref verification also reports corruption.
Empty-WAL metadata debris is warning/local-debris state: it must be visible, but it must not be presented
as sealed-history corruption.

### Rollback Draft Freshness

A rollback draft append must not claim success if the target ref changed after inverse planning but
before append. DC-15's freshness check closes that race by deriving lock-free, acquiring the active lock,
then re-reading and comparing the ref tip without inverting seal's lock order. It does not prove rollback
authorization, does not require seal-time rollback approval, and does not introduce rollback refs.

### Seal Retry Cleanup

Draining active WAL/ref metadata after an already-published seal transition is local cleanup, not a new
authority decision. It must not require fresh maintainer trust validation, but it must prove the expected
transition is already published before draining. If that proof fails, cleanup must fail closed.

### Signing Input Validation

Key ids are part of the role-bound signature preimage. Production signing paths must reject key ids that
could be ambiguous, unsafe as local identifiers, or too long for the canonical preimage length field.
Signature preimage construction must be fallible and must not silently truncate length metadata. The same
guard must protect signing and verification preimage reconstruction.

### Legacy Placeholder Wording

Legacy placeholder key ids or signature bytes may appear in production code only as rejection guards or
compatibility diagnostics. They must not be described as accepted authority in release notes, user docs,
or FDD security text.

## Required Security Tests

- non-empty active WAL with missing metadata is reported and blocks safe publication;
- non-empty active WAL with malformed or invalid metadata is reported and blocks safe publication;
- empty-WAL metadata debris is visible as warning/local-debris state;
- stale rollback draft append fails when the selected ref advances before append;
- rollback freshness re-read does not invert seal lock ordering;
- seal retry cleanup drains only after proving an already-published transition and without requiring a
  fresh maintainer trust check;
- invalid signing key ids are rejected before signature preimage construction;
- overlong key ids fail shared preimage construction on signing and verification paths;
- docs and comments distinguish rejected legacy placeholders from accepted production authority.
