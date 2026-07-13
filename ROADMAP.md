# Prikk Roadmap

This repository follows the design-first Prikk roadmap. Change history is tracked in `CHANGELOG.md`;
milestone/status detail is in `rfcs/IMPLEMENTATION-STATUS.md`.

## Current Increment

- No active design/implementation increment is selected after the 0.17.5 release.

## Release Candidate Increment

- No active release candidate is selected after the 0.17.5 release.

## Last Released Increment

- **DC-31 - repository layout and authority reference (released as 0.17.5).** The release adds a
  current-state mdBook reference for the initialized `.prikk/` layout, `.prikk/FORMAT`, object/ref/
  active/trust paths, and authority-vs-pointer/cache boundaries. It is documentation-only and does not
  change code, schema, CLI behavior, repository behavior, trust policy, verification, repair, or
  repository-format stability claims.

- **DC-30 - key management and signing setup guide (released as 0.17.4).** The release adds a
  current-state operator guide for AUTHOR/MAINTAINER signing setup, environment key inputs,
  repository-local maintainer trust configuration, seed-handling foot-guns, missing key-lifecycle
  features, and the absence of built-in key-generation/public-key-derivation commands. It is
  documentation-only and does not change code, schema, CLI behavior, repository behavior, trust
  policy, signing behavior, verify, or seal behavior.

- **DC-29 - verify and doctor integrity/recovery reference (released as 0.17.3).** The release adds a
  current-state mdBook reference for what `prikk verify` checks and does not prove, how `prikk doctor`
  interprets verification results, current doctor issue codes/severities, rollback-verification
  relationship, and narrow repair boundaries. It is documentation-only and does not change code,
  schema, CLI behavior, repository behavior, verify, doctor, trust, or repair behavior.

- **DC-28 - durability and crash-recovery reference (released as 0.17.2).** The release adds a
  current-state mdBook reference for active-WAL durability, WAL replay/tail handling, seal publication
  flow, ref recovery, doctor repair limits, stale-lock limits, and deferred crash/platform evidence.
  It is documentation-only and does not change code, schema, CLI behavior, repository behavior, WAL,
  refs, seal, verify, doctor, or release semantics.

- **DC-27 - patch algebra and merge-evidence concepts reference (released as 0.17.1).** The release
  adds an authoritative current-state mdBook reference for patch algebra, commutation/confluence,
  merge-evidence outcomes, reason-code/proof-phase vocabulary, and merge-plan status mapping. It is
  documentation-only and does not add merge execution, merge-base discovery, command behavior, schema
  changes, persisted proof/witness objects, JSON output, or public Rust API stabilization.

- **DC-25 - merge planning surface (released as 0.17.0).** `prikk merge-plan` exposes a read-only
  planning classification over the existing explicit-input merge evidence path, preserving evidence
  outcome/reason while adding `ConfluentSubset` / `Blocked*` status and action text. The release also
  removes the temporary 0.16.1 FDD-00/FDD-04 compatibility pointers after the DC-26 documentation-home
  transition window. It does not execute merges, infer merge bases, publish branches, write refs/WAL,
  create merge commits, persist plan/proof/witness/evidence objects, change schema, or stabilize
  `prikk-replay` as a public API.

- **DC-26 - documentation home correction (released as 0.16.1).** Current-state architecture/concept
  references now live in the published `docs/src/reference/` book as their authoritative,
  self-contained home. FDD-00/FDD-04 remained as temporary compatibility pointers through 0.16.1 and
  were removed in 0.17.0. This release is documentation-structure only: no code, schema, trust, CLI,
  or RFC lifecycle policy changes.

- **DC-23 plus DC-24 documentation work (released as 0.16.0).** `prikk merge-evidence` text output is
  stabilized with clearer selector summaries, unambiguous cross-side item display, displayed/total item
  counts, and report-level output cleanup. The release also reorganizes the mdBook source tree, adds
  the merge-evidence command page, prepares GitHub Pages publication, and adds current-state FDD/mdBook
  references for Prikk's data model and trust/threat model. It does not add merge execution,
  merge-base discovery, branch publication, merge commits, persisted evidence objects, display-path
  filtering, scoped/path-limited merge analysis, JSON output, schema changes, trust-store enforcement
  changes, or public `prikk-replay` API stabilization.

