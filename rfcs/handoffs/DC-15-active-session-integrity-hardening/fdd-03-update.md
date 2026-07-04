# DC-15 FDD-03 Update - Ref Publication and Signature Preimage Boundaries

Status: Revised for v0.8.0 design re-review after architect review v1
Related RFC: `../../done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md`
Target FDD: FDD-03 Object Schema and Canonical Identity

## Purpose

DC-15 does not add object types, canonical tags, operation records, or payload fields. It tightens
production validation around existing identity-bearing values: branch ref names used for publication and
key ids used in role-bound signature preimages.

## Required FDD-03 Body Updates

### Ref Publication Boundary

Production branch publication must validate `RefPublication.ref_name` with the shared local branch-ref
validator before writing objects, appending ref logs, or promoting pointers.

This validation is a production publication boundary. It does not change canonical decoding of historical
`RefState` or `RefUpdate` payloads unless a later schema design explicitly makes invalid historical refs
unreadable.

No rollback, tag, remote, symbolic, or malformed ref namespace is introduced by DC-15.

### Key-Id Validation and Signature Preimage Construction

Production signing key ids must be validated on the shared role-bound signature preimage path:

- non-empty;
- no NUL or control characters;
- no path separators or traversal-like components;
- length fits the canonical preimage length field without truncation;
- same policy for AUTHOR and MAINTAINER signing where practical.

Signature preimage construction must be fallible and must not silently truncate key-id length metadata.
The guard belongs on the shared preimage function because both signing and verification reconstruct that
preimage. Signer-only validation is insufficient.

## Required Tests

- `RefStore::publish` rejects invalid branch refs before object writes and log appends.
- Historical `RefState` / `RefUpdate` decoding compatibility remains unchanged.
- AUTHOR signer construction rejects invalid and overlong key ids.
- MAINTAINER signer construction rejects invalid and overlong key ids.
- Signature preimage construction returns an error for invalid or overlong key ids.
- Verification paths that reconstruct a signature preimage also reject invalid or overlong key ids.
