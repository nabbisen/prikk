# Prikk Stable App Requirements v1.1

Status: Stable Requirements Baseline — supporting refresh v1.2, Design-First Revision  
Date: 2026-06-26  
Project: Prikk — next-generation distributed version control system  
Supersedes: `prikk-stable-app-requirements-v1.0.md`  
Depends on: Prikk v0.9 Foundation Baseline, NFR v1.1, External Design v1.1, FDD package v0.3

---

## 1. Purpose of This Revision

This revision stabilizes the product requirements before implementation kickoff. It does not add a large new feature surface. Instead, it clarifies release scope, non-functional requirements, design gates, and ecosystem boundaries so that the project can proceed in a design-first way.

Prikk is a correctness-sensitive VCS. It must not begin unrestricted core implementation until the foundational design documents are approved. Repository scaffolding, CI preparation, developer onboarding, and non-persistent prototypes are allowed; final storage, WAL, schema, locking, patch algebra, and plugin ABI implementation remain gated.

---

## 2. Product Vision

Prikk is a standalone distributed version control system designed around block-oriented patch theory. It aims to provide the safety and semantic precision of patch-based versioning while maintaining practical performance through sealed immutable blocks.

Prikk is designed to be:

- easy to use for ordinary development workflows;
- safe and secure by default;
- resilient against corruption and interrupted operations;
- flexible enough for local, peer, and future hosted workflows;
- fast for long-lived repositories by keeping expensive patch reasoning bounded to active work.

Prikk is not a Git wrapper. It uses a native `.prikk/` repository format. Git import and migration tooling are allowed as adoption tools, but Git compatibility must not constrain the core repository model.

---

## 3. Product Scope

### 3.1 In Scope for the Core Product

The core product includes:

1. Native `.prikk/` repository initialization.
2. Durable commits through a signed WAL-backed patch object model.
3. Patch objects represented as canonical signed envelopes.
4. Sealed block DAG history.
5. Branch and tag references through signed ref-state objects.
6. Patch application, inverse, and minimal commutation logic.
7. Conflict witnesses when algebraic commutation fails.
8. Safe checkout and worktree materialization.
9. Integrity verification and recovery diagnostics.
10. WASM-only audit plugin host for v1.
11. Attestation records linked to blocks and publication policy.
12. Local peer sync as the first remote foundation.

### 3.2 In Scope as Ecosystem / Adoption Work

The following are in scope, but staged after the core foundations:

1. Git import and migration tooling.
2. Hosting / server mode / forge integration.
3. Release packaging, signing, and distribution verification.
4. Repository bundles, backup, export, and disaster recovery tooling.
5. Large-file policy and future pack/archive optimization.

### 3.3 Out of Scope for v1 Core

The following are out of scope for v1 core:

1. Native plugin execution inside the core process.
2. Git object compatibility.
3. Real-time collaborative editing using OT or CRDTs.
4. Centralized cloud service dependency.
5. Full forge replacement in the local CLI core.
6. Advanced cryptographic PKI, key revocation, and hardware signing as mandatory v1 requirements.
7. Semantic language-aware merge beyond the defined patch algebra.

---

## 4. User Classes

### 4.1 Developer

The developer performs everyday version-control tasks: commit, status, seal, branch, merge, checkout, verify, and recover. The developer needs fast feedback, clear errors, and minimal ceremony.

### 4.2 Maintainer

The maintainer controls publication, sealing policy, branch movement, and release readiness. The maintainer signs ref-state updates and is responsible for enforcing project policy.

### 4.3 Security / Compliance Reviewer

The security reviewer configures audit plugins, verifies attestations, reviews policy failures, and inspects published history.

### 4.4 Repository Administrator

The administrator manages repository format upgrades, backups, trust configuration, hosting integration, and long-term archival policy.

### 4.5 Tooling / Integration Developer

The tooling developer builds plugins, importers, forge integrations, IDE bridges, and release packaging automation around the stable core interfaces.

---

## 5. Core Concepts

### 5.1 Patch

A patch is the atomic unit of logical change. A patch contains an ordered sequence of operations, preconditions, optional advisory intent metadata, and author signature. Patch identity is derived from canonical unsigned payload data; signatures are external to the identity hash.

### 5.2 Operation

An operation is a single ordered change within a patch. Operation order is defined by explicit `op_seq`. Operations include file creation, deletion, text edit, rename, permission change, symlink creation, and binary replacement.

### 5.3 Block

A block is a sealed immutable collection of patches. Blocks form a DAG and may have multiple parents. A block is the scalability boundary for long-lived history.

### 5.4 Ref State

