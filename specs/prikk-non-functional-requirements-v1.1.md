# Prikk Non-Functional Requirements v1.1

Status: Updated for Design-First Execution  
Date: 2026-06-26  
Applies to: Prikk Requirements v1.1, External Design v1.1, FDD Package v0.1, RFC-000 through RFC-029

## 1. Purpose

This document updates the Prikk non-functional requirements after the design refresh. The purpose is not to change the product direction, but to convert quality goals into measurable gates for design approval, implementation, testing, and release.

Prikk is a version control system, an append-only durability system, a patch-algebra engine, and a security-sensitive publication system. Its non-functional requirements must therefore be stricter than those of an ordinary CLI application.

## 2. Design-First Gate Model

The following work remains allowed immediately:

- documentation refinement;
- FDD review and approval;
- repository scaffolding;
- CI skeletons;
- issue templates;
- newtype skeletons;
- non-persistent prototypes for canonical encoding and fixtures.

The following work remains blocked until FDD approval:

- final object storage implementation;
- final WAL implementation;
- final schema/protobuf definitions;
- final locking implementation;
- patch algebra and commutation implementation;
- plugin runtime execution path;
- publication policy enforcement.

## 3. FDD Traceability Matrix

| NFR Area | Primary FDD | Secondary FDDs | Notes |
|---|---|---|---|
| Canonical object identity | FDD-03 | FDD-02, FDD-04 | Object ID formula, deterministic encoding, signatures, validators. |
| WAL durability and crash recovery | FDD-02 | FDD-03, FDD-04 | Commit success means signed envelope is fsync'd to WAL. |
| Ref state, ref log, CAS safety | FDD-02 | FDD-03 | RefState object model, ref pointer file, log replay, rollback detection. |
| Patch correctness and algebra | FDD-01 | FDD-03 | Apply, inverse, preconditions, commutation, conflict witnesses. |
| Path safety and worktree safety | FDD-03 | FDD-04 | UTF-8 path constraint, NFC normalization, symlink and traversal policy. |
| Plugin sandboxing | FDD-05 | FDD-04 | WASM capabilities, fuel/memory limits, no ambient IO. |
| Audit and attestation trust | FDD-05 | FDD-03, FDD-04 | Attestations target blocks; publication policy links attestations to refs. |
| Threat coverage | FDD-04 | All FDDs | Plugin escape, decompression bombs, malformed input, ref rollback, signature replay. |
| Verification and doctor UX | FDD-02 | FDD-03, FDD-04 | Corruption detection, repair scope, evidence output. |
| Performance and caching | FDD-02 | FDD-03 | Caches are rebuildable and never roots of trust. |

## 4. Non-Functional Requirement Matrix

### 4.1 Correctness and Integrity

| ID | Requirement | Target / Rule | Gate | Evidence |
|---|---|---|---|---|
| NFR-CI-01 | Object identity must be deterministic. | Same canonical payload always yields same ObjectId on all supported platforms. | M0 / FDD-03 | Canonical encoding test vectors. |
| NFR-CI-02 | Object signatures must be non-circular. | Signatures sign ObjectId and role-bound signature domain, not the payload that contains the signature. | M0 / FDD-03 | Signature fixtures, replay-negative tests. |
| NFR-CI-03 | Persisted objects must validate before use. | Invalid schema, duplicate fields, unsorted required lists, or unsupported versions are rejected. | M1 | Validator test suite. |
| NFR-CI-04 | Ref state must be verifiable. | Current ref pointer, RefState object, and RefUpdate log must form a consistent chain. | M1/M3 | Ref rollback and lost-ref recovery tests. |
| NFR-CI-05 | Verification must detect corruption. | Bit flips in WAL, objects, refs, signatures, and logs are detected. | M1 | `prikk verify` corruption fixtures. |

### 4.2 Durability and Recovery

| ID | Requirement | Target / Rule | Gate | Evidence |
|---|---|---|---|---|
| NFR-DR-01 | Commit durability. | `prikk commit` returns success only after signed ObjectEnvelope is appended to WAL and fsync'd. | M1 | Crash test after commit success. |
| NFR-DR-02 | WAL recovery. | Partial trailing WAL entries are truncated safely; valid entries remain recoverable. | M1 | Forced-kill WAL matrix. |
| NFR-DR-03 | Seal atomicity. | A crash during seal never produces a ref pointing to missing patches or blocks. | M1/M3 | Seal crash matrix. |
| NFR-DR-04 | Ref update durability. | Ref candidate is fsync'd, atomically renamed, and directory fsync'd. | M1 | Ref update crash tests. |
| NFR-DR-05 | Recovery UX. | Common WAL/index/ref corruption has actionable `prikk doctor` output. | M1/M3 | Doctor scenario tests. |
| NFR-DR-06 | Manual repair boundary. | Cases that cannot be safely repaired automatically must be reported as manual repair, never guessed. | M1/M3 | Negative recovery tests. |

