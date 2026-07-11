# Prikk RFCs

This directory stores Prikk design and implementation decision records.

The lifecycle policy is tracked as [RFC-000](./done/000-rfc-lifecycle-policy.md). RFC-000 is the
authority for what the `proposed/`, `accepted/`, `done/`, `archive/`, optional `draft/`, and
`handoffs/` directories mean.

## Lifecycle Summary

Prikk uses RFC-000's 5-folder variant:

- `proposed/` contains RFCs under design review; implementation should not start from these records.
- `accepted/` contains reviewed designs that may be implemented but have not yet released.
- `done/` contains implemented/released RFC records.
- `archive/` contains withdrawn, superseded, or historical umbrella RFCs that are no longer live
  implementation authority.
- `draft/` may be added later if shared pre-review drafts become useful.
- `handoffs/` contains companion execution/FDD handoff material. Handoffs do not define an independent
  lifecycle; their state follows the related RFC.

RFC-000 says folder location is lifecycle authority. The status text inside each RFC should be kept
consistent with its folder.

## Proposed

No proposed RFCs are currently tracked.

## Accepted

These records currently live under `accepted/`.

| ID | Title |
|---|---|
| DC-25 | [Merge Planning Surface](./accepted/DC-25-MERGE-PLANNING-SURFACE.md) |
| DC-26 | [Documentation Home Correction](./accepted/DC-26-DOCUMENTATION-HOME-CORRECTION.md) |

## Done

These records currently live under `done/`.

| ID | Title |
|---|---|
| RFC-000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) |
| DC-10 | [Rollback Draft Identity and AUTHOR Signing](./done/DC-10-ROLLBACK-DRAFT-SIGNING.md) |
| DC-11 | [Publication Signing and Minimal Trust Store](./done/DC-11-MAINTAINER-TRUST-STORE.md) |
| DC-12 | [Arbitrary-Span Text Edits](./done/DC-12-ARBITRARY-SPAN-TEXT-EDITS.md) |
| DC-13 | [Non-Default Ref Genesis](./done/DC-13-NONDEFAULT-REF-GENESIS.md) |
| DC-14 | [Arbitrary-Span Text Direct Inverse and Rollback Exposure](./done/DC-14-ARBITRARY-SPAN-TEXT-INVERSE-ROLLBACK.md) |
| DC-15 | [Active-Session Integrity and Verification Hardening](./done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md) |
| DC-16 | [Patch Algebra Foundation](./done/DC-16-PATCH-ALGEBRA-FOUNDATION.md) |
| DC-17 | [Patch Algebra Evidence Contract](./done/DC-17-PATCH-ALGEBRA-EVIDENCE-CONTRACT.md) |
| DC-18 | [Patch Algebra Commutation and Confluence Contract](./done/DC-18-PATCH-ALGEBRA-COMMUTATION-CONFLUENCE.md) |
| DC-19 | [Replay/Lifecycle Crate Boundary and Extraction Plan](./done/DC-19-REPLAY-LIFECYCLE-CRATE-BOUNDARY.md) |
| DC-20 | [Replay Boundary Stabilization](./done/DC-20-REPLAY-BOUNDARY-STABILIZATION.md) |
| DC-21 | [Merge Conflict Evidence Contract](./done/DC-21-MERGE-CONFLICT-EVIDENCE-CONTRACT.md) |
| DC-22 | [Public Merge Evidence UX Boundary](./done/DC-22-PUBLIC-MERGE-EVIDENCE-UX.md) |
| DC-23 | [Public Merge Evidence UX Stabilization](./done/DC-23-MERGE-EVIDENCE-UX-STABILIZATION.md) |
| DC-24 | [Data Model and Trust/Threat Documentation](./done/DC-24-DATA-MODEL-TRUST-THREAT-DOCS.md) |
| PR-001 | [Implementation Handoff](./done/PR-001-IMPLEMENTATION-HANDOFF.md) |
| PR-002 | [CI Fix Handoff](./done/PR-002-CI-FIX-HANDOFF.md) |
| PR-003 | [Persistent Store Handoff](./done/PR-003-PERSISTENT-STORE-HANDOFF.md) |
| PR-004 | [WAL Handoff](./done/PR-004-WAL-HANDOFF.md) |
| PR-005 | [CI Fix Handoff](./done/PR-005-CI-FIX-HANDOFF.md) |
| PR-006 | [Verification Handoff](./done/PR-006-VERIFY-HANDOFF.md) |
| PR-007 | [Ref Publication Handoff](./done/PR-007-REF-PUBLICATION-HANDOFF.md) |
| PR-008 | [Commit Scaffold Handoff](./done/PR-008-COMMIT-SCAFFOLD-HANDOFF.md) |
| PR-009 | [Seal Scaffold Handoff](./done/PR-009-SEAL-SCAFFOLD-HANDOFF.md) |
| PR-010 | [Verify Hardening Handoff](./done/PR-010-VERIFY-HARDENING-HANDOFF.md) |
| PR-011 | [Doctor Diagnostics Handoff](./done/PR-011-DOCTOR-HANDOFF.md) |
| PR-012 | [Doctor Repair Handoff](./done/PR-012-DOCTOR-REPAIR-HANDOFF.md) |
| PR-013 | [Ref Recovery Handoff](./done/PR-013-REF-RECOVERY-HANDOFF.md) |
| PR-014 | [History Inspection Handoff](./done/PR-014-HISTORY-HANDOFF.md) |
| PR-015 | [Checkout Plan Handoff](./done/PR-015-CHECKOUT-PLAN-HANDOFF.md) |
| PR-016 | [Snapshot Path-Safety Handoff](./done/PR-016-SNAPSHOT-PATH-SAFETY-HANDOFF.md) |
| PR-017 | [Snapshot Materialization Handoff](./done/PR-017-SNAPSHOT-MATERIALIZATION-HANDOFF.md) |
| PR-018 | [Worktree Status Handoff](./done/PR-018-WORKTREE-STATUS-HANDOFF.md) |
| PR-019 | [Worktree Patch Draft Handoff](./done/PR-019-WORKTREE-PATCH-HANDOFF.md) |
| PR-020 | [Patch Replay Handoff](./done/PR-020-PATCH-REPLAY-HANDOFF.md) |
| PR-021 | [Patch Materialization Handoff](./done/PR-021-PATCH-MATERIALIZATION-HANDOFF.md) |
| PR-022 | [Patch Deletion Handoff](./done/PR-022-PATCH-DELETION-HANDOFF.md) |
| PR-023 | [Text Anchor Scaffold Handoff](./done/PR-023-TEXT-ANCHOR-HANDOFF.md) |
| PR-024 | [Conservative Text Replay Handoff](./done/PR-024-TEXT-REPLAY-HANDOFF.md) |
| PR-025 | [Opt-In Full-File Text Edit Generation Handoff](./done/PR-025-TEXT-GENERATION-HANDOFF.md) |
| PR-026 | [Supported Patch Inverse Planning Handoff](./done/PR-026-INVERSE-PLAN-HANDOFF.md) |
| PR-027 | [Non-Mutating Rollback Preview Handoff](./done/PR-027-ROLLBACK-PREVIEW-HANDOFF.md) |
| PR-028 | [Rollback Draft Handoff](./done/PR-028-ROLLBACK-DRAFT-HANDOFF.md) |
| PR-029 | [Rollback Draft Verification Handoff](./done/PR-029-ROLLBACK-DRAFT-VERIFY-HANDOFF.md) |
| PR-030 | [Sealed Rollback History Classification Handoff](./done/PR-030-SEALED-ROLLBACK-HISTORY-HANDOFF.md) |