A ref state is a signed object representing the current state of a branch or tag reference. Ref files are pointers to current ref-state objects, not the root of trust.

### 5.5 Ref Update

A ref update is an append-only event describing a transition from an old ref state to a new ref state. It supports auditability and recovery.

### 5.6 Attestation

An attestation records policy/audit results about a target block. It does not define block identity. Publication policy links attestations to ref updates.

### 5.7 Seal

Seal is the operation that turns active signed patches into a block, runs required policy checks, writes attestations, and advances a ref if policy passes.

---

## 6. Functional Requirements

### 6.1 Repository Initialization

- `prikk init` must create a valid `.prikk/` layout.
- The repository must contain version metadata, trust-store scaffolding, object directories, active-session directories, ref directories, log directories, and configuration.
- Initialization must reject unsupported worktree path conditions, including path normalization failures and unsafe filesystem states.

### 6.2 Commit

- `prikk commit` must produce a signed `Patch` object envelope.
- The signed envelope must be appended to the active WAL.
- A commit must not return success until the WAL append is fsync'd.
- Commit must not run audit plugins or scan the full worktree.
- Commit latency is bounded by diff construction, envelope construction, signing, WAL append, and fsync.
- Recovery must reconstruct the exact signed patch envelope from WAL without re-signing.

### 6.3 Status

- `prikk status` must show worktree state, active patches, unsealed patch count, active branch/ref, and obvious integrity warnings.
- If known patch conflicts exist, status must show a human-readable reason, not just a generic conflict label.
- When active patch count reaches the warning threshold, status must recommend sealing.

### 6.4 Seal

- `prikk seal` must convert active WAL patches into persistent patch objects and a block object.
- Patch objects must be persisted before the block references them.
- Seal must run required policy checks when configured.
- Audit failure must block publication, not commit.
- Successful seal must create or update a signed ref state and append a ref-update record.
- Seal must be atomic from the user perspective: after a crash, the repository must recover to either the old published state or the new valid state.

### 6.5 Branch and Ref Management

- Branch heads must be represented by signed ref-state objects.
- Ref files must point to ref-state objects.
- Ref updates must use compare-and-swap semantics.
- Ref update logs must support rollback detection and recovery.
- Ref locks must be path-safe and must not allow ref-name path injection.

### 6.6 Tagging

- Tags must be immutable tag objects.
- Moving a tag must create a new ref state or require explicit policy, but must not mutate the original tag object.
- Tag refs must point to tag objects, not directly to arbitrary mutable data.

### 6.7 Checkout

- `prikk checkout` must materialize a worktree from a block or ref safely.
- Checkout must enforce path normalization and traversal rules.
- Checkout must reject paths that are unsafe on supported platforms.
- Binary files may be materialized as opaque blob replacements.

### 6.8 Merge and Patch Reasoning

- Merge must operate over block DAG history and active patch sets.
- Patch reasoning must be driven by algebraic rules, not advisory intent tags.
- When commutation fails, Prikk must produce a conflict witness.
- Intent metadata may improve display, grouping, and suggestions, but must not override correctness.

### 6.9 Verify and Doctor

- `prikk verify` must validate object IDs, signatures, canonical encoding, object reachability, block DAG consistency, ref-state chains, and WAL integrity where relevant.
- `prikk doctor` must diagnose common corruption and interrupted-operation states.
- Automatic repair must be conservative and must not discard user data without explicit confirmation or preservation.

### 6.10 Plugin Host and Audit

- v1 plugins must use WASM/WIT only.
- Plugins must run with explicit capabilities.
- Default plugin capability set is empty.
- Plugins must not access arbitrary filesystem paths, network, or process execution in v1.
- Audit plugins must produce deterministic or explicitly marked non-deterministic results.
- Attestations must be stored separately from blocks and target block IDs.

### 6.11 Remote Sync

- Local peer sync is the first remote foundation.
- Push and pull must verify object hashes, signatures, and policy compatibility.
- Policy mismatch must quarantine incoming data rather than silently accepting it.
- Advanced hosting, forge integration, and partial clone are staged follow-up work.

---

## 7. Non-Functional Requirements

### 7.1 Durability

- A successful commit must survive process crash and OS restart subject to normal filesystem fsync semantics.
- A successful ref update must survive process crash after directory fsync.
- Interrupted seal must be recoverable without repository corruption.
- Crash tests must cover WAL append, object write, block write, attestation write, ref candidate write, atomic rename, and ref log update.

### 7.2 Integrity

- All persistent objects must be content-addressed.
- Object IDs must be computed from canonical unsigned payloads.
- Signatures must bind object type, object ID, signer role, and key ID.
- Rebuildable indexes and caches must never be roots of trust.

