# Prikk Roadmap and Milestones v1.1

Status: Updated for Design-First Execution  
Date: 2026-06-26  
Planning Mode: FDD-first, implementation-gated  
Calendar Type: Indicative planning schedule, not a delivery promise

## 1. Purpose

This roadmap updates the previous project schedule to reflect the design refresh:

- Requirements v1.1 is the stable product baseline.
- External Design v1.1 is the system design baseline.
- FDD Package v0.1 is the implementation bridge.
- RFC-000 through RFC-029 define the staged work program.
- Final core implementation remains gated by FDD approval.

The schedule separates three kinds of work:

1. design and review work;
2. safe scaffolding work;
3. gated implementation work.

## 2. Planning Assumptions

| Area | Assumption |
|---|---|
| Team shape | 1 architect/reviewer, 2-4 Rust developers, 1 security reviewer part-time, 1 QA/release owner part-time. |
| Sprint length | 2 weeks. |
| Current planning date | 2026-06-26. |
| First active week | Week of 2026-06-29. |
| Schedule confidence | High for M0-M1 structure; medium for M2-M5; low for ecosystem phases until early implementation evidence exists. |
| Implementation discipline | Storage/WAL/schema/algebra/plugin execution remain blocked until relevant FDDs are approved. |

## 3. Roadmap Overview

```text
M0  Design Lock & Scaffolding
    ↓
M1  Core Storage, Identity, WAL, Refs
    ↓
M2  Minimal Patch Engine
    ↓
M3  Block DAG, Checkout, Merge, Conflict UX
    ↓
M4  WASM Plugin Host, Audit, Attestation
    ↓
M5  Local/Remote Sync and Quarantine
    ↓
M6  Alpha Hardening, GC, Backup, Trust Lifecycle
    ↓
M7  Public Preview / Beta Readiness
    ↓
M8+ Ecosystem: Git Import, Hosting/Forge, Release Packaging
```

## 4. Indicative Calendar Plan

| Phase | Target Dates | Duration | Main Output |
|---|---:|---:|---|
| M0-A: FDD Review Preparation | 2026-06-29 to 2026-07-03 | 1 week | Review checklist, reviewer assignments, issue board. |
| M0-B: FDD Approval | 2026-07-06 to 2026-07-17 | 2 weeks | FDD-03, FDD-02, FDD-01, FDD-04, FDD-05 approved. |
| M0-C: Safe Scaffolding | 2026-06-29 to 2026-07-17 | parallel | Workspace, CI, templates, newtypes, fixtures. |
| M1: Core Storage & Identity | 2026-07-20 to 2026-08-14 | 4 weeks | Object store, WAL, signatures, ref state/log, verify basics. |
| M2: Minimal Patch Engine | 2026-08-17 to 2026-09-11 | 4 weeks | Apply/inverse, text/binary ops, commutation basics, ConflictWitness. |
| M3: Block DAG & Checkout | 2026-09-14 to 2026-10-09 | 4 weeks | Seal, block DAG, branch refs, checkout, merge base, conflict UX. |
| M4: WASM Plugin & Audit | 2026-10-12 to 2026-11-06 | 4 weeks | WASM host, audit API, attestations, policy enforcement. |
| M5: Sync & Quarantine | 2026-11-09 to 2026-12-04 | 4 weeks | Local peer sync, signature/object validation, policy quarantine. |
| M6: Alpha Hardening | 2026-12-07 to 2027-01-15 | 5-6 weeks | GC basics, backup/export, deep verification, fuzzing, docs. |
| M7: Public Preview Readiness | 2027-01-18 to 2027-02-12 | 4 weeks | Packaging, release evidence, install docs, preview criteria. |

The calendar should be re-estimated after M1 because storage and crash-recovery evidence will reveal the real implementation velocity.

## 5. Milestone Details

### M0 — Design Lock and Safe Scaffolding

Goal: approve the implementation-critical design documents and prepare the repository without committing to final core logic prematurely.

Required design outputs:

- FDD-03 Object Schema and Canonical Identity;
- FDD-02 Storage Transaction Model;
- FDD-01 Patch Algebra;
- FDD-04 Threat Model v1.1;
- FDD-05 Plugin ABI.

Allowed engineering outputs:

- Rust workspace scaffold;
- CI lint/test skeleton;
- issue templates;
- error taxonomy draft;
- newtype wrappers;
- non-persistent canonical encoding prototype;
- fixture generation tools.

Exit criteria:

- all five FDDs approved;
- NFR v1.1 gates reflected in issues;
- RFC/FDD dependency map approved;
- no forbidden implementation merged;
- first M1 sprint backlog ready.

### M1 — Core Storage and Identity

Goal: implement the durable substrate of Prikk.

Scope:

- `.prikk/` repository initialization;
- ObjectEnvelope read/write/verify;
- ObjectId computation;
- Ed25519 signatures and role-bound verification;
- WAL append/replay/truncation;
- active.lock and ref-specific locks;
- RefState object and RefUpdate log;
- `prikk verify` basic corruption checks.

Exit criteria:

- forced-kill tests for commit, object write, ref update;
- valid object/sig fixtures;
- corrupt object and corrupt WAL tests;
- cache/index deletion does not destroy authoritative data;
- no block references missing patch objects.

### M2 — Minimal Patch Engine

Goal: implement the smallest correct patch engine before expanding functionality.

Scope:

- repo state abstraction;
- Create/Delete/Edit/Rename/ChangePerm/Symlink/ReplaceBinary primitive apply;
- operation preconditions;
- inverse generation for supported operations;
- basic non-conflicting commutation;
- ConflictWitness generation.

