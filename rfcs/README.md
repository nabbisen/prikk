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

These records are under design review. All proposed RFCs must respect the dependencies in
[`MILESTONES.md`](../MILESTONES.md).

| ID | Title | Milestone |
|---|---|---|
| DC-42 | [Performance and Maintainability Gates](./proposed/DC-42-PERFORMANCE-MAINTAINABILITY-GATES.md) | M2 / 0.19.0 |
| DC-43 | [Release Security and Distribution Controls](./proposed/DC-43-RELEASE-SECURITY-CONTROLS.md) | M2 / 0.19.0 |
| DC-44 | [Migration, Backup, and Restore Evidence](./proposed/DC-44-MIGRATION-BACKUP-RESTORE-EVIDENCE.md) | M3 / unassigned |
| DC-49 | [Portable-Logic Platform Matrix](./proposed/DC-49-PORTABLE-LOGIC-PLATFORM-MATRIX.md) | M2; blocked on a release-lane event |
| DC-52 | [Python and Oracle Decommissioning](./proposed/DC-52-PYTHON-ORACLE-DECOMMISSIONING.md) | M2 / 0.19.0 |
| DC-53 | [Repository-Wide AUTHOR Trust Verification](./proposed/DC-53-REPOSITORY-WIDE-AUTHOR-TRUST-VERIFICATION.md) | Post-M2, unscheduled |

DC-41 design review is accepted (below); its staged implementation does not require DC-42 or DC-43 to
wait. The active M2 development order is DC-55, then DC-42, then DC-52, then DC-43. **DC-55 is accepted
and cleared to start**; the rest still require individual design acceptance before implementation — see
[`EXECUTION-ORDER.md`](./EXECUTION-ORDER.md) for what each is blocked on and why DC-55 precedes DC-42.
DC-44 remains scheduled after M2 and DC-53 is
unscheduled; both have design briefs rather than implementation handoffs. DC-49 is proposed but cannot
complete while the release lane is parked. DC-45 through DC-48 are accepted preparatory work already
landed, not competing future increments in this sequence.

## Accepted

These reviewed designs may govern downstream work but have not yet released.

