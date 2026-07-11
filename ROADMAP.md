# Prikk Roadmap

This repository follows the design-first Prikk roadmap. Change history is tracked in `CHANGELOG.md`;
milestone/status detail is in `rfcs/IMPLEMENTATION-STATUS.md`.

## Current Increment

- **DC-26 - documentation home correction (accepted).** The current documentation-structure increment
  implements the accepted decision to move current-state architecture/concept references from
  `rfcs/fdds/` into the published `docs/src/reference/` book as their authoritative, self-contained
  home. It is documentation-structure only: no code, schema, trust, CLI, or RFC lifecycle policy
  changes.

## Release Candidate Increment

- No active release candidate is selected after 0.16.0.

## Last Released Increment

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

1. **DC-25 merge planning surface (accepted, 0.17.0 candidate)**: the accepted substantive next
   implementation increment adds a read-only `prikk merge-plan` boundary over explicit baseline and
   left/right targets. It remains non-executing and does not add merge-base discovery.
2. **DC-26 documentation home correction (accepted, 0.16.1)**: move current-state architecture/concept
   references from `rfcs/fdds/` into the published `docs/src/reference/` book as their authoritative,
   self-contained home, keeping `rfcs/` for design-process/gating material. This must land **before**
   the documentation reference series below, so those references are not built on the home the
   correction retires. See `rfcs/accepted/DC-26-DOCUMENTATION-HOME-CORRECTION.md`.
3. **Documentation reference series (0.16.1 or later)**: the current-state reference subjects surfaced
   by the DC-24 spec recap — durability/crash-recovery, verify/doctor, patch-algebra concepts, key
   setup, and further layout/safety/policy references. Tracked in the *0.16.1+ Documentation Reference
   Backlog* section below. Graduation homes are pending DC-26 acceptance.
4. Branch copy/fork, branch switching, tags/remotes, rollback refs, conflict/inverse evidence,
   rollback authorization, audit/plugin, key lifecycle, and sync remain gated by
   their dedicated plans and FDDs.

Final feature scope remains governed by the FDDs and RFCs.

## 0.16.0 Release Task Management

These tasks are tracked here because `.git-exclude/tasks/` is scratch space and not a durable backlog.
They remain managed in this section until each completion condition is met.

| ID | Owner | Status | Trigger / next action | Completion condition |
|---|---|---|---|---|
| TASK-01 docs Phase-2 physical subdirectories | Designer, then architect reviewer | Done | Reviewed in `.git-exclude/reviewed/prikk-0.16.0-docs-phase2-subdirs-review-v1.md` and committed | Accepted review and committed docs source-tree move |
| TASK-02 consolidated data-model + trust/threat-model docs | Architect + maintainer | Done | Reviewed in `.git-exclude/reviewed/prikk-dc-24-0.16.0-docs-implementation-review-v1.md`, repaired, and committed | DC-24 docs are reviewed and committed |
| TASK-03 docs Pages workflow hardening | Maintainer | Local prep done; external deploy verification pending | After release/docs workflow changes are pushed, run GitHub Actions `workflow_dispatch` or observe the release-triggered Pages run | First Pages build/deploy succeeds, or a tracked follow-up records any GitHub-side failure |
| TASK-04 DC-23 store-unit test carry-forwards | Designer/implementer | Done | Reviewed and committed with the accepted 0.16.0 pre-release hardening bundle | Store-level cross-item test is committed |
| TASK-05 0.16.0 release finalization | Maintainer | Done | Released as tag `0.16.0` and crates published | `0.16.0` tag and crate publish are completed by the maintainer |

## 0.16.1+ Documentation Reference Backlog

Documentation-only, current-state reference increments targeted for **0.16.1 or later**, following
DC-24 (data model + trust/threat). They are grounded through the tracked DC-24 baseline recap
(`rfcs/handoffs/DC-24-data-model-trust-threat-docs/baseline-recap.md`) and current released code/RFCs;
local `.git-exclude/specs/` files are not reviewer-facing authority. Each task must carry source
anchors and mandatory honest-limits caveats when it graduates. Sequencing: Tier 1 first; TASK-06 and
TASK-08 are the largest gaps; TASK-09 rides close behind the DC-24 threat model. These must not creep
into DC-24, and every one must preserve the same honest-limits discipline (unit-test-evidenced
durability, no repository-format stability, `verify` is not a global-trust proof) or it re-creates the
over-trust risk DC-24 exists to prevent.