Exit criteria:

- property tests for apply + inverse;
- conflict witness fixtures;
- no commutation unless FDD-defined proof conditions hold;
- binary blobs treated as opaque replace.

### M3 — Block Lifecycle, DAG, Checkout, Merge

Goal: make Prikk behave like a minimal VCS locally.

Scope:

- seal without external sync;
- signed BlockRecord creation;
- multi-parent block support;
- branch ref movement;
- checkout materialization with path safety;
- merge-base on DAG;
- bounded active block behavior;
- conflict status and repair workflow skeleton.

Exit criteria:

- create branch, commit, seal, checkout sequence works;
- multi-parent merge block test passes;
- path traversal and symlink escape tests pass;
- active block warnings and limits are enforced;
- conflict explanation is actionable.

### M4 — WASM Plugin Host, Audit, Attestation

Goal: add governance without weakening core safety.

Scope:

- WASM runtime with capability manifest;
- memory/fuel/time limits;
- audit plugin input/output schema;
- audit-secrets example plugin;
- AttestationRecord creation;
- publication policy check before ref advancement;
- seal failure UX.

Exit criteria:

- plugin cannot read arbitrary files or use network;
- plugin resource exhaustion is contained;
- failing audit blocks seal, not commit;
- attestations target blocks and are referenced by publication ref state/log;
- audit output is understandable.

### M5 — Sync and Quarantine

Goal: safely exchange objects and refs between repositories.

Scope:

- local filesystem peer sync;
- object negotiation;
- signature verification on incoming data;
- policy mismatch quarantine;
- ref rollback detection;
- partial clone design hooks if ready.

Exit criteria:

- incoming corrupted object rejected;
- missing object dependency detected;
- policy mismatch quarantines instead of accepting;
- ref rollback attempt detected;
- sync does not require trusted remote behavior.

### M6 — Alpha Hardening

Goal: make Prikk reliable enough for internal dogfooding.

Scope:

- GC reachability prototype;
- backup/export bundle;
- deep verification mode;
- performance benchmark harness;
- fuzzing campaign;
- error/doctor UX cleanup;
- trust store/key rotation design implementation if ready.

Exit criteria:

- repository survives repeated crash/fuzz tests;
- backup/restore verified offline;
- GC cannot delete reachable data;
- benchmark report exists;
- known limitations documented.

### M7 — Public Preview / Beta Readiness

Goal: prepare for external users without promising stable format too early.

Scope:

- release packaging dry runs;
- signed artifacts;
- SBOM;
- installation docs;
- migration warning docs;
- security reporting workflow;
- release verification evidence.

Exit criteria:

- reproducible or documented release build process;
- release artifacts are signed and verifiable;
- users can install and verify installation;
- preview limitations are clear;
- repository format stability policy is documented.

## 6. RFC Placement by Milestone

| Milestone | Primary RFCs |
|---|---|
| M0 | RFC-000, RFC-001, RFC-002, RFC-003, RFC-015, RFC-016, FDD package. |
| M1 | RFC-002, RFC-003, RFC-004, RFC-005, RFC-006, RFC-007, RFC-012. |
| M2 | RFC-008, RFC-009, RFC-010. |
| M3 | RFC-006, RFC-007, RFC-011, RFC-012, RFC-018, RFC-024. |
| M4 | RFC-013, RFC-014, RFC-015, RFC-025. |
| M5 | RFC-017, RFC-026. |
| M6 | RFC-019, RFC-020, RFC-021, RFC-022, RFC-023, RFC-025. |
| M7 | RFC-029. |
| M8+ | RFC-027, RFC-028, expanded RFC-029. |

## 7. Release Roadmap

| Release | Approx. Milestone | Purpose | Stability |
|---|---|---|---|
| v0.1-dev | M1 | Durable object/WAL/ref substrate. | Internal only. |
| v0.2-dev | M2 | Patch apply/inverse/commutation basics. | Internal only. |
| v0.3-dev | M3 | Local minimal VCS workflow. | Internal dogfood candidate. |
| v0.4-alpha | M4 | Seal-time audit and attestation. | Alpha. |
| v0.5-alpha | M5 | Safe local peer sync. | Alpha. |
| v0.6-beta-prep | M6 | GC, backup, verification, hardening. | Beta candidate. |
| v0.7-preview | M7 | Signed public preview. | Preview, format not necessarily stable. |
| v1.0 | Later | Stable repository format and user-facing guarantees. | Stable. |

## 8. Critical Risks and Schedule Buffers

| Risk | Impact | Mitigation |
|---|---|---|
| Patch algebra proves more complex than expected. | M2/M3 delay. | Keep M2 primitive scope small; defer advanced operations. |
| Filesystem fsync semantics differ by platform. | M1 reliability risk. | Document platform behavior in FDD-02; test Linux/macOS/Windows separately. |
| Plugin sandbox integration takes longer. | M4 delay. | Keep v1 WASM-only; minimal capabilities. |
| Conflict UX becomes too technical. | M3 usability risk. | RFC-024 and golden CLI outputs. |
| Repository format changes after implementation. | Rework risk. | FDD-03 must freeze schema before final storage code. |
| Remote protocol expands too soon. | M5 complexity risk. | Local peer sync first; harden before server mode. |

## 9. Replanning Rules

Replan after:

- FDD approval;
- M1 crash-test evidence;
- M2 property-test evidence;
- first end-to-end local seal/checkout workflow;
- first plugin sandbox proof;
- first sync/quarantine proof.

Any slip in a hard correctness gate should move the schedule, not weaken the gate.
