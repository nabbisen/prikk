# DC-24 Baseline Recap - Requirements, NFR, External Design, and v0.2.0 Handoff

Status: Tracked recap for proposed DC-24
Related RFC: `../../accepted/DC-24-DATA-MODEL-TRUST-THREAT-DOCS.md`

## Purpose

This recap makes the untracked baseline inputs reviewable in the tracked repository. The original
inputs live under `.git-exclude/specs/`, which is scratch space and is not globally shared by VCS:

- `prikk-app-requirements-v1.2.md`;
- `prikk-non-functional-requirements-v1.1.md`;
- `prikk-external-design-v1.2.md`;
- `prikk-v0.2.0-handoff-bundle-v0.2.0.tar.gz`.

DC-24 should treat this file as the tracked recap of those inputs. If implementation needs a claim from
the original scratch files that is not captured here, that claim must be added to tracked RFC/FDD docs
before it becomes part of the published data-model or trust/threat documentation.

This recap is not a new source of implementation authority by itself. Current code, released RFCs,
tracked FDD/reference docs, `CHANGELOG.md`, `ROADMAP.md`, and `rfcs/IMPLEMENTATION-STATUS.md` remain
the durable authorities. Where this recap describes older v0.2.0 state, DC-24 must reconcile it with
the later released DC-10 through DC-23 state.

## App Requirements Recap

The app requirements define Prikk as a standalone distributed version control system built around
block-oriented patch theory. Prikk uses a native `.prikk/` repository format and is not a Git wrapper.
Git import or migration may exist as adoption tooling, but Git object compatibility must not constrain
the repository model.

The core product direction includes:

- repository initialization;
- durable signed commits through a WAL-backed Patch object model;
- canonical signed object envelopes;
- sealed Block DAG history;
- branch and tag references through signed RefState objects;
- RefUpdate transition records;
- patch application, inverse, and minimal commutation logic;
- conflict witnesses when algebraic commutation fails;
- safe checkout and worktree materialization;
- integrity verification and conservative recovery diagnostics;
- WASM-only audit plugin host for v1;
- attestation records linked to Blocks and publication policy;
- local peer sync as the first remote foundation.

Ecosystem/adoption work is allowed later: Git import, hosting/forge integration, release packaging,
repository bundles/backups, and large-file policy.

The v1 core excludes:

- native plugin execution inside the core process;
- Git object compatibility;
- OT/CRDT real-time collaboration;
- centralized cloud dependency;
- full forge replacement in the local CLI core;
- mandatory advanced PKI, key revocation, or hardware signing;
- semantic language-aware merge beyond defined patch algebra.

The required user classes are developer, maintainer, security/compliance reviewer, repository
administrator, and tooling/integration developer. The public docs should therefore distinguish operator
how-to material from maintainer/security-reviewer reference material.

The core concepts to preserve in documentation are:

- Patch: atomic logical change with ordered operations, preconditions, intent metadata, and AUTHOR
  signature; identity derives from canonical unsigned payload, not signatures.
- Operation: one ordered change within a Patch, ordered by explicit `op_seq`.
- Block: sealed immutable collection of Patches and scalability boundary for long-lived history.
- RefState: signed state for a branch or tag reference; ref files are pointers, not roots of trust.
- RefUpdate: append-only evidence for a ref transition.
- Attestation: policy/audit result targeting a Block; not part of Block identity.
- Seal: transition from active signed Patches to a Block plus publication policy/ref movement.

The requirements also establish release discipline: design documents gate identity-bearing and
security-sensitive implementation; docs must match behavior; and future docs must not imply a stable
repository format or production readiness before those guarantees exist.

## Non-Functional Requirements Recap

The NFR baseline treats Prikk as an append-only durability system, patch-algebra engine, and
security-sensitive publication system. Its documentation must therefore be more exact than ordinary CLI
documentation.

The FDD traceability model maps:

- FDD-01 to patch correctness, apply/inverse, commutation, and conflict witnesses;
- FDD-02 to WAL durability, crash recovery, refs, ref logs, verification, and doctor UX;
- FDD-03 to canonical object identity, deterministic encoding, schema validation, and path safety;
- FDD-04 to threat coverage, signature replay, malformed input, path safety, plugin escape, and trust;
- FDD-05 to plugin sandboxing, audit capability boundaries, and attestations.

Correctness and integrity requirements include deterministic object identity, non-circular signatures,
validation before use, verifiable ref state, and corruption detection. Durability requirements include
commit durability after WAL fsync, safe WAL recovery, seal atomicity, durable ref updates, actionable
doctor output, and manual-repair posture when automatic repair is unsafe.

Patch-algebra requirements include explicit operation order, enforced preconditions, inverse
correctness for supported primitives, commutation only when proof conditions hold, and first-class
conflict witnesses.

Security requirements include safe defaults, role-bound signatures, path safety, malformed input
handling without panics or corruption, bounded plugin resources, and downgrade resistance for
publication policy/attestation history.

Performance and reliability requirements include bounded commit latency, active-block limits, merge
complexity scoped to active blocks and sealed summaries, rebuildable caches that are never roots of
trust, no silent data loss, conservative GC, verifiable backup/restore, and typed actionable errors.

Portability and maintainability requirements include Linux/macOS/Windows as design targets, UTF-8
repository paths, cross-platform reserved-name handling, case-collision policy, `unsafe_code =
forbid`, separated module boundaries, test-first critical paths, and documented sign-off for spec
drift.

For DC-24, the important consequence is that data-model and threat-model docs must carry explicit
verification boundaries and evidence expectations, not just conceptual prose.

## External Design Recap

The external design establishes these architectural principles:

- native `.prikk/` repository format;
- object identity before signatures;
- WAL as the active truth;
- Blocks as scalability boundaries;
- refs as signed state objects, not trusted mutable files;
- audit as publication evidence separate from Block identity;
- WASM-only plugins in v1.

The external design's layer diagram includes `prikk-cli`, `prikk-core`, `prikk-object`, `prikk-store`,
`prikk-patch`, `prikk-worktree`, `prikk-path`, `prikk-crypto`, `prikk-plugin-host`,
`prikk-audit-api`, and `prikk-net`. The current implementation does not use that exact crate split,
so DC-24 must describe the current implementation honestly while preserving the intended boundary
concepts: CLI formats commands, store owns repository integration, object owns identity and schemas,
crypto owns role-bound signatures, replay/lifecycle remains separate from store integration, and
plugins/sync remain deferred.

The repository layout concept includes `.prikk/FORMAT`, config, trust store, object directories,
active sessions, refs, ref logs, indexes, cache, quarantine, and GC. Durable authorities are
content-addressed objects, signed RefStates, signed RefUpdates, WAL entries, and trusted config.
Indexes and caches are rebuildable.

The object model includes ObjectEnvelope, Patch, Block, RefState, RefUpdate, Tag, Attestation, and Blob.
Object reachability roots include current ref-state pointers, reachable Block DAGs, Block Patch IDs,
attestations referenced by publication policy, tag refs, quarantine roots, and backup/export manifests.
DC-24 must separate current implemented object types and CLI support from future v1 object/design
intent.

Ref design requires flat pointer files, content-addressed RefState objects, append-only signed
RefUpdate logs, compare-and-swap semantics, and ref-specific locks. WAL design requires exact signed
Patch envelopes and recovery without re-signing. Seal design requires persisting Patches before Blocks,
policy/attestation hooks when configured, signed RefState/RefUpdate publication, and crash semantics
that recover to the old or new valid state.