- **DC-22 - public merge evidence UX boundary (released as 0.15.0).** `prikk merge-evidence` exposes
  DC-21 merge/conflict evidence through a read-only public UX with explicit baseline and candidate
  targets. It does not infer merge bases, execute merges, publish merge commits, write refs or WAL,
  materialize worktree conflicts, persist proof/witness objects, change schema, extract patch algebra,
  or stabilize `prikk-replay` as an external API.

- **DC-21 - merge conflict evidence contract (released as 0.14.0).** Internal, read-only evidence
  report vocabulary and adapters now sit over existing pair commutation and flat confluence analysis.
  Reports are non-mutating and privacy-preserving, with explicit `EvidenceFailure`,
  `InvalidCandidate`, `Unsupported`, `Deferred`, `Conflict`, `OrderedDependency`, `NotConfluent`, and
  `Confluent` outcomes. No merge execution, CLI merge, persisted proof/witness objects, schema
  changes, worktree conflict materialization, patch-algebra extraction, or public `prikk-replay` API
  stabilization was added.

- **DC-20 - replay boundary stabilization (released as 0.13.0).** `prikk-replay` remains internally
  scoped and non-stable as an external Rust API, `prikk-store` remains the repository integration
  crate, and filesystem root joining stays store-owned while `RepoPath` remains lexical in
  `prikk-replay`. No CLI,
  schema, repository-layout, public API, patch-algebra extraction, text-span extraction, resolver,
  cache-persistence, worktree, merge, confluence, or conflict surface was added.

- **DC-19 - replay/lifecycle crate boundary (released as 0.12.0).** `prikk-replay` now owns the
  workspace-internal lifecycle substrate and lexical repository path type needed by lifecycle state,
  while `prikk-store` remains the repository integration crate through compatibility wrappers. No CLI,
  schema, merge, public confluence, patch-algebra extraction, text-span extraction, worktree
  extraction, or storage/cache/ref/WAL ownership changes were included.

- **DC-18 - patch algebra commutation and confluence contract (released as 0.11.0).** Internal
  commutation now requires classifier independence plus replay-both-orders proof, and flat
  two-sequence confluence requires individual replay-validity, commuting cross-pairs, composed replay,
  and final lifecycle-state equality. Required sealed candidate evidence failures, including
  replacement blob evidence, remain outer evidence errors and are not hidden by algebraic `Unknown`.
  No CLI, schema, merge execution, persisted witness/proof, public confluence API, or production merge
  surface was added.

- **DC-17 - patch algebra evidence contract (released as 0.10.0).** Internal pair classification now
  uses a scoped evidence contract and store-backed resolver boundary so required sealed evidence
  failures are distinguishable from ordinary unsupported algebra, while optional unsealed candidate
  evidence still fails closed as `Unknown`. No CLI, schema, merge execution, persisted witness, or
  production confluence surface was added.

- **DC-16 - patch algebra foundation (released as 0.9.0).** Internal pair classification now covers
  `Independent`, `OrderedDependency`, `Conflict`, and `Unknown`, with baseline preimage validation,
  structured path effects, fail-closed insufficient-evidence handling, and oracle-backed vectors. No CLI,
  schema, merge execution, persisted witness, or production confluence surface was added.

- **DC-15 - active-session integrity and verification hardening (released as 0.8.0).** `verify` and
  `doctor` report active-WAL metadata integrity explicitly, rollback-draft append re-checks target tip
  freshness under the active lock, ref publication validates `heads/*` at the lower-level boundary, and
  signature key-id validation is shared across AUTHOR, MAINTAINER, and trust-policy paths.

