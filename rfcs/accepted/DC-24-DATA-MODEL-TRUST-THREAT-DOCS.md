# RFC (accepted) - DC-24 Data Model and Trust/Threat Documentation

**Status.** Accepted for implementation after architect design review.
**Target release.** v0.16.0 candidate, if accepted before release.
**Tracks.** TASK-02 consolidated data-model and trust/threat-model documentation.
**Touches.** mdBook documentation, RFC/FDD reference structure, data-model wording, trust/threat
model wording, release and roadmap status.
**Companion handoffs.**
`../handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md`,
`../handoffs/DC-24-data-model-trust-threat-docs/fdd-docs-update.md`.

## Context

The published mdBook is now organized by operator function and covers command use. It still does not
give evaluators one reliable place to understand Prikk's data model, repository lifecycle, or current
trust and threat model.

The untracked baseline inputs from `.git-exclude/specs/` have been recapped into the tracked companion
handoff `../handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md`. Reviewers should use that
recap rather than relying on local scratch files. The remaining relevant material exists as fragments
across historical tracked records:

- node and operation model material in `rfcs/archive/DC-09-PHASE-4-NODE-MODEL.md`;
- publication trust and trust-store material in DC-11 and its FDD-02/FDD-04 handoffs;
- rollback signing and trust caveats in DC-10, DC-14, and DC-15;
- patch algebra, merge evidence, replay, and lifecycle details across DC-16 through DC-23;
- current implementation status in `rfcs/IMPLEMENTATION-STATUS.md`.

That fragmentation is acceptable for design history, but it is weak public documentation. A reader can
learn how to run commands before learning what a Block, Patch, RefState, RefUpdate, WAL, trust policy,
or verification result means. That ordering risks over-trust, especially because the current trust
posture is intentionally immature: single-maintainer threshold, no key revocation or rotation,
no hardware signing, no remote trust distribution, and no repository-wide AUTHOR trust enforcement.

DC-24 is a documentation-design increment. It does not change object formats, trust behavior, CLI
behavior, or repository semantics. Its job is to create a single reviewed documentation surface for the
current data model and current trust/threat model, with clear links back to the RFC/FDD authority.

## Design Goals

1. Give readers one stable entry point for Prikk's current data model: object/envelope identity,
   Patch, Block, RefState, RefUpdate, WAL, refs, node addressing, seal, replay, verification, and
   doctor.
2. Give readers one stable entry point for Prikk's current trust/threat model: AUTHOR and MAINTAINER
   roles, signing boundaries, local trust store, what `verify` checks, what `verify` does not check,
   and current key-lifecycle limits.
3. Surface honest trust caveats in the published book before users can mistake the tool for a
   production Git replacement.
4. Keep RFC/FDD material single-source. mdBook may introduce and link to authoritative references, but
   must not fork a second architecture or threat model that can drift.
5. Ground every security claim in current code or accepted/released RFC/FDD material.
6. Preserve the design-first lifecycle: review the documentation structure and claims before
   implementation.

## Non-goals

DC-24 does not add:

- repository-format changes;
- object schema changes;
- signature, key, trust-store, or verification behavior changes;
- AUTHOR trust-store enforcement;
- key rotation, revocation, expiration, hardware signing, or multi-maintainer thresholds;
- remote trust distribution, sync, remotes, or hosted forge behavior;
- merge execution, branch publication, conflict resolution, or persisted evidence objects;
- public Rust API stabilization for `prikk-replay`, `prikk-store`, or patch algebra internals;
- a replacement for the RFC lifecycle policy.

## Proposed Documentation Shape

### Authoritative Reference Home

Create reviewed FDD/reference files under `rfcs/` as the source of truth for architecture and threat
model claims:

```text
rfcs/fdds/
  FDD-00-DATA-MODEL.md
  FDD-04-TRUST-THREAT-MODEL.md
```

The numbering is intentional. `FDD-04` preserves continuity with the existing threat-model trace.
`FDD-00` is a new consolidation reference for current data-model facts that were previously split
across storage, schema, identity, and lifecycle notes. Each new FDD must explain that it consolidates
current-state material and that the broader aspirational FDD-01/FDD-02/FDD-03/FDD-05 scheme remains
unconsolidated or deferred unless later RFCs create those references.

