# RFC (proposed) - DC-41 Integrity Evidence Campaign

**Status.** Proposed; implementation blocked on DC-36 through DC-40.
**Target milestone.** M2 - post-correction assurance milestone.
**Tracks.** Architect review N4, N6, and missing-evidence items.
**Touches.** Failpoint/property/fuzz test infrastructure, hash differential evidence, platform matrix,
verification claims, and evidence records. Production behavior changes require separate RFCs.

## Design

Build repeatable evidence around the corrected contracts:

- run the DC-38 crash matrix through reusable deterministic failpoints;
- add property/fuzz targets for canonical decoders, WAL/ref-log framing, replay, and bounded patch
  algebra inputs, with a committed corpus and reproducible seeds for failures;
- add SHA-256 vectors at 55/56/63/64-byte boundaries, multi-block vectors, and randomized differential
  comparison against an audited development dependency;
- exercise supported gates on Linux plus explicit macOS and Windows CI jobs where behavior is portable;
- record filesystem/platform limitations rather than treating CI presence as durability proof;
- update public evidence claims only from observed, reproducible results.

Fuzzers and differential dependencies must remain development-only and must not enter object identity or
runtime trust paths. A discovered behavior defect opens a dedicated corrective RFC instead of being
silently normalized into a test expectation.

## Non-goals

- No formal proof of crash safety, certification, production-readiness claim, or all-filesystem claim.
- No replacement of the first-party SHA-256 implementation in this RFC.
- No merge-scope expansion or random mutation of real user repositories.

## Acceptance criteria

The evidence commands are documented and reproducible, CI separates required gates from optional
platform evidence, hash comparison has no unexplained mismatch, and an adversarial review receives the
matrix and failure corpus rather than only aggregate pass counts.