- **DC-14 - arbitrary-span text direct inverse and rollback exposure (released as 0.7.0).** The existing
  inverse/rollback surfaces support arbitrary-span `EditText` by recomputing direct inverse identity
  against the post-forward text.

- **DC-13 - non-default ref genesis (released as 0.6.0).** First-commit genesis on explicit
  non-default branch refs is implemented with branch-ref validation, active-WAL ref ownership, and
  `seal --ref` publication rules.

- **DC-12 - arbitrary-span text edits (released as 0.5.0).** Worktree text edits are authored and
  replayed as deterministic arbitrary spans while keeping inverse/rollback, commutation, confluence,
  and conflict witnesses deferred.

- **DC-11 - publication signing and minimal trust store (released as 0.4.0).** Publication objects carry
  real role-bound Ed25519 MAINTAINER signatures verified against a minimal repository-local trust
  policy.

- **DC-10 - rollback-draft identity and AUTHOR signing (released as 0.3.0).** Rollback-draft identity is
  `PatchPurpose::RollbackDraft`, not a reserved AUTHOR key id, and rollback drafts carry real role-bound
  Ed25519 AUTHOR signatures. This closed the broad claim for AUTHOR-role Patch signatures produced by
  production commands.

## Next Increments

1. **Remaining Tier-1 documentation reference series (after DC-27 unless reprioritized)**:
   durability/crash-recovery, verify/doctor, and key setup remain the next major current-state
   reference gaps. Tracked in the *Post-0.16.1 Documentation Reference Backlog* section below.
   Graduation homes follow the DC-26 `docs/src/reference/` authority model.
2. Branch copy/fork, branch switching, tags/remotes, rollback refs, conflict/inverse evidence,
   rollback authorization, audit/plugin, key lifecycle, and sync remain gated by
   their dedicated plans and FDDs.

Final feature scope remains governed by accepted RFCs, genuine gating FDDs when present, and the
current-state reference docs.

## 0.16.0 Release Task Management

These tasks are tracked here because the original task notes are local scratch space and not a durable
backlog.
They remain managed in this section until each completion condition is met.

| ID | Owner | Status | Trigger / next action | Completion condition |
|---|---|---|---|---|
| TASK-01 docs Phase-2 physical subdirectories | Designer, then architect reviewer | Done | Reviewed and committed | Accepted review and committed docs source-tree move |
| TASK-02 consolidated data-model + trust/threat-model docs | Architect + maintainer | Done | Reviewed, repaired, and committed | DC-24 docs are reviewed and committed |
| TASK-03 docs Pages workflow hardening | Maintainer | Local prep done; external deploy verification pending | After release/docs workflow changes are pushed, run GitHub Actions `workflow_dispatch` or observe the release-triggered Pages run | First Pages build/deploy succeeds, or a tracked follow-up records any GitHub-side failure |
| TASK-04 DC-23 store-unit test carry-forwards | Designer/implementer | Done | Reviewed and committed with the accepted 0.16.0 pre-release hardening bundle | Store-level cross-item test is committed |
| TASK-05 0.16.0 release finalization | Maintainer | Done | Released as tag `0.16.0` and crates published | `0.16.0` tag and crate publish are completed by the maintainer |

## Post-0.16.1 Documentation Reference Backlog

Documentation-only, current-state reference increments targeted for after **0.16.1**, following
DC-24 (data model + trust/threat). They are grounded through the tracked DC-24 baseline recap
(`rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md`) and current released code/RFCs;
local scratch specs are not reviewer-facing authority. Each task must carry source
anchors and mandatory honest-limits caveats when it graduates. Sequencing: Tier 1 first; TASK-06 and
TASK-08 are the largest gaps; TASK-09 rides close behind the DC-24 threat model. These must not creep
into DC-24, and every one must preserve the same honest-limits discipline (unit-test-evidenced
durability, no repository-format stability, `verify` is not a global-trust proof) or it re-creates the
over-trust risk DC-24 exists to prevent.

