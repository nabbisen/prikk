# RFC (proposed) - DC-43 Release Security and Distribution Controls

**Status.** Proposed; security/architect design review required.
**Target milestone.** M2 - required before reconsidering public-preview readiness.
**Tracks.** Architect review N7.
**Touches.** Vulnerability reporting, dependency policy, SBOM/provenance, release attestations, CI
platform policy, and public release documentation.

## Design goals

- Add a tracked `SECURITY.md` with supported-version and private vulnerability-reporting guidance that
  does not expose or request secrets in public issues.
- Select and configure a dependency/advisory policy with explicit allowed licenses, duplicate/version
  handling, and failure ownership.
- Generate an SBOM for release artifacts and record artifact digests without changing an already
  published asset.
- Define provenance/attestation generation and verification for source archives and published crates.
- Make required release gates, optional evidence jobs, and failure handling visible in tracked policy.
- Preserve the project's explicit experimental/no-production warning until an independent review says
  otherwise.

Tool and hosted-service choices require design review before workflow edits. Actions must be pinned to
reviewed immutable revisions according to existing CI policy. Publishing credentials remain external
secrets and must never be copied into RFCs, logs, fixtures, or review packages.

## Non-goals

- No guarantee of vulnerability-free dependencies, paid support SLA, bug bounty, automatic emergency
  release, or production-readiness claim.
- No key-management redesign for repository AUTHOR/MAINTAINER signatures.
- No mutation or replacement of released tags, crates, archives, checksums, or attestations.

## Acceptance criteria

Security reporting is usable, dependency policy runs reproducibly, release artifacts have reviewable
SBOM/digest/provenance evidence, workflow permissions are least-privilege, and independent review finds
no unsupported readiness claim.