Patch algebra design includes state space, ordered Patch operations, operation kinds, preconditions,
commutation, and conflict witnesses. Worktree/path design requires UTF-8 repo-relative paths,
rejection of traversal/absolute/reserved/unsafe paths, symlink escape protection, and cautious checkout
materialization. Plugin/audit design keeps plugins WASM-only with explicit capabilities and attestations
targeting Blocks rather than defining Block identity.

Trust/signature design covers Ed25519, role separation, signature binding to object type, object ID,
role, key ID, and algorithm, plus local trust policy. The original design allowed TOFU concepts, but
later released DC-11 chose explicit local maintainer trust configuration for the current implementation.
DC-24 must document the released behavior rather than reviving older or broader design intent.

Command flows in the external design include init, commit, status, seal, verify, doctor, checkout, and
merge. Current docs must distinguish implemented command behavior from future merge/plugin/sync flows.

Data lifecycle states are active, sealed, quarantined, archived, and backup/export. Schema evolution is
pre-1.0 and must not promise stable migration beyond current release notes. Remote/hosting design is
deferred beyond local peer sync foundations.

## v0.2.0 Handoff Bundle Recap

The v0.2.0 bundle was a whole-project snapshot, not a lifecycle authority. It explicitly says the
durable lifecycle authority remains `rfcs/`, `ROADMAP.md`, and `CHANGELOG.md`.

At v0.2.0, the project had a usable local core: `init -> commit -> seal -> log -> verify`, with
node-addressed commits, real role-bound Ed25519 AUTHOR signatures for the commit path, and fail-closed
corruption/concurrency edges. At that time, patch commutation/merge algebra, plugin/audit execution,
remote sync, trust-store/key management, MAINTAINER publication signing, and repository-format
stability remained out of scope.

The v0.2.0 handoff recorded several risks. Some are now closed by later releases and must not be
treated as still-current:

- rollback-draft fake AUTHOR marker was addressed by DC-10;
- MAINTAINER publication signing and minimal trust store were addressed by DC-11;
- non-default ref genesis was addressed by DC-13;
- active-session integrity was hardened by DC-15;
- patch algebra and merge evidence progressed through DC-16 through DC-23.

Some risks remain relevant to DC-24:

- repository format is still not stable;
- symlink authoring and full path/platform edge coverage are incomplete;
- large-file/streaming behavior remains deferred;
- plugin/audit and sync remain deferred;
- key lifecycle beyond the minimal local trust store remains deferred;
- release and security checks are still partly manual.

The handoff's decision log captures decisions DC-24 should preserve or consciously reconcile:

- design leads code for identity-bearing bytes;
- PATCH-framing identity anchors are frozen regression checks;
- node-addressed baselines come from authoritative replay, not snapshots/caches;
- zero-operation Patches are non-representable;
- signatures are role-bound and non-circular;
- genesis must never silently recreate history;
- active-session lock discipline is load-bearing;
- errata should integrate into document bodies, not live as drifting overlays;
- release deliverables must be auditable and superseded rather than mutated;
- Rust safety and lint standards are project policy.

The release/security recap emphasizes pre-1.0 semver, versioned tarballs, recorded checksums, green gate
logs, docs matching behavior, threat-model updates when data flows or auth logic change, no real
secrets in the repo/CI/release archives, role-bound signature security, dependency caution, and
superseding bad releases rather than mutating shipped artifacts.

## Consequences for DC-24

DC-24 should use this recap to design the durable documentation surface, then reconcile every public
claim against current tracked implementation state.

The data-model reference must identify which external-design concepts are implemented now, which are
future v1 intent, and which were superseded by later DCs.

The trust/threat reference must emphasize released behavior: explicit local maintainer trust
configuration, role-bound signatures, current verification boundaries, no repository-wide AUTHOR trust
policy, and no key rotation/revocation/threshold/hardware/remote trust support.

The mdBook pages must be short entry points that link to tracked FDD/RFC references. They must not
depend on `.git-exclude/specs/` being available to readers or reviewers.