**Documentation home.** Released **DC-26** retires the DC-24 `rfcs/fdds/` pattern for current-state
references. The durable homes below are authoritative `docs/src/reference/` or `docs/src/guide/` pages;
`rfcs/fdds/` is reserved for future gating FDDs only.

| ID | Tier | Owner | Status | Trigger / next action | Completion condition | Durable home |
|---|---:|---|---|---|---|---|
| TASK-06 durability & crash-recovery reference | 1 | Architect + maintainer | Released in 0.17.2 | Complete; use the reference as the current public durability/recovery baseline. | Reviewed durability/crash-recovery reference is committed. | `docs/src/reference/durability-recovery.md` |
| TASK-07 verify & doctor reference | 1 | Architect + maintainer | Released in 0.17.3 | Complete; use the reference as the current public verify/doctor diagnostic baseline. | Reviewed integrity/recovery docs define what `verify` and `doctor` do and do not prove. | `docs/src/reference/integrity-recovery.md` |
| TASK-08 patch algebra & merge-evidence concepts | 1 | Architect + maintainer | Released in 0.17.1 | Complete; use the reference as the current public concept baseline. | Reviewed current concept page explains commutation, evidence outcomes, and non-goals. | `docs/src/reference/patch-algebra.md` |
| TASK-09 key management & signing setup | 1 | Designer/maintainer | Released in 0.17.4 | Complete; use the guide as the current public signing setup baseline. | Reviewed operator guide is committed and links the trust/threat reference without promising key lifecycle features. | `docs/src/guide/security-setup.md` |
| TASK-10 repository layout & authority model | 2 | Architect + maintainer | Released in 0.17.5 | Complete; use the reference as the current public repository-layout/authority baseline. | Current directories and authority-vs-cache rules are reviewed and committed. | `docs/src/reference/repository-layout.md` |
| TASK-11 path & worktree safety rules | 2 | Architect + maintainer | Open | Start before expanding checkout/worktree docs or when path rejection UX needs public explanation. | Reviewed path/worktree safety reference is committed with current gaps marked. | `docs/src/reference/path-safety.md` |
| TASK-12 concurrency & locking model | 2 | Architect + maintainer | Open | Coordinate with TASK-06, especially stale `active.lock` and CAS behavior. | Reviewed locking/concurrency docs are committed and describe manual stale-lock limits. | `docs/src/reference/concurrency-locking.md` |
| TASK-13 release, versioning & compatibility policy | 2 | Maintainer | Open | Start before the next release/docs pass that makes compatibility or repository-format claims. | Reviewed release/compatibility policy is committed and linked from README/CHANGELOG as needed. | `docs/src/reference/release-compatibility.md` or contributing policy page |
| TASK-14 consolidated non-goals / deferred features | 3 | Maintainer/architect | Open | Start when deferred-feature lists begin drifting across README, ROADMAP, mdBook, and release notes. | Reviewed non-goals page is committed and links ROADMAP as the planning authority. | `docs/src/reference/non-goals.md` |
| TASK-15 roles & user-classes orientation | 3 | Designer | Open | Start when the docs need a clearer audience map after the Reference section settles. | Reviewed orientation page or index update is committed. | `docs/src/index.md` or `docs/src/guide/audience.md` |
| TASK-16 error taxonomy & diagnostics | 3 | Implementer/architect | Open | Start with TASK-07 or when diagnostics need user-facing interpretation. | Reviewed diagnostics reference is committed and grounded in `crates/prikk-error`. | `docs/src/reference/errors.md` |

Scratch detail may exist locally until each task graduates; update the **Status**, **Trigger / next
action**, **Completion condition**, and **Durable home** here as they land. This section is the durable
record, not the scratch files.

## Historical Note — PR-030

PR-030 closed the observability gap after rollback drafts are sealed by the existing `seal --allow-no-audit`
path: sealed history labels Blocks that contain rollback-marked Patch objects, and repository verification
counts sealed rollback Blocks and sealed rollback Patch references. It did not introduce rollback-specific
refs, authorize rollback, mutate the worktree, or change seal publication semantics.