### 4.3 Patch Algebra Safety

| ID | Requirement | Target / Rule | Gate | Evidence |
|---|---|---|---|---|
| NFR-PA-01 | Patch operations must have explicit order. | Operations use `op_seq`; validators reject duplicates or gaps if FDD-01 requires strict sequence. | M0/M2 | Schema and algebra tests. |
| NFR-PA-02 | Preconditions must be enforced. | Patch application fails safely if required preconditions do not hold. | M2 | Apply failure fixtures. |
| NFR-PA-03 | Inverse correctness. | For supported primitive operations, applying a patch and its inverse returns to the original state when preconditions hold. | M2 | Property tests. |
| NFR-PA-04 | Commutation correctness. | The engine may commute patches only when FDD-defined proof conditions hold. | M2 | Property tests and conflict fixtures. |
| NFR-PA-05 | Conflict explainability. | Non-commuting patches produce a first-class ConflictWitness suitable for CLI explanation. | M2/M3 | Conflict witness golden tests. |

### 4.4 Security

| ID | Requirement | Target / Rule | Gate | Evidence |
|---|---|---|---|---|
| NFR-SEC-01 | Safe defaults. | Plugins have no filesystem, network, or process capability by default. | M4 | Sandbox denial tests. |
| NFR-SEC-02 | Role-bound signatures. | Author, maintainer, CI, and ref-updater signatures are not interchangeable. | M1 | Cross-role replay-negative tests. |
| NFR-SEC-03 | Path safety. | Absolute paths, `..`, reserved names, symlink escape, and case-insensitive collisions are rejected. | M1/M3 | Path traversal fixtures. |
| NFR-SEC-04 | Malformed input handling. | Malformed objects, WAL entries, plugin outputs, and remote objects never panic or corrupt state. | M1-M5 | Fuzzing and negative fixtures. |
| NFR-SEC-05 | Plugin resource limits. | WASM plugins are bounded by memory/fuel/time limits. | M4 | Timeout/OOM/fuel tests. |
| NFR-SEC-06 | Policy downgrade resistance. | Publication policy and required attestations are recorded in RefUpdate/RefState history. | M4/M5 | Policy mismatch tests. |

### 4.5 Performance and Scalability

| ID | Requirement | Target / Rule | Gate | Evidence |
|---|---|---|---|---|
| NFR-PERF-01 | Commit latency. | Commit cost is bounded by patch construction, signature, WAL append, and fsync; no plugin scan or full-tree scan. | M1 | Commit benchmark report. |
| NFR-PERF-02 | Active block bound. | Warn at 800 active patches; hard block at 1000 by default unless configured. | M3 | CLI behavior tests. |
| NFR-PERF-03 | Merge scope. | Merge complexity claims are scoped to active block size and sealed block summaries. | M3 | Merge benchmarks with bounded active blocks. |
| NFR-PERF-04 | Cache trust boundary. | Indexes and caches improve performance but are rebuildable and never authoritative. | M1/M3 | Cache deletion/rebuild tests. |
| NFR-PERF-05 | Verification modes. | Provide quick verification for common operations and deep verification for audits/releases. | M3/Beta | Verify mode tests and timing reports. |

### 4.6 Reliability and Availability

| ID | Requirement | Target / Rule | Gate | Evidence |
|---|---|---|---|---|
| NFR-REL-01 | No silent data loss. | On uncertainty, Prikk preserves objects and reports manual repair rather than deleting data. | M1 | Recovery tests. |
| NFR-REL-02 | Safe GC. | GC deletes only unreachable objects proven unreachable by authoritative refs and retention policy. | RFC-020 / Beta | GC reachability tests. |
| NFR-REL-03 | Backup and restore. | Repository export/backup can be verified offline before restore. | RFC-022 / Beta | Bundle restore test. |
| NFR-REL-04 | Error taxonomy. | Errors are typed, actionable, and distinguish corruption, policy failure, conflict, and concurrency. | M1 | Error snapshot tests. |