| ID | Title | Milestone |
|---|---|---|
| DC-34 | [Publication and Identity Authority](./accepted/DC-34-PUBLICATION-IDENTITY-AUTHORITY.md) | M0 complete; governs DC-38 through DC-40 |
| DC-35 | [Release Compatibility and Status Correction](./accepted/DC-35-RELEASE-COMPATIBILITY-STATUS-CORRECTION.md) | M1 / 0.18.0; implementation accepted; signer bootstrap pending separately |
| DC-36 | [Existing-Object Publication Integrity](./accepted/DC-36-EXISTING-OBJECT-PUBLICATION-INTEGRITY.md) | M1 / 0.18.0; implementation accepted |
| DC-37 | [Required Filesystem Durability](./accepted/DC-37-REQUIRED-FILESYSTEM-DURABILITY.md) | M1 / 0.18.0; implementation accepted |
| DC-38 | [Ref Publication Crash Recovery](./accepted/DC-38-REF-PUBLICATION-CRASH-RECOVERY.md) | M1 / 0.18.0; implementation accepted |
| DC-39 | [Signature and Envelope Authority](./accepted/DC-39-SIGNATURE-ENVELOPE-AUTHORITY.md) | M1 / 0.18.0; complete at `8f565f2`, post-commit evidence accepted |
| DC-40 | [State Merkle Root and Format Transition](./accepted/DC-40-STATE-MERKLE-FORMAT-TRANSITION.md) | M1 / 0.18.0; complete at `70c3902`, post-commit evidence accepted |
| DC-41 | [Integrity Evidence Campaign](./accepted/DC-41-INTEGRITY-EVIDENCE-CAMPAIGN.md) | M2 / 0.19.0; all four stages implemented and accepted (`fb4153c`, `d5bd096`, `540d4db`, `2824695`); descoped platform matrix tracked as DC-49 |
| DC-50 | [First-Party SHA-256 ROI Decision](./accepted/DC-50-FIRST-PARTY-SHA256-ROI-DECISION.md) | M2; accepted by the project owner 2026-07-28 with the performance question and DC-51 allowlist collision folded in. **Closed at `4005efb` with a replace decision**; produces no code, so it stays here rather than moving to `done/`. Authorized DC-55 |
| DC-51 | [Product Dependency Placement Gate](./accepted/DC-51-PRODUCT-DEPENDENCY-PLACEMENT-GATE.md) | M2; accepted by the project owner 2026-07-28 after the author's re-examination folded in the `[target.*]` and dependency-renaming amendments. Implementation complete at `d3e939b`, post-commit review accepted with one blocking finding, repaired at `4c8b7a3` |
| DC-54 | [Operation Path Validation Symmetry](./accepted/DC-54-OPERATION-PATH-VALIDATION-SYMMETRY.md) | M2; accepted by the project owner 2026-07-28 after the author's design-completion self-critique. Implementation complete at `e8f780a`, architect post-commit review accepted 2026-07-28, no repair required. Opened by the DC-41 stage-4 campaign finding |
| DC-55 | [First-Party SHA-256 Replacement](./accepted/DC-55-FIRST-PARTY-SHA256-REPLACEMENT.md) | M2; accepted by the project owner 2026-07-28 after design review v1's blocking finding and five notes were resolved in revision. Identity-bearing; design review was author re-examination and the RFC's Status field records that. Implementation pending |
| DC-45 | [Release Policy Tooling Consolidation](./accepted/DC-45-RELEASE-POLICY-TOOLING-CONSOLIDATION.md) | M2 / 0.19.0; Rust command authoritative, later stability and Python retirement pending |
| DC-46 | [Workspace Rust 1.85 Compatibility](./accepted/DC-46-WORKSPACE-RUST-1.85-COMPATIBILITY.md) | M2 / before 0.19.0 RC; complete at `0d221af`, post-commit evidence accepted |
| DC-47 | [Stable Clippy Gate Alignment](./accepted/DC-47-STABLE-CLIPPY-GATE-ALIGNMENT.md) | M2 / before 0.19.0 RC; complete at `ea95e92`, post-commit evidence accepted |
| DC-48 | [Legacy Clippy Production Retirement](./accepted/DC-48-LEGACY-CLIPPY-PRODUCTION-RETIREMENT.md) | M2 / before 0.19.0 RC; complete at `383e503`, post-commit evidence accepted |

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
| DC-25 | [Merge Planning Surface](./done/DC-25-MERGE-PLANNING-SURFACE.md) |
| DC-26 | [Documentation Home Correction](./done/DC-26-DOCUMENTATION-HOME-CORRECTION.md) |
| DC-27 | [Patch Algebra and Merge-Evidence Concepts Reference](./done/DC-27-PATCH-ALGEBRA-MERGE-EVIDENCE-CONCEPTS.md) |
| DC-28 | [Durability and Crash-Recovery Reference](./done/DC-28-DURABILITY-CRASH-RECOVERY-REFERENCE.md) |
| DC-29 | [Verify and Doctor Integrity/Recovery Reference](./done/DC-29-VERIFY-DOCTOR-INTEGRITY-RECOVERY-REFERENCE.md) |
| DC-30 | [Key Management and Signing Setup Guide](./done/DC-30-KEY-MANAGEMENT-SIGNING-SETUP-GUIDE.md) |
| DC-31 | [Repository Layout and Authority Reference](./done/DC-31-REPOSITORY-LAYOUT-AUTHORITY-REFERENCE.md) |
| DC-32 | [Path and Worktree Safety Reference](./done/DC-32-PATH-WORKTREE-SAFETY-REFERENCE.md) |
| DC-33 | [Concurrency and Locking Reference](./done/DC-33-CONCURRENCY-LOCKING-REFERENCE.md) |
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

Companion handoff directories currently exist for DC-10 through DC-25 and corrective DC-37, DC-39,
and DC-40:

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
- [DC-37 required filesystem durability](./handoffs/DC-37-required-filesystem-durability/)
- [DC-39 signature and envelope authority](./handoffs/DC-39-signature-envelope-authority/)
- [DC-40 state Merkle and format transition](./handoffs/DC-40-state-merkle-format-transition/)

## Current Reference Docs

Current-state references consolidate implementation facts for public documentation. They are not RFC
lifecycle records; their authoritative home is the published mdBook source under `docs/src/reference/`.

- [Data model](../docs/src/reference/data-model.md)
- [Repository layout and authority](../docs/src/reference/repository-layout.md)
- [Concurrency and locking](../docs/src/reference/concurrency-locking.md)
- [Path and worktree safety](../docs/src/reference/path-safety.md)
- [Trust and threat model](../docs/src/reference/trust-threat-model.md)
- [Durability and crash recovery](../docs/src/reference/durability-recovery.md)
- [Integrity and recovery diagnostics](../docs/src/reference/integrity-recovery.md)
- [Patch algebra and merge evidence](../docs/src/reference/patch-algebra.md)

The old `rfcs/fdds/FDD-00-DATA-MODEL.md` and `rfcs/fdds/FDD-04-TRUST-THREAT-MODEL.md` compatibility
pointers were removed in 0.17.0 after the 0.16.1 transition window. Future `rfcs/fdds/` content is
reserved for genuine gating FDDs.
