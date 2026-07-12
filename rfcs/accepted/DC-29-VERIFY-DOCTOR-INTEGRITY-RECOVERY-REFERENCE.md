# RFC (accepted) - DC-29 Verify and Doctor Integrity/Recovery Reference

**Status.** Accepted for implementation after architect design review.
**Target release.** 0.17.3.
**Tracks.** TASK-07 verify and doctor integrity/recovery reference.
**Touches.** mdBook reference documentation, verify/doctor integrity wording, claim-to-source
anchors, roadmap/status docs.
**Companion handoff.** None. This is a current-state documentation reference and does not create a
gating FDD.

## Context

DC-24 added the current data-model and trust/threat references. DC-26 moved current-state references
into the published mdBook. DC-28 added the durability and crash-recovery reference and explicitly left
the full `verify` / `doctor` diagnostic catalog to TASK-07.

The current public docs state that `verify` is read-only, `doctor` is conservative, and `verify` is not
a global trust proof. They still do not give users and reviewers one authoritative page for:

- what `prikk verify` checks;
- what `prikk verify` intentionally does not prove;
- which verification failures are hard integrity failures versus reported trust issues;
- what `prikk doctor` diagnoses;
- what doctor issue codes and severities mean;
- what repairs are available and why unsafe cases stay manual.

DC-29 closes that documentation gap without changing verification, doctor, repair, trust, CLI, or
repository behavior.

## Problem

1. **Integrity checks are visible but under-explained.** CLI output prints counters and active-WAL
   metadata status, but users need a current-state reference for interpreting those fields.
2. **Trust scope is easy to overread.** `verify` checks repository-local publication trust for
   publication objects, but it does not enforce repository-wide AUTHOR trust or historical PKI
   semantics.
3. **Doctor diagnostics need a catalog.** `DoctorIssue` codes, severities, recommendations, and exit
   behavior are code-backed but not centrally documented.
4. **Recovery wording must not duplicate DC-28.** DC-28 owns durability/crash-recovery framing.
   DC-29 should own diagnostic interpretation and link to DC-28 for recovery mechanics.
5. **Repair posture needs honest limits.** Doctor has two opt-in repairs. It must remain clear that
   repairs are narrow and unsafe cases are reported rather than guessed.

## Design Goals

1. Add a self-contained current-state reference page at `docs/src/reference/integrity-recovery.md`.
2. Explain the scope of `prikk verify`: object placement and identity, envelope decoding, Block
   references, ref pointer/log consistency, active WAL replay, active WAL metadata health,
   rollback-draft/rollback-block classification, and publication-trust checks.
3. Explain the trust boundary: `verify` checks current repository-local publication trust for Block,
   RefState, and RefUpdate envelopes; it is not a global trust proof and does not enforce
   repository-wide AUTHOR trust.
4. Explain verification result counters and active-WAL metadata states at the level needed to read CLI
   output.
5. Explain `prikk verify` exit behavior: active-WAL metadata integrity issues and publication-trust
   issues produce command failure even when structural verification returns a report.
6. Explain `prikk doctor` as an actionable diagnostic layer over verification.
7. Catalog current doctor issue codes, severities, and recommendations without treating the output as
   a stable machine-readable schema.
8. Explain doctor repair switches and refusal conditions while cross-linking DC-28 for recovery
   mechanics.
9. Preserve honest caveats: no global trust proof, no repository-wide AUTHOR trust, no historical PKI,
   no broad repair, no missing-object synthesis, no production-readiness claim.
10. Include visible claim-to-source anchors linking major claims to code paths and released records.

## Non-goals

DC-29 does not add:

- code, schema, or CLI behavior;
- new verification checks;
- new doctor issue codes, severities, recommendations, or exit semantics;
- new repair behavior;
- repository-wide AUTHOR trust enforcement;
- key rotation, revocation, expiration, hardware signing, remote trust, or historical PKI;
- crash-recovery mechanics beyond the DC-28 reference;
- automatic stale-lock cleanup;
- missing-object, malformed-log, checksum-mismatch, signature, trust-policy, or key-material repair;
- stable machine-readable output for `verify` or `doctor`;
- repository-format stability or migration guarantees;
- a new current-state FDD under `rfcs/fdds/`.

## Proposed Documentation Shape

Create:

```text
docs/src/reference/integrity-recovery.md
```

Add it under the mdBook `# Reference` section near the durability and trust/threat references:

```md
- [Integrity and Recovery Diagnostics](reference/integrity-recovery.md)
```

The page should be written as a current-state reference, not a tutorial and not a future design. It
should cross-link:

- `docs/src/reference/trust-threat-model.md` for trust scope;
- `docs/src/reference/durability-recovery.md` for WAL/ref recovery mechanics;
- `docs/src/reference/data-model.md` for object/ref/WAL concepts;
- `docs/src/guide/rollback/rollback-draft-verify.md` for the stronger selected-ref rollback draft
  verification command.