`FDD-00-DATA-MODEL.md` should consolidate current implementation facts and released RFC decisions
about:

- object envelopes, object identity, canonical payload identity, and signatures;
- Patch payloads, operation identity, node addressing, and Patch purpose;
- Blocks as immutable sealed history units;
- RefState and RefUpdate publication evidence;
- active WAL semantics and active-session boundaries;
- repository layout at the conceptual level, without duplicating every path detail;
- seal, replay, checkout/materialization, verify, and doctor lifecycle boundaries;
- what remains intentionally deferred.

`FDD-04-TRUST-THREAT-MODEL.md` should consolidate current implementation facts and released RFC
decisions about:

- trust roots and role-separated signing;
- AUTHOR and MAINTAINER signature meaning;
- local maintainer trust store and policy;
- what `seal` checks before publication;
- what repository-wide `verify` checks and reports;
- what rollback-draft verification checks and does not check;
- known limits: single-maintainer threshold, explicit local trust setup, no rotation/revocation,
  no remote trust, no hardware signing, no repository-wide AUTHOR trust policy, and no stable
  repository-format guarantee;
- privacy expectations for diagnostic output.

These files must be written as current-state reference documents, not as new feature promises. They
must include claim-to-source anchor tables linking data-model, trust, and security claims to current
code paths or released RFC/FDD records. They should include provenance sections linking to the relevant
done/archive RFCs and handoffs.

### Published mdBook Surface

Add a small `docs/src/reference/` section to the mdBook:

```text
docs/src/reference/
  data-model.md
  trust-threat-model.md
```

The mdBook pages should be reader-facing entry points. They should:

- summarize only the minimum needed to orient readers;
- prominently state current trust and repository-format limits;
- link to the authoritative `rfcs/fdds/` files and relevant done RFCs;
- avoid copying long RFC/FDD passages;
- avoid claiming production readiness.

The mdBook pages must repeat the core safety caveats inline where public safety depends on seeing them
without following links: Prikk is early implementation software, it is not a production Git
replacement, maintainer trust is repository-local with the current minimal `required = 1` policy,
there is no key rotation or revocation, and `verify` is not a global trust proof. The detailed
architecture and threat model remain in `rfcs/fdds/`.

### Navigation

Add a top-level mdBook section after `# Guide` and before `# Contributing`:

```md
# Reference

- [Data Model](reference/data-model.md)
- [Trust and Threat Model](reference/trust-threat-model.md)
```

The section is intentionally not called `Architecture` unless the implementation creates a broader
architecture index. The first increment should stay focused on the two missing references.

## Required Claim Boundaries

The implementation must say, in public docs:

- Prikk is early implementation software and not a production Git replacement.
- `.prikk/` is Prikk's native repository format and is not Git-compatible storage.
- Ref files are pointers, not roots of trust.
- Blocks and publication objects are signed, but trust is local and policy-limited.
- MAINTAINER trust is repository-local and currently supports only the released minimal trust policy.
- Seal publication uses real role-bound Ed25519 MAINTAINER signatures verified against the local
  maintainer trust store.
- `verify` checks structural integrity and current local publication trust for relevant publication
  objects, but does not provide full historical PKI semantics.
- AUTHOR signatures are role-bound signatures on Patches, but repository-wide AUTHOR trust policy is
  not implemented.
- AUTHOR key material is supplied by the current environment-variable mechanism and Prikk does not
  provide local secret storage.
- Rollback-draft verification is structural/semantic for the supported subset and must not be described
  as full rollback authorization.
- There is no key rotation, revocation, expiration, multi-maintainer threshold, hardware signing,
  remote trust, sync trust, or stable migration policy yet.
- Durability and recovery claims are supported by the current unit/integration test evidence, not by a
  completed crash-matrix or fuzzing campaign.
- Linux is the only platform exercised by the current project gates; cross-platform fsync and path
  semantics remain design targets, not verified release claims.

The implementation must not say or imply:

- that `verify` proves a repository is globally trustworthy;
- that first observed maintainers are implicitly trusted;
- that current trust policy supports revocation or thresholds beyond `required = 1`;
- that durability has been proven by crash-matrix or fuzz evidence;
- that cross-platform filesystem behavior has been fully verified;
- that archived DC-09 is live implementation authority by itself;
- that FDD references are broader than the released implementation can support.

## Source Recap and Audit Requirements

Before implementation, the writer must audit
`rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md`. That recap captures the local
requirements, NFR, external-design, and v0.2.0 handoff inputs that are not under VCS. If the writer
uses any additional claim from the local `.git-exclude/specs/` files, that claim must first be added to
tracked RFC/FDD material.

The writer must also audit at least these tracked records and implementation surfaces:

- `rfcs/archive/DC-09-PHASE-4-NODE-MODEL.md`;
- `rfcs/done/DC-10-ROLLBACK-DRAFT-SIGNING.md`;
- `rfcs/done/DC-11-MAINTAINER-TRUST-STORE.md`;
- `rfcs/done/DC-13-NONDEFAULT-REF-GENESIS.md`;
- `rfcs/done/DC-14-ARBITRARY-SPAN-TEXT-INVERSE-ROLLBACK.md`;
- `rfcs/done/DC-15-ACTIVE-SESSION-INTEGRITY-HARDENING.md`;
- `rfcs/done/DC-16-PATCH-ALGEBRA-FOUNDATION.md` through
  `rfcs/done/DC-23-MERGE-EVIDENCE-UX-STABILIZATION.md`;
- `rfcs/handoffs/DC-11-maintainer-trust-store/fdd-02-update.md`;
- `rfcs/handoffs/DC-11-maintainer-trust-store/fdd-04-update.md`;
- `rfcs/handoffs/DC-14-arbitrary-span-text-inverse-rollback/fdd-04-update.md`;
- `rfcs/IMPLEMENTATION-STATUS.md`;
- `crates/prikk-crypto`;
- AUTHOR key input through `PRIKK_AUTHOR_KEY_ID` and `PRIKK_AUTHOR_SEED`;
- current signing, trust, verification, doctor, seal, and rollback-draft verification code.

If a claim cannot be grounded in code or released design records, the claim must be removed or marked
as future/deferred.

## Implementation Plan

1. Create `rfcs/fdds/` if it does not exist.
2. Write `FDD-00-DATA-MODEL.md` as the current data-model reference.
3. Write `FDD-04-TRUST-THREAT-MODEL.md` as the current trust/threat-model reference.
4. Add mdBook `reference/data-model.md` and `reference/trust-threat-model.md` pages that link to the
   FDD references and expose the key caveats.
5. Update `docs/src/SUMMARY.md`.
6. Update `README.md`, `ROADMAP.md`, and `rfcs/IMPLEMENTATION-STATUS.md` only enough to point readers
   at the new references and mark DC-24 status.
7. Run documentation verification.

## Review Gates

Design review should verify:

- the proposed authoritative home avoids split-brain documentation;
- the required trust caveats are complete and honest;
- the source audit list is sufficient;
- the mdBook surface is useful without copying RFC/FDD content.

Implementation review should verify:

```text
mdbook build docs
git diff --check
```

and should additionally include:

- proof that every new mdBook page is reachable from `SUMMARY.md`;
- grep evidence that no old architecture page names are orphaned if names change during review;
- a source-audit checklist showing which code/RFC/FDD files were checked;
- claim-to-source anchor tables in each new FDD reference;
- evidence that mdBook inline caveats match the FDD caveats without drift;
- line-count evidence for new documentation files if project documentation-size guidance applies.

## Acceptance Criteria

DC-24 is accepted when reviewers agree that:

- the data model has one current authoritative reference;
- the trust/threat model has one current authoritative reference;
- published docs expose the current trust limitations clearly;
- no implementation or release note overstates Prikk's maturity;
- future trust/key-lifecycle work remains clearly deferred.

DC-24 is done when the reviewed docs are committed, the mdBook build passes, and release/status files
point to the new references.
