# Prikk Roadmap

This repository follows the design-first Prikk roadmap. Change history is tracked in `CHANGELOG.md`;
the corrective release sequence is in `MILESTONES.md`, and current-state detail is in
`rfcs/IMPLEMENTATION-STATUS.md`.

## Current Increment

- **DC-45 - release policy tooling consolidation (accepted).** Architect design repair re-review v1
  accepted the executable oracle, schema-profile, Cargo-boundary, and cutover/rollback contracts on
  2026-07-16. Duplicate-name profile hardening was accepted and committed as profile-contract identity
  `ea427df`. The separately scoped Python observation adapter was reviewed; its
  review v1 required independent final projection and a final-only negative check. The repair and
  top-level identity self-test were accepted after implementation repair re-review v1 on 2026-07-16;
  the adapter was committed as `6be65af`. Its per-case comparison has zero mismatches across 145
  baseline cases and adds only the nine accepted profile cases. Oracle implementation review v1 found
  five blocking closure/contract defects. The repaired 154-case exact-byte oracle now materializes
  release-state governance dependencies, binds fixture/oracle identifiers and two-snapshot sequences,
  and enforces exact reason and coverage contracts. Architect repair re-review v1 accepted the freeze
  semantics on 2026-07-17, but project-owner acceptance is withheld because the candidate adds 247
  files, including 237 per-case vector files. Architect footprint QA conditionally approved a
  three-pack direction, and architect design amendment re-review v1 accepted the explicit decoding,
  location, closure, and archive contract on 2026-07-17. The untracked compact implementation now has
  exactly ten root artifacts and three packs, preserves all 237 logical vectors, and awaits separate
  implementation re-review and owner acceptance. Implementation re-review v1 found one blocking raw
  dot-segment grammar defect; the repaired candidate and focused end-to-end negatives await repair
  re-review. Architect repair re-review v1 accepted the repair with no findings on 2026-07-17. Explicit
  project-owner acceptance of the exact 13-file inventory is still required before the isolated freeze
  commit. Architect design repair re-review v1 accepted the explicit compact-oracle retirement schedule
  on 2026-07-17, satisfying the lifecycle-design condition for the owner's separate decision. The five
  Python authoring/verification files remain through the first Rust-gated 0.19.0 release. The first
  later release-candidate increment is blocked until an architect accepts a later-commit stability
  rerun; the following release-candidate increment is blocked until an exhaustive five-file
  decommissioning review removes each file or records an individual owner-approved, event-bound
  exception. Rust must replace the complete accepted manifest verifier and self-test matrix, not only
  the differential-disagreement test. The other eight frozen contract/evidence files remain until a
  later equivalence-backed replacement/consolidation review or an explicit final-retirement review
  closes migration and rollback needs. The project owner committed the exact 13-file oracle with the
  reviewed design/status update as stage-1 freeze commit `47aec9c` on 2026-07-17. Two deterministic
  archives matched, checkout and extracted-archive verification/self-test passed all 154 cases, all 19
  manifest-bound direct dependencies and seven required checkout/archive identities matched, and all
  seven product package listings excluded oracle/tool paths. Architect post-commit evidence review v1
  accepted the isolated freeze and this evidence on 2026-07-17. The stage-2 candidate now adds the
  isolated unpublished Rust tool, mature offline Draft 2020-12 validation, independent typed policy
  evaluation, the complete 154-case oracle and negative matrix, differential disagreement detection,
  Cargo/package boundaries, and command-reference/publication inventories. The first repair re-review
  accepted Pages parity, complete self-test replacement, and independent invariants. Repair re-review
  v2 then accepted consumed-byte input identity but retained command closure over quoted comments,
  long Python option sequences, and malformed executable text. The third repair candidate closes
  those parser surfaces. Repair re-review v3 retained empty-quoted-word and dynamic Cargo authority
  gaps. The fourth repair candidate tracks shell word start independently and rejects dynamic Cargo
  executables/subcommands plus Cargo-less Rust-policy shapes under an explicit literal-inventory
  authority boundary. Repair re-review v4 closed reference scanning but retained generic dynamic
  executable-plus-phase publication. The fifth repair candidate rejects dynamic command heads by
  structural command position, independent of variable names. Repair re-review v5 retained inline
  YAML sequence and wrapper-option-operand gaps. The sixth repair candidate replaces permissive prefix
  skipping with a closed YAML/assignment/`env`/`command` grammar. Repair re-review v6 accepted those
  cases but retained arbitrary exec-wrapper indirection. The seventh repair candidate adds a
  fail-closed default for unrecognized literal heads carrying dynamic arguments, with only an explicit
  inert set exempted. Repair re-review v7 accepted that wrapper-class closure but retained backtick
  command substitution and opaque shell-string execution. The eighth repair candidate fails closed on
  executable backticks, shell `-c`, and `eval` command strings. Repair re-review v8 closed backticks
  but retained the general opaque-string class. The ninth repair candidate structurally extracts YAML
  `run:` scripts and applies a default-closed governed-command allowlist, closing unknown wrappers and
  interpreters as a class. Repair re-review v9 accepted the default-closed model but found sequence
  whitespace and non-first flow-key extraction gaps. The tenth repair candidate hardens both forms and
  fails closed on malformed recognized command mappings. Repair re-review v10 closed those B7.3 gaps
  but found that equivalent quoted or whitespace-separated block-mapping `run` keys were silently
  omitted. The eleventh repair normalizes block and flow keys through the same quote/whitespace-aware
  path, rejects recognized empty command values, and preserves the non-command `defaults.run` mapping
  used by GitHub Actions. Architect repair re-review v11 accepted B7.4 and authorized the isolated
  stage-2 implementation commit on 2026-07-17. The frozen dependency set remains unchanged under an
  explicit fail-closed extractor decision; adding a YAML dependency would require a separate reviewed
  dependency and lockfile re-freeze. Before Python retirement, follow-up controls must validate the
  allowed nested keys under `defaults.run`, treat extractor/allowlist changes as policy changes, and
  close responsibility-map executable correspondence. The accepted stage-2 implementation was
  committed as `6a65a35` on 2026-07-21. Two deterministic archives match; isolated checkout and
  extracted-archive Python/Rust, differential, boundary, and reference checks pass; source trees and
  frozen identities match; and all seven product package listings exclude tool/oracle paths. This
  post-commit evidence was accepted after architect review v1 on 2026-07-21. The next authorized work
  is preparation of an isolated authoritative-command cutover candidate and disposable rollback
  rehearsal for separate review. Preparation found that the accepted stale-reference gate hardcodes
  the Python primary executable and required live command, so the RFC-permitted inventory/reference-
  only Rust cutover cannot pass without changing Rust tool source. Architect QA v1 confirmed the
  mismatch and required a separate two-state repair with exact immutable Python/Rust path-command
  descriptors. The repair enforces exact descriptor selection, regular anchors, one selected live
  registration at each required path, and mixed/unknown-state rejection. Architect implementation
  review v1 accepted it, and it was committed as `2bfb7cc` on 2026-07-21. Deterministic archive,
  isolated checkout/extraction, authority behavior, identity, and product-package preservation
  evidence was accepted after architect post-commit evidence review v1 on 2026-07-21. Preparation of
  the isolated inventory/live-reference cutover candidate and disposable rollback rehearsal was then
  authorized. The exact four-file cutover was committed as `6a8e365`; deterministic archive, clean
  checkout/extraction, full gate, and committed-identity rollback evidence was accepted after final
  architect ruling v1 on 2026-07-21. The Rust command is governance-authoritative. Python and the
  frozen oracle remain required through the first Rust-gated 0.19.0 release and an accepted later-
  commit stability rerun.
  Design acceptance satisfies only the pre-bootstrap planning dependency: no signer bootstrap or
  release action is authorized by DC-45. The required Rust cutover is accepted, but it is not release
  authority. DC-46 separately tracks the pre-existing mismatch between the declared Rust 1.85 minimum
  and the locked product workspace. Architect design rereview v1 accepted restoration of Rust 1.85
  through three bounded source rewrites, focused trust regressions, and pinned locked CI gates on
  2026-07-21. The bounded implementation candidate exposed a conflict with the accepted DC-45
  procedure grammar. Architect command-grammar amendment QA v1 authorized five exact ordinary-Cargo
  vectors and existing scanner tests. Architect implementation review v1 accepted the complete
  candidate on 2026-07-21; the owner commit and post-commit checkout/archive evidence remain pending.
  Before the 0.19.0 release candidate, deliberately reconcile the public `--all-features` Clippy gate
  with the governed classifier's accepted CI vector; do not discover that divergence in a later
  workflow edit. DC-46 must close, or be explicitly
  rescheduled through reviewed roadmap and release-contract changes, before the 0.19.0 release
  candidate.