### 7.3 Security

- v1 plugin execution must be WASM-only.
- Path safety must reject traversal, absolute paths, reserved names, unsafe symlinks, and case-insensitive collisions.
- The system must fail closed when policy or trust checks are ambiguous.
- Malformed input must produce safe errors, not panics or partial state mutation.

### 7.4 Performance

- Historical operations must be independent of total sealed patch count when bounded by block summaries and active patch limits.
- Active block warning threshold defaults to 800 patches.
- Active block hard threshold defaults to 1000 patches unless explicitly configured.
- Commit must avoid plugin overhead and full-tree scans.
- Performance claims must be benchmarked against explicit repository shapes.

### 7.5 Portability

- Supported platforms are Linux, macOS, and Windows.
- v1 repository paths are normalized UTF-8.
- Platform-specific filesystem behavior must be captured in tests.
- Unsafe path behavior must be rejected consistently across platforms where possible.

### 7.6 Usability

- Error messages must be actionable.
- Conflict messages must explain why patches do not commute.
- Recovery commands must clearly separate diagnosis from destructive repair.
- Daily commands must remain small and understandable.

### 7.7 Maintainability

- Core crates should forbid unsafe code unless explicitly justified.
- Storage, object identity, patch algebra, plugin host, and CLI layers must remain modular.
- Schema evolution must be explicit and versioned.
- RFC/FDD acceptance must include evidence.

---

## 8. Release Scope

### 8.1 M0 — Design and Safe Scaffolding

Allowed:

- requirements and external design stabilization;
- FDD authoring;
- RFC/handoff alignment;
- workspace scaffolding;
- CI/lint/test harness setup;
- non-persistent encoding prototypes;
- test fixtures and error taxonomy.

Forbidden:

- final object storage implementation;
- final WAL implementation;
- final locking implementation;
- final schema/proto freeze before FDD-03 approval;
- final patch commutation implementation before FDD-01 approval;
- final plugin ABI implementation before FDD-05 approval.

### 8.2 M1 — Core Storage and Identity

M1 begins after all five FDDs are approved. M1 delivers object identity, signed envelopes, WAL append/replay, atomic refs, verify, and crash recovery evidence.

### 8.3 M2 — Minimal Patch Engine

M2 delivers patch operation application, inverse, non-conflicting commutation, conflict witness generation, and property tests.

### 8.4 M3 — Block Lifecycle and DAG

M3 delivers seal without external ecosystem dependencies, block DAG navigation, branch refs, tags, checkout, and merge-base logic.

### 8.5 M4 — WASM Plugin Host and Audit

M4 delivers capability-limited WASM plugins, audit input/output schema, example audit plugin, attestation records, and publication-policy enforcement.

### 8.6 M5 — Local Peer Sync

M5 delivers local peer sync, object verification on transfer, quarantine, and policy mismatch handling.

### 8.7 M6/M7 — Beta Readiness and Ecosystem Preparation

M6/M7 may include GC/packing, backup/export, large-file policy, trust lifecycle, conflict UX, Git import, hosting/server mode, and release packaging.

---

## 9. Design Gates

The following FDDs are required before unrestricted M1 implementation:

1. FDD-03 — Object Schema and Canonical Identity.
2. FDD-02 — Storage Transaction Model.
3. FDD-01 — Patch Algebra.
4. FDD-04 — Threat Model v1.3.
5. FDD-05 — Plugin ABI v0.3.

FDD-03 must be approved first because the other FDDs depend on object identity and schema definitions. FDD-02 must align with FDD-03. FDD-01 must use FDD-03 operations. FDD-04 must consider FDD-02 and FDD-03. FDD-05 must reflect the FDD-04 threat model.

---

## 10. Acceptance Criteria for Requirements Completion

The requirements phase is considered complete when:

- product scope is stable;
- implementation gates are explicit;
- NFRs are measurable enough for milestone acceptance;
- FDD ownership and ordering are clear;
- core and ecosystem scopes are separated;
- no unresolved requirements-level contradiction remains.

By this definition, Prikk requirements are ready to proceed to deeper external design and FDD approval.

---

## 11. Open Items Deferred to External Design / FDDs

The following must not churn app requirements further unless they change product scope:

- exact canonical serialization byte layout;
- exact WAL record format;
- exact fsync sequence per supported OS;
- exact conflict witness schema;
- exact plugin WIT interface;
- exact trust-store file layout;
- exact ref-state object storage path;
- exact pack/archive format;
- exact remote protocol frames.

These belong in External Design v1.1 and the FDD package.