`PR-*` files are legacy implementation handoff records retained as historical shipped records. New
design-change records use `DC-*` RFCs plus optional `rfcs/handoffs/DC-*` companions.

## Archive

These records currently live under `archive/`.

| ID | Title | Status |
|---|---|---|
| DC-09 | [Phase 4 Node Model and Operation Application](./archive/DC-09-PHASE-4-NODE-MODEL.md) | Superseded / partially implemented historical umbrella. |

## Handoffs

Companion handoff directories currently exist for DC-10 through DC-25:

- [DC-10 rollback draft signing](./handoffs/DC-10-rollback-draft-signing/)
- [DC-11 maintainer trust store](./handoffs/DC-11-maintainer-trust-store/)
- [DC-12 arbitrary-span text edits](./handoffs/DC-12-arbitrary-span-text-edits/)
- [DC-13 non-default ref genesis](./handoffs/DC-13-nondefault-ref-genesis/)
- [DC-14 arbitrary-span text inverse rollback](./handoffs/DC-14-arbitrary-span-text-inverse-rollback/)
- [DC-15 active-session integrity hardening](./handoffs/DC-15-active-session-integrity-hardening/)
- [DC-16 patch algebra foundation](./handoffs/DC-16-patch-algebra-foundation/)
- [DC-17 patch algebra evidence contract](./handoffs/DC-17-patch-algebra-evidence-contract/)
- [DC-18 patch algebra commutation confluence](./handoffs/DC-18-patch-algebra-commutation-confluence/)
- [DC-19 replay lifecycle crate boundary](./handoffs/DC-19-replay-lifecycle-crate-boundary/)
- [DC-20 replay boundary stabilization](./handoffs/DC-20-replay-boundary-stabilization/)
- [DC-21 merge conflict evidence contract](./handoffs/DC-21-merge-conflict-evidence-contract/)
- [DC-22 public merge evidence UX](./handoffs/DC-22-public-merge-evidence-ux/)
- [DC-23 merge evidence UX stabilization](./handoffs/DC-23-merge-evidence-ux-stabilization/)
- [DC-24 data model and trust/threat docs](./handoffs/DC-24-data-model-trust-threat-docs/)
- [DC-25 merge planning surface](./handoffs/DC-25-merge-planning-surface/)

## Current Reference Docs

Current-state references consolidate implementation facts for public documentation. They are not RFC
lifecycle records; their authoritative home is the published mdBook source under `docs/src/reference/`.

- [Data model](../docs/src/reference/data-model.md)
- [Trust and threat model](../docs/src/reference/trust-threat-model.md)

The old `rfcs/fdds/FDD-00-DATA-MODEL.md` and `rfcs/fdds/FDD-04-TRUST-THREAT-MODEL.md` paths are
temporary compatibility pointers for 0.16.1 and are scheduled for removal in 0.17.0 unless a later
review extends the window. Future `rfcs/fdds/` content is reserved for genuine gating FDDs.