- **M1 remains active.** DC-35 implementation is accepted and committed, but the signer allowlist
  remains empty and fail-closed. DC-39 remains proposed, DC-40 remains accepted with implementation
  evidence pending, and the combined 0.18.0/M1 release gate remains closed.

## Release Candidate Increment

- No active release candidate is selected after the 0.17.7 release.

## Last Released Increment

- **DC-33 - concurrency and locking reference (released as 0.17.7).** The release adds a current-state
  mdBook reference for active-session locking, ref-specific publication locks, compare-and-swap
  behavior, narrow ref repair locking, and manual stale-lock limits. It is documentation-only and does
  not change code, schema, CLI behavior, lock behavior, repository behavior, verification, doctor,
  trust, release semantics, or repository-format stability claims.

- **DC-32 - path and worktree safety reference (released as 0.17.6).** The release adds a
  current-state mdBook reference for repository path validation, checkout/worktree materialization
  safety, worktree authoring safety, and deferred path/platform gaps. It is documentation-only and
  does not change code, schema, CLI behavior, checkout behavior, materialization behavior, worktree
  authoring behavior, repository behavior, or repository-format stability claims.

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

1. **M0 architecture ratification:** DC-34 decides publication and signature identity authority. It is
   a no-release design gate.
2. **M1 corrective storage and identity baseline (target 0.18.0):** DC-35 through DC-40 close the five
   blocking architecture findings together. No intermediate feature/docs release is planned.