### 4.7 Usability and Diagnostics

| ID | Requirement | Target / Rule | Gate | Evidence |
|---|---|---|---|---|
| NFR-UX-01 | Clear seal failure. | Seal audit failures list plugin, file/line when available, severity, and suggested fix. | M4 | CLI golden tests. |
| NFR-UX-02 | Clear conflict output. | Conflict output shows diff context plus algebraic reason. | M2/M3 | Conflict UX fixtures. |
| NFR-UX-03 | Recovery guidance. | `doctor` never prints cryptic internal-only errors without user action. | M1/M3 | Doctor output review. |
| NFR-UX-04 | Progressive disclosure. | Advanced proof/debug data is available but not the default user output. | M3/M4 | CLI review. |

### 4.8 Portability and Filesystem Behavior

| ID | Requirement | Target / Rule | Gate | Evidence |
|---|---|---|---|---|
| NFR-PORT-01 | Supported platforms. | Linux, macOS, and Windows are considered design targets; platform-specific fsync/rename semantics are documented. | FDD-02/M1 | Platform transaction notes and tests. |
| NFR-PORT-02 | UTF-8 path constraint. | v1 accepts normalized UTF-8 repo paths only. | M1/M3 | Path fixtures. |
| NFR-PORT-03 | Cross-platform reserved names. | Windows reserved names are rejected even on Unix to preserve portability. | M1/M3 | Path fixtures. |
| NFR-PORT-04 | Case collision handling. | Case-insensitive collisions are rejected by repo policy. | M1/M3 | Collision tests. |

### 4.9 Maintainability and Engineering Quality

| ID | Requirement | Target / Rule | Gate | Evidence |
|---|---|---|---|---|
| NFR-MAINT-01 | Rust safety policy. | Core crates use `#![forbid(unsafe_code)]`; any exception needs RFC approval. | Scaffolding/M1 | CI lint. |
| NFR-MAINT-02 | Module boundaries. | Object, store, patch, core, worktree, crypto, plugin host, CLI remain separated. | Scaffolding/M1 | Workspace review. |
| NFR-MAINT-03 | Test-first critical paths. | Object identity, WAL, refs, patch algebra, and plugin sandbox need tests before broad feature expansion. | M1-M4 | CI evidence. |
| NFR-MAINT-04 | Spec drift control. | FDD/RFC deviations require documented sign-off. | Always | RFC log. |

## 5. Milestone NFR Gates

| Milestone | Required NFR Evidence |
|---|---|
| M0 — Design Approval | FDD-01 through FDD-05 approved; canonical identity vectors; transaction crash matrix; threat model signoff. |
| M1 — Core Storage & Identity | WAL crash tests; object corruption tests; signature fixtures; ref update tests; path validation basics. |
| M2 — Patch Engine | Apply/inverse property tests; precondition fixtures; commutation tests; ConflictWitness fixtures. |
| M3 — Block DAG & Checkout | Multi-parent block tests; checkout path safety; merge-base tests; active block limit behavior. |
| M4 — Plugin & Audit | WASM sandbox tests; resource limit tests; audit failure UX; attestation linking tests. |
| M5 — Sync | Quarantine tests; remote object validation; ref rollback detection; policy mismatch tests. |
| Beta | Deep verification mode; GC/backup tests; performance reports; fuzzing campaign summary; recovery guide. |
| Public Preview | Reproducible release evidence; signed artifacts; SBOM; installation verification; security reporting process. |

## 6. Hard Blockers

Implementation must not proceed past scaffolding if any of the following are unresolved:

1. FDD-03 not approved before final schema/object storage work.
2. FDD-02 not approved before WAL/ref locking work.
3. FDD-01 not approved before commutation logic.
4. FDD-04/FDD-05 not approved before plugin runtime execution.
5. No crash-test plan for M1.
6. No canonical encoding test vectors.
7. No role-bound signature negative tests.
8. No path traversal/collision test plan before checkout implementation.

## 7. Evidence Repository Layout Recommendation

```text
verification/
  canonical-encoding/
  crash-matrix/
  fuzzing/
  property-tests/
  signature-fixtures/
  sandbox-tests/
  path-safety/
  benchmarks/
  release-evidence/
```

## 8. Approval Rule

NFR v1.1 is accepted when:

- each FDD explicitly references its relevant NFR gates;
- each milestone has test/evidence tasks in the issue tracker;
- no implementation-blocking NFR is deferred without RFC approval.