### Boundary With DC-28

DC-28 owns durability and crash-recovery framing: active-WAL persistence, WAL-tail recovery,
ref-pointer reconstruction as a recovery action, and stale-lock caveats. DC-29 owns diagnostic
interpretation: what `verify` checks, what doctor reports, issue codes/severities, exit behavior, and
what users should understand from a diagnostic result.

The DC-29 implementation may summarize repair switches for completeness, but it must link to DC-28 for
the recovery mechanics and must not duplicate DC-28's detailed seal-publication flow or old-or-new
valid-state discussion.

### Required Sections

The implemented page should contain at least:

1. **Core Caveats.** Early implementation status, no global trust proof, no repository-wide AUTHOR
   trust, no historical PKI semantics, narrow repair boundary, no stable output schema.
2. **Verify Scope.** Current structural, WAL, ref, rollback, and publication-trust checks.
3. **What Verify Does Not Prove.** Production readiness, global trust, AUTHOR trust, key lifecycle,
   complete durability, cross-platform filesystem behavior, semantic merge safety, or stable format.
4. **Verify Output and Exit Behavior.** Counters, publication-trust issues, active-WAL metadata state,
   trailing partial WAL warning, and command failure conditions.
5. **Active WAL Metadata States.** The six current `ActiveWalMetadataStatus` variants: empty-WAL
   absent metadata, empty-WAL valid stale metadata, empty-WAL malformed stale metadata, non-empty WAL
   with valid metadata, non-empty WAL with missing metadata, and non-empty WAL with malformed
   metadata.
6. **Doctor Scope.** Doctor as a diagnostic layer over verification.
7. **Doctor Issue Catalog.** Current issue codes, severities, meaning, and action model, including all
   nine current codes and their no-issue healthy-state boundaries.
8. **Doctor Repair Boundary.** `--repair-wal-tail` and `--repair-main-ref`, refusal when repository
   health has errors, and manual-repair posture for unsafe cases.
9. **Relationship to Rollback Verification.** Repository `verify` counts/classifies rollback material;
   `rollback-draft-verify` is the selected-ref semantic check for an active rollback draft.
10. **Deferred Work.** Broader repair, stale-lock policy, key lifecycle, AUTHOR trust policy, JSON
    output, stable diagnostic schema, backup/restore, production readiness.
11. **Claim-to-Source Anchors.** A visible table tying claims to code paths and released records.
12. **Provenance.** State that the page consolidates released records through DC-28 and follows the
    DC-26 documentation-home model.

## Required Claim Boundaries

The implementation must say, in public docs:

- `prikk verify` is read-only.
- `prikk verify` checks structural integrity and current local publication trust for publication
  objects.
- Publication trust issues are reported separately from structural verification errors.
- `verify` does not prove global trust, repository-wide AUTHOR trust, historical PKI, revocation,
  rotation, remote trust, or production readiness.
- Active-WAL metadata issues on non-empty WALs are integrity issues; active-WAL metadata on empty WALs
  is local debris/warning state.
- Empty-WAL metadata debris has two code-distinct sub-cases: valid stale metadata and malformed stale
  metadata. Both are warnings.
- Empty WAL with absent metadata and non-empty WAL with valid metadata are healthy metadata states and
  do not produce doctor issues by themselves.
- `prikk doctor` is a diagnostic layer over verification.
- Doctor repairs are opt-in and narrow.
- Doctor refuses unsafe repair rather than guessing.
- Doctor recommendations are human guidance, not an automated recovery policy.
- Output fields and issue-code names are current CLI vocabulary, not stable machine-readable schema.

The implementation must not say or imply:

- that `verify` proves the repository is globally trustworthy;
- that AUTHOR signatures are checked against a repository-wide AUTHOR trust policy;
- that current trust supports key revocation, rotation, threshold policy beyond `required = 1`, or
  remote identity;
- that doctor can repair arbitrary corruption;
- that missing objects, malformed logs, checksum mismatches, signatures, trust policy, or key material
  can be synthesized;
- that a warning means data is safe to discard;
- that current diagnostics are a stable JSON/API contract;
- that current verification makes Prikk production-ready.

## Source Audit Requirements

Implementation must audit at least:

- `rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md`;
- `rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md`;
- `rfcs/done/DC-14-ARBITRARY-SPAN-TEXT-INVERSE-ROLLBACK.md`;
- `rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md`;
- `rfcs/done/DC-28-DURABILITY-CRASH-RECOVERY-REFERENCE.md`;
- `rfcs/done/PR-006-VERIFY-HANDOFF.md`;
- `rfcs/done/PR-010-VERIFY-HARDENING-HANDOFF.md`;
- `rfcs/done/PR-011-DOCTOR-HANDOFF.md`;
- `rfcs/done/PR-012-DOCTOR-REPAIR-HANDOFF.md`;
- `rfcs/done/PR-013-REF-RECOVERY-HANDOFF.md`;
- `rfcs/done/PR-029-ROLLBACK-DRAFT-VERIFY-HANDOFF.md`;
- `rfcs/done/PR-030-SEALED-ROLLBACK-HISTORY-HANDOFF.md`;
- `rfcs/IMPLEMENTATION-STATUS.md`;
- `docs/src/reference/data-model.md`;
- `docs/src/reference/trust-threat-model.md`;
- `docs/src/reference/durability-recovery.md`;
- `docs/src/guide/rollback/rollback-draft-verify.md`;
- `crates/prikk-store/src/verify.rs`;
- `crates/prikk-store/src/doctor.rs`;
- `crates/prikk-store/src/trust.rs`;
- `crates/prikk-store/src/rollback_verify.rs`;
- `crates/prikk-cli/src/main.rs`;
- `crates/prikk-cli/src/output.rs`;
- `crates/prikk-cli/src/args.rs`;
- `crates/prikk-cli/src/output/help.rs`.

The writer may use `.git-exclude/tasks/002-update-management/TASK-07-doc-verify-doctor.md` as
scheduling context, but claims must be grounded in tracked code or released RFCs. Local
`.git-exclude/specs/` files are not reviewer-facing authority unless recapped into tracked material.

Anchor fidelity is part of the implementation contract. The implementation must cite tracked code,
released DCs, and PR handoffs for diagnostic claims. It must not cite a standalone `FDD-02` as if that
file exists, and it must not propagate stale code-comment labels as documentation authority.

Current `DoctorIssue` code strings and severity labels may be documented as current CLI vocabulary
because they are current static strings in the implementation. The docs must not treat comments such as
"stable diagnostic code" or "stable lower-case label" as evidence of a stable machine-readable schema;
the RFC boundary that this is not a stable JSON/API contract is authoritative.

## Implementation Plan

1. Create `docs/src/reference/integrity-recovery.md`.
2. Add it to `docs/src/SUMMARY.md` under `# Reference`.
3. Cross-link from `docs/src/reference/trust-threat-model.md` and
   `docs/src/reference/durability-recovery.md` where verify/doctor boundaries already appear.
4. Cross-link from rollback verification docs where useful without duplicating command examples.
5. Update `README.md`, `ROADMAP.md`, `rfcs/README.md`, and `rfcs/IMPLEMENTATION-STATUS.md` only enough
   to reflect the active documentation increment and the new reference after implementation.
6. Do not change Rust code, command output, object schema, release version, or repository behavior
   during implementation.
7. Prepare an implementation review package after the page is drafted.

## Review Gates

Design review should verify:

- the page scope is current-state reference documentation, not new diagnostic behavior;
- DC-29 and DC-28 ownership boundaries are clear;
- the trust caveats prevent overclaiming `verify`;
- the doctor catalog scope is sufficient but does not imply a stable machine-readable schema;
- the source audit list covers code paths for verification, doctor output, repair, trust, and rollback
  verification;
- no current-state FDD under `rfcs/fdds/` is introduced.

Implementation review should verify:

```text
mdbook build docs
git diff --check
```

and should additionally include:

- proof that `docs/src/reference/integrity-recovery.md` is reachable from `docs/src/SUMMARY.md`;
- built-book link/reachability checks for integrity-recovery, trust-threat, durability-recovery, and
  rollback-draft-verify pages;
- a source-audit checklist showing which released DCs, PR handoffs, docs, and code paths were checked;
- verification that the page owns diagnostic interpretation only and does not duplicate DC-28 recovery
  mechanics;
- verification that the active-WAL metadata section maps one-to-one to the six
  `ActiveWalMetadataStatus` variants and explicitly identifies the two healthy no-issue states;
- verification that the doctor issue catalog enumerates all nine current issue codes, including
  `PRIKK-DOCTOR-ACTIVE-REF-METADATA-MALFORMED-DEBRIS`, with correct severities;
- verification that the repair section lists only `--repair-wal-tail`, `--repair-main-ref`, and the
  health-error refusal condition, links to `durability-recovery.md` for recovery mechanics, and does
  not restate the seal-publication flow or old-or-new-valid-state discussion from DC-28;
- verification that the page does not contain standalone `FDD-02` anchors or stale code-comment
  authority labels;
- claim-to-source anchor table review;
- line-count evidence for new/changed docs.

## Acceptance Criteria

DC-29 is complete when:

- `docs/src/reference/integrity-recovery.md` exists and is reachable from the mdBook summary;
- the page explains current `verify` checks, non-proofs, output, exit behavior, active-WAL metadata
  states, doctor diagnostics, doctor issue codes/severities, and repair boundaries;
- the page has visible claim-to-source anchors;
- related current-state reference pages cross-link where useful;
- ROADMAP/status docs track the documentation increment honestly;
- implementation review accepts the documentation; and
- the completed release records DC-29 as documentation-only with no code, schema, CLI, or repository
  behavior change.