**Home pending DC-26.** The *Home* targets in the table below reflect the DC-24 `rfcs/fdds/` pattern,
which accepted **DC-26** (documentation home correction) retires in favour of authoritative
`docs/src/reference/` pages. Do not build these references until DC-26 is implemented; during DC-26
implementation, update this table's homes to `docs/src/reference/` in the same pass.

| ID | Tier | Owner | Status | Trigger / next action | Completion condition | Durable home |
|---|---:|---|---|---|---|---|
| TASK-06 durability & crash-recovery reference | 1 | Architect + maintainer | Open | After DC-24 docs are reviewed/committed, draft the storage transaction/durability reference. | Reviewed FDD-02 durability material and mdBook entry are committed. | `rfcs/fdds/FDD-02-STORAGE-TRANSACTION-MODEL.md` + `docs/src/reference/durability-recovery.md` |
| TASK-07 verify & doctor reference | 1 | Architect + maintainer | Open | Coordinate with TASK-06 or start when verify/doctor scope needs public release wording. | Reviewed integrity/recovery docs define what `verify` and `doctor` do and do not prove. | FDD-02 section or `docs/src/reference/integrity-recovery.md` |
| TASK-08 patch algebra & merge-evidence concepts | 1 | Architect + maintainer | Open | Next Tier-1 concept candidate after DC-24; draft current-state FDD-01 and mdBook concept page. | Reviewed FDD-01/current concept page explains commutation, evidence outcomes, and non-goals. | `rfcs/fdds/FDD-01-PATCH-ALGEBRA.md` + `docs/src/reference/patch-algebra.md` |
| TASK-09 key management & signing setup | 1 | Designer/maintainer | Open | After DC-24 trust model lands, write the operator setup guide for current env-var key input and maintainer trust. | Reviewed operator guide is committed and links DC-24/FDD-04 without promising key lifecycle features. | `docs/src/guide/security-setup.md` |
| TASK-10 repository layout & authority model | 2 | Architect + maintainer | Open | Start when FDD-00 needs more layout detail or when format/authority claims expand. | Current directories and authority-vs-cache rules are reviewed and committed. | Extend `rfcs/fdds/FDD-00-DATA-MODEL.md` or add `docs/src/reference/repository-layout.md` |
| TASK-11 path & worktree safety rules | 2 | Architect + maintainer | Open | Start before expanding checkout/worktree docs or when path rejection UX needs public explanation. | Reviewed path/worktree safety reference is committed with current gaps marked. | `docs/src/reference/path-safety.md` |
| TASK-12 concurrency & locking model | 2 | Architect + maintainer | Open | Coordinate with TASK-06, especially stale `active.lock` and CAS behavior. | Reviewed locking/concurrency docs are committed and describe manual stale-lock limits. | FDD-02 section or `docs/src/reference/concurrency-locking.md` |
| TASK-13 release, versioning & compatibility policy | 2 | Maintainer | Open | Start before the next release/docs pass that makes compatibility or repository-format claims. | Reviewed release/compatibility policy is committed and linked from README/CHANGELOG as needed. | `docs/src/reference/release-compatibility.md` or contributing policy page |
| TASK-14 consolidated non-goals / deferred features | 3 | Maintainer/architect | Open | Start when deferred-feature lists begin drifting across README, ROADMAP, mdBook, and release notes. | Reviewed non-goals page is committed and links ROADMAP as the planning authority. | `docs/src/reference/non-goals.md` |
| TASK-15 roles & user-classes orientation | 3 | Designer | Open | Start when the docs need a clearer audience map after the Reference section settles. | Reviewed orientation page or index update is committed. | `docs/src/index.md` or `docs/src/guide/audience.md` |
| TASK-16 error taxonomy & diagnostics | 3 | Implementer/architect | Open | Start with TASK-07 or when diagnostics need user-facing interpretation. | Reviewed diagnostics reference is committed and grounded in `crates/prikk-error`. | `docs/src/reference/errors.md` |

Scratch detail lives in `.git-exclude/tasks/002-update-management/TASK-06..16-*.md` until each
graduates; update the **Status**, **Trigger / next action**, **Completion condition**, and **Durable
home** here as they land. This section is the durable record, not the scratch files.

## Historical Note — PR-030

PR-030 closed the observability gap after rollback drafts are sealed by the existing `seal --allow-no-audit`
path: sealed history labels Blocks that contain rollback-marked Patch objects, and repository verification
counts sealed rollback Blocks and sealed rollback Patch references. It did not introduce rollback-specific
refs, authorize rollback, mutate the worktree, or change seal publication semantics.
