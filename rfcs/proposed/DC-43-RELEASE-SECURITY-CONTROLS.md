# RFC (proposed) - DC-43 Release Security and Distribution Controls

**Status.** Proposed; security/architect design review required.

**Status update, 2026-08-27 (evidenced, not a ruling — the schedule position below is stale; the
public-preview prerequisite is unaffected and stands as originally written).** DC-42, the cited
predecessor, does not exist as live work: it was superseded 2026-07-29 into DC-56, DC-57, and
DC-58 (`rfcs/archive/DC-42-PERFORMANCE-MAINTAINABILITY-GATES.md`), and all three have since been
implemented — DC-56 at `8748f00`, DC-57 at `caa2fc2` (accepted 2026-08-02), DC-58 at `6f53da3`
(accepted 2026-07-31; `rfcs/EXECUTION-ORDER.md` lines 121, 127, 189). **There is no longer a
predecessor for this RFC to sit behind.** The record does not support naming a specific successor
in DC-42's place either — nothing establishes that DC-43 should now wait on any one of the three,
or on anything else. The schedule position is unknown, not merely outdated, and should be read
that way rather than corrected to a guess.

**Target milestone.** M2 - required before reconsidering public-preview readiness.
**Schedule position.** ~~Third remaining post-M1 increment, after DC-42.~~ **Stale — see the status
update above.** Completion remains a prerequisite for any public-preview reconsideration; this
program order is not implementation authority.
**Tracks.** Architect review N7.
**Touches.** Vulnerability reporting, dependency policy, SBOM/provenance, release attestations, release-
key lifecycle, registry-owner lifecycle, CI platform policy, and public release documentation.

## Design goals

- Add a tracked `SECURITY.md` with supported-version and private vulnerability-reporting guidance that
  does not expose or request secrets in public issues.
- Select and configure a dependency/advisory policy with explicit allowed licenses, duplicate/version
  handling, and failure ownership.
- Generate an SBOM for release artifacts and record artifact digests without changing an already
  published asset.
- Define provenance/attestation generation and verification for source archives and published crates.
- Define mature release-key custody/backup, scheduled rotation, expiration/revocation monitoring,
  hardware-backed-key policy, scalable maintainer quorum, and registry-owner lifecycle controls without
  weakening DC-35's minimum non-deadlocking bootstrap/recovery authority.
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
- No removal of DC-35's M1 multi-signer capability, two-person authority review, break-glass recovery,
  or official-upstream/community boundary; DC-43 may strengthen those controls.
- No mutation or replacement of released tags, crates, archives, checksums, or attestations.

## Acceptance criteria

Security reporting is usable, dependency policy runs reproducibly, release artifacts have reviewable
SBOM/digest/provenance evidence, release-key and registry-owner lifecycle controls are recoverable and
reviewed, workflow permissions are least-privilege, and independent review finds no unsupported
readiness claim.