3. **M2 assurance and distribution baseline (target 0.19.0):** DC-45 is the first tooling increment;
   DC-41 through DC-43 add adversarial evidence, performance/maintainability gates, and release security
   controls after corrected behavior exists. DC-45 cutover is required before the release candidate.
   DC-46 then resolves the separately tracked Rust 1.85 workspace compatibility mismatch before that
   candidate unless an architect-reviewed schedule and compatibility-contract amendment says otherwise.
4. **M3 migration and recoverable backup (release target unassigned):** DC-44 owns NFR-REL-03,
   verifiable backup/restore, and migration exercises that are explicitly outside DC-40 and M2.
5. Branch copy/fork, branch switching, tags/remotes, rollback refs, conflict/inverse evidence,
   rollback authorization, audit/plugin, key lifecycle, sync, and unrelated documentation themes remain
   frozen through M1.

Final feature scope remains governed by accepted RFCs, genuine gating FDDs when present, and the
current-state reference docs.

## Corrective Program After 0.17.7

The independent architecture review of 0.17.7 found a critical ref-publication interruption state and
high-severity gaps in state-root identity, required directory durability, existing-object validation,
and signature-preimage authority. The tracked finding-to-RFC matrix, dependencies, target releases, and
completion gates are in `MILESTONES.md`.

The project remains an experimental architecture implementation. Successful routine gates do not
override reproduced negative evidence. Public-preview readiness remains no-go through M2 and requires
a new independent review after M2. Production suitability remains no-go through M3 and likewise
requires an explicit independent ruling. Repository-format stability is not granted by any scheduled
milestone; it requires a separate future stability RFC and review.

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
| TASK-11 path & worktree safety rules | 2 | Architect + maintainer | Released in 0.17.6 | Complete; use the reference as the current public path/worktree safety baseline. | Reviewed path/worktree safety reference is committed with current gaps marked. | `docs/src/reference/path-safety.md` |
| TASK-12 concurrency & locking model | 2 | Architect + maintainer | Released in 0.17.7 | Complete; use the reference as the current public concurrency/locking baseline. | Reviewed locking/concurrency docs are committed and describe manual stale-lock limits. | `docs/src/reference/concurrency-locking.md` |
| TASK-13 release, versioning & compatibility policy | 2 | Maintainer | DC-35 implementation committed and DC-45 design accepted on 2026-07-16; signer set empty | Bootstrap the first signer only through its separate reviewed governance transaction before the 0.18.0 release candidate; completed Rust migration is not a bootstrap prerequisite. | Reviewed release/compatibility policy is committed, strict fixtures and DC-45 direction are accepted, and signer bootstrap remains an explicit release prerequisite. | `docs/src/reference/release-compatibility.md` |
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
