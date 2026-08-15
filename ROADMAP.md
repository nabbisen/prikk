# Prikk Roadmap

This repository follows the design-first Prikk roadmap. Change history is tracked in `CHANGELOG.md`;
the corrective release sequence is in `MILESTONES.md`, current-state detail is in
and `rfcs/IMPLEMENTATION-STATUS.md`. Review findings live in the review result that raised them; anything
that must outlive a review is documented in `docs/` or `rfcs/`.

Development priority and release readiness are separate planning lanes. The project owner selects the
active development theme by product value. Release readiness remains dormant until the project owner
explicitly activates preparation for a named version; implementation completion, a listed release gate,
or the word "next" does not activate signer, hold, RC, tag, or publication work.

Activation requires a reviewed tracked commit that atomically records `active` plus the exact version in
this file, `MILESTONES.md`, and `rfcs/IMPLEMENTATION-STATUS.md`. Until that commit lands, discussion,
review recommendations, roadmap targets, and untracked messages leave the lane parked and cannot trigger
a fingerprint request. Before bootstrap begins, parking or retargeting uses the same reviewed three-file
transition; after bootstrap begins, DC-35 governance and hold rules control closure. A later target cannot
bypass an unpublished increment: the first release containing any still-unshipped accepted RFC inherits
all of that RFC's release conditions and lifecycle/status corrections. If the three authorities disagree,
the release lane is parked; see `MILESTONES.md` under Baseline and release posture.

## Current Increment

- **DC-41 - integrity evidence campaign (accepted; stage-1 implementation next).** Design accepted
  2026-07-23 after two repair rounds (B1-B4 closed). Four independently staged workstreams — crash-matrix
  audit, hash vectors, hash differential, property/fuzz — each land as its own implementation review; no
  stage may be bundled. All four are completable inside the parked development lane. A fifth workstream
  (platform matrix) was descoped into its own future increment, gated on the M1 portability-claim doc
  correction; it is recorded, not dropped. This acceptance does not activate 0.18.0 release preparation.
  Release-specific reproduction and gate evidence must still be rerun when an RC is explicitly selected.

- **DC-40 - state Merkle root and format transition (complete at `70c3902`).** The accepted RFC and
  companion state-root/format FDD define the remaining M1 identity correction: a canonical clean-tree
  Merkle root, repository-format-aware reads, seal/replay/verify integration, pinned vectors, and
  explicit format-1/format-2 compatibility behavior. DC-39 now supplies the accepted strict envelope
  validator required by this increment. Architect implementation review v1 required strict format-2
  read admission, anchored marker rereads at mutation boundaries, an opaque exact-recovery cleanup
  authority, and a genuine format-1 CLI matrix. The repaired implementation candidate closes those
  findings. Architect repair re-review v1 accepted the candidate, committed as `70c3902`; architect
  post-commit evidence review v1 accepted independent no-hardlink checkout and deterministic-archive
  evidence on 2026-07-23. DC-40 implementation delivery is complete but remains in `accepted/` until
  0.18.0 is released; no release authority follows from completion.
- **DC-39 - signature and envelope authority (complete at `8f565f2`).** Architect review v1
  required the public canonical envelope serializer and strict Ed25519 shape to enter the authority
  boundary. The repaired design now pins those rules, invalid-predecessor handling for `add_signature`, and
  deterministic diagnostic multiplicity/order. It also retains the accepted DC-34 preimage with a
  literal deterministic Ed25519 vector, defines one duplicate/order tuple excluding advisory
  signature time, separates structural format-1 diagnosis from strict
  new-write/format-2 validation, inventories persistence and signing surfaces, and records the
  schema-1 RefUpdate zero no-clock sentinel in a companion FDD-03 erratum. Architect design re-review
  v1 accepted the repaired RFC and companion on 2026-07-22. The bounded implementation
  enforces strict canonical admission before new persistence, retains structural format-1 reads with
  deterministic warning-only diagnostics, pins the signature preimage and shape matrix, and documents
  the public compatibility boundary. Implementation repair re-review v2 accepted the candidate and
  post-commit evidence review v1 accepted independent no-hardlink checkout and deterministic-archive
  evidence on 2026-07-22. The implementation is complete but remains in `accepted/` until 0.18.0 is
  released; no release authority follows from completion.
- **DC-48 - legacy Clippy production retirement (complete).** DC-47's implementation commit
  and post-commit evidence are accepted, satisfying the trigger for a separate subtractive design to
  remove the historical unlocked and locked no-all-features classifier productions. DC-48 was required
  to restore classifier-enforced canonical selection before the 0.19.0 release candidate and did so.
  Architect design review
  v1 accepted the bounded subtraction and required exact bare/prefixed A/B rejection evidence on
  2026-07-22. Architect implementation review v1 accepted the bounded candidate, committed as
  `383e503`. Architect post-commit evidence review v1 accepted its clean checkout/archive evidence on
  2026-07-22. DC-48 and the legacy-Clippy-production blocker are complete.
- **DC-47 - stable Clippy gate alignment (complete).** DC-46 implementation and post-commit reviews
  identified that DC-35's public release gate uses `--all-features` while stable CI and the DC-45
  default-closed classifier recognize only the no-all-features form. All workspace packages currently
  declare zero features and the exact locked all-features command passes. DC-47's accepted design
  preserved DC-35's stronger contract, aligned stable CI and contributor guidance, and added one exact
  non-authority classifier vector. Architect design review v1 accepted the bounded design on
  2026-07-21. Architect legacy-vector test-contract QA v1 resolved the accepted test contradiction:
  the locked legacy vector remains positive, while all non-colliding near misses fail closed. The
  bounded implementation was accepted by architect implementation review v1 on 2026-07-21 and committed
  as `ea95e92`. Architect post-commit evidence review v1 accepted the clean checkout/archive evidence
  on 2026-07-21. DC-47 is complete; its separately reviewed DC-48 follow-up is also complete.
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
  semantics on 2026-07-17, but project-owner acceptance was initially withheld because that candidate
  added 247 files, including 237 per-case vector files. Architect footprint QA conditionally approved a
  three-pack direction, and architect design amendment re-review v1 accepted the explicit decoding,
  location, closure, and archive contract on 2026-07-17. The resulting untracked compact candidate had
  exactly ten root artifacts and three packs and preserved all 237 logical vectors. Implementation
  re-review v1 found one blocking raw dot-segment grammar defect; architect repair re-review v1 accepted
  the repaired candidate and focused end-to-end negatives with no findings on 2026-07-17. Architect
  design repair re-review v1 accepted the explicit compact-oracle retirement schedule
  on 2026-07-17, satisfying the lifecycle-design condition for the owner's separate decision. The five
  Python authoring/verification files remain through the first Rust-gated 0.19.0 release. The first
  later release-candidate increment is blocked until an architect accepts a later-commit stability
  rerun; the following release-candidate increment is blocked until an exhaustive five-file
  decommissioning review removes each file or records an individual owner-approved, event-bound
  exception. The accepted Rust implementation was required to replace the complete manifest verifier
  and self-test matrix, not only the differential-disagreement test, and later did so. The other eight
  frozen contract/evidence files remain until a
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
  was preparation of an isolated authoritative-command cutover candidate and disposable rollback
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
  authority. DC-46 separately tracked the pre-existing mismatch between the declared Rust 1.85 minimum
  and the locked product workspace. Architect design rereview v1 accepted restoration of Rust 1.85
  through three bounded source rewrites, focused trust regressions, and pinned locked CI gates on
  2026-07-21. The bounded implementation candidate exposed a conflict with the accepted DC-45
  procedure grammar. Architect command-grammar amendment QA v1 authorized five exact ordinary-Cargo
  vectors and existing scanner tests. Architect implementation review v1 accepted the candidate,
  committed as `0d221af`, and architect post-commit evidence review v1 accepted its clean
  checkout/archive evidence on 2026-07-21. DC-46 and the Rust 1.85 compatibility blocker are complete;
  DC-47 and DC-48 subsequently completed the public `--all-features` Clippy and governed-classifier
  reconciliation. **Current disposition:** the Rust command is authoritative and the DC-46 through
  DC-48 compatibility/Clippy gates are closed. Only DC-45's event-bound obligations remain: retain
  Python and the frozen oracle through the first Rust-gated 0.19.0 release, obtain accepted later-commit
  stability evidence, and complete the separately reviewed Python/oracle retirement or consolidation
  events.
- **M1 corrective implementation is complete; 0.18.0 release activation is parked.** The signer
  allowlist remains empty and fail-closed. No bootstrap transaction, hold, or RC is active. If the
  project owner explicitly activates 0.18.0 preparation, its conditional sequence is: complete the
  separately reviewed DC-35 signer bootstrap; observe the mandatory public 72-hour hold; rerun the
  literal DC-38 stale-pointer/ahead-log reproduction and correct durable portability claims; obtain an
  explicit architect/security hold-lift ruling; then prepare and review the combined RC. None of those
  gates blocks ordinary design-first development against the accepted corrective baseline. The cosmetic
  unknown/malformed-marker diagnostic (`unsupported format version: 0`) is a non-blocking pre-RC
  correction candidate, not a prerequisite unless selected.

## Release Candidate Increment

- Release lane state: **parked**.
- Activated release target: **none**.
- **0.20.0 released 2026-08-16** — RFC 102 complete (containers, format 6, compaction), `prikk compact`,
  `prikk unlock`, `prikk trust maintainer remove`, dead-surface consolidation. Formats 1-5 rejected at
  open. Windows remains read-only; DC-87 targets 0.21.0.
- **0.20.0 activated 2026-08-16** — RFC 102 complete (containers, format 6, compaction), `prikk compact`,
  `prikk unlock`, `prikk trust maintainer remove`, dead-surface consolidation. **Every format-2 through
  format-5 repository is rejected at open**; migration is `bundle export` on an older binary then
  `bundle import`. Windows remains read-only — DC-87 is retargeted to 0.21.0.
- **0.19.0 released 2026-08-08** — merge execution (DC-74) and merge block lineage (DC-75). See
  MILESTONES.md.
- **0.18.4 released 2026-08-04** (see MILESTONES.md).
- **0.18.1 released 2026-08-03**; 0.18.0 was tagged but never published (see MILESTONES.md).
- Activated 2026-08-02 by the architect under the owner's delegation of minor/patch release scheduling.
- **Why now:** 0.17.7 is the published release and cannot edit the same text file twice (DC-65). That is a
  defect in a shipped artifact, not accumulated scope. 129 commits stand behind it.
- **Why minor, not patch:** DC-61 added `RefState` envelope schema 2; DC-60 and DC-63 added `branch
  create/close` and `tag create/list`. New surface plus a format change is a minor bump.
- Signer bootstrap, hold clock, RC finalization, tag, and publication remain **not** selected — DC-35's
  authority preconditions are unmet (see MILESTONES.md).

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

1. **M0 architecture ratification (complete):** DC-34 decided publication and signature identity
   authority as a no-release design gate.
2. **M1 corrective storage and identity baseline (implementation complete; 0.18.0 release parked):**
   DC-35 through DC-40 remain under `accepted/` until they ship. Signer bootstrap, hold, literal DC-38
   reproduction, portability correction, hold lift, and RC review are conditional release gates that
   activate only through the reviewed tracked release-lane transition. If a later version is selected
   before 0.18.0 ships, that first shipping release inherits every M1 gate and lifecycle correction.
3. **M2 assurance and distribution baseline (development active; eventual target 0.19.0):** DC-45
   through DC-48 preparatory tooling and compatibility work already landed. Proceed in order through
   DC-41 adversarial integrity evidence, DC-42 performance and maintainability gates, and DC-43 release
   security and distribution controls against the accepted corrective baseline. Each proposed RFC still
   requires its own design acceptance before implementation. Release-specific evidence is rerun when an
   RC is explicitly selected, and DC-43 remains required before any public-preview reconsideration.
4. **M3 migration and recoverable backup (release target unassigned):** DC-44 owns NFR-REL-03,
   verifiable backup/restore, and migration exercises that are explicitly outside DC-40 and M2.
5. Branch copy/fork, branch switching, tags/remotes, rollback refs, conflict/inverse evidence,
   rollback authorization, audit/plugin, key lifecycle, sync, and unrelated documentation themes are no
   longer frozen solely by an unpublished M1 release. They remain unselected and require normal
   design-first prioritization before implementation.

Final feature scope remains governed by accepted RFCs, genuine gating FDDs when present, and the
current-state reference docs.

## Future Themes

Recorded 2026-08-04 so they are findable rather than conversational. **None is scheduled**; each names its
own prerequisite. Ordering against the accepted roadmap items (node-model apply → merge execution → M4
attestation slice) is not implied.

### Sync — recorded independently, prerequisite is a threat model

M5 bundles "Sync and Quarantine." **They are separable, and sync alone is at least three distinct
questions** — bundling them under one label would repeat the "increment 4.4" error, where one marker
covered two unrelated blockers and produced a wrong roadmap framing:

- **Sync — now criterion 1 of the status-claim criteria** (`MILESTONES.md`). Recorded 2026-08-09: nothing in the tree exchanges history between repositories, so a *distributed* VCS cannot currently distribute. **This is the largest single gap between prikk and dropping the "early implementation" badge**, and it is unowned with no increment behind it. Cross-platform mutation lets more people use prikk alone; only sync lets two people work together across machines.
- **Multi-parent block lineage** — deferred out of DC-74 on 2026-08-08, **not rejected**. `BlockPayload.parent_block_ids` is already `Vec<ObjectId>`, sorted and unique, with a source comment anticipating *"a later design adds semantic parent roles"* — so this is a replay question, never a format change. `patch_replay.rs:206` fails closed on multi-parent lineage, and lifting that reopens what a baseline is for DC-64's cache, what `rollback_preview` walks, and what a horizon means. **The open question is whether it buys anything**: under DC-74's adoption model the patch DAG already records a merge structurally, so block parentage may be bookkeeping that duplicates it. Product **M3** is named "Block DAG and Checkout", which may encode a Git-inherited assumption worth re-examining rather than inheriting.
- **Transport** — what moves objects between repositories, and whether prikk owns that at all.
- **Peer trust** — what a remote is permitted to assert. All trust is local today
  (`trust maintainer add`); a peer claiming a ref advanced is a new authority question.
- **Quarantine policy** — what happens to objects that arrive untrusted. `.prikk/quarantine` already
  exists in the layout, so the original design anticipated this.

**Prerequisite, per the owner's 2026-08-04 direction ("security is strongly prioritized to function;
secure by default; we should not be in a hurry"): a threat model before any sync code exists.** Sync is
the first capability that gives prikk an attack surface it does not have today — verified: zero networking
crates in `Cargo.lock`, no networked verb in the CLI.

**Dependency note.** An async runtime in `prikk-store` would need a DC-51 amendment
(`placement.rs:11` permits only `getrandom` and `rustix`). That is part of the sync design, not a
discovery to be made during it.

### Repository layout when sync arrives — decided 2026-08-04, applied later

**Nested directories under one workspace**, not multiple workspaces:
`crates/{shared,client,server}/…`, with today's seven crates moving to `crates/shared/` at that point.

**Why not multiple workspaces:** DC-51's placement gate runs `MetadataCommand` against the root
`Cargo.toml` (`boundary.rs:48-51`) and sees **one** workspace. Splitting would require four invocations
and reconciliation logic, plus four `Cargo.lock` files — so `--locked` would stop meaning what it means
today — and four `rust-version` declarations to keep aligned.

**Cargo does not care about directory depth**, so nesting needs no mechanism, only longer `members` paths.

**Not applied now**, deliberately: eight flat members are not messy, and moving them would churn every
path in `Cargo.toml`, the placement allowlist, and every `use` in the tree for a problem that does not yet
exist. **Apply it once, when sync lands, informed by what sync actually needs** — which may be far smaller
than a tier, since a sync endpoint might be a dumb object store with a trust boundary rather than an
application server.

Separately: **crate names are global on crates.io and nesting does not change them.** Naming discipline is
its own decision.

**Executable structure is a separate question, raised 2026-08-04 and not decided.** One binary with a
`prikk serve` subcommand, or separate `prikk` and `prikk-server` binaries?

**The security argument favours separate binaries**, and it follows the project's own "secure by default"
posture: a single binary means every user who only commits locally still ships and links network-capable
code they never run. DC-51's placement gate is **per crate**, so a server crate could legitimately take an
async runtime the client crates cannot — but if the *binary* is one, the client links it anyway. Separate
binaries keep the gate's benefit at the executable level, not only the crate level.

**Against:** two artifacts per target in DC-70's release workflow, two install paths to document, and a
version-skew surface between client and server that a single binary makes impossible by construction.

**Decide with the sync threat model**, not before — it depends on whether the server is an application or
a dumb object store with a trust boundary.

### Merge execution — CONFIRMED as the next accepted increment (roadmap item B)

Owner-ruled 2026-08-04: **B then C** — merge execution, then the M4 attestation slice. `merge-evidence` and
`merge-plan` exist; **nothing applies a merge** (`IMPLEMENTATION-STATUS.md:302`). DC-16's conservative
subset and its soundness oracle are the foundation; execution is the unbuilt half. RFC not yet written.

### Conflict arbitration — recorded, and it is a trust question before it is a UX one

**Conflict *detection* exists.** `patch_algebra` produces typed conflict witnesses (`ConflictWitnessKind` —
`SamePathCreate`, `NodeIdReuse`, `LiveStateMismatch`, text-anchor kinds). **Nothing resolves them.**

**The question that decides the design:** in prikk's model a resolution is itself a **patch**, which must be
authored and signed. So an arbitrator that resolves automatically is producing signed content on someone's
behalf. That is the same class of question as DC-35's "automation may verify evidence but cannot occupy an
accountable approval identity" — and it should be answered from that precedent rather than treated as an
ergonomics feature.

Depends on merge execution existing. Not scoped.

### Patch aggregation — an original concept that is NOT in the requirements

**Recorded 2026-08-04 at the owner's prompting. Finding: it appears nowhere in `specs/`.** Grepping the
requirements, NFRs, external design, and roadmap for *aggregate*, *compose*, *squash* returns nothing. The
only related material is DC-18's **sequence confluence** — a proof that two candidate sequences compose to
the same authored result — which is a *property*, not a capability that emits a composed patch.

**So an original design concept never reached the written requirements.** That is worth knowing
independently of whether it gets built.

**Intended workflow, as described by the owner:** after a branch's development completes, generate a
**block-unit patch** — one patch representing the whole block's change.

**The tension that must be resolved before designing it.** prikk's thesis is history that cannot lie by
construction: every change signed by its author, every publication policy-gated. **Aggregation that
discards the constituent patches destroys exactly that** — the aggregate would carry one signature where
there were N, and the record of who authored what would be gone.

**A defensible shape exists**: the aggregate is *derived*, the constituents are *retained*, and the
aggregate is offered as a view or a transfer unit rather than as a replacement for history. `BlockPayload`
already holds `patch_ids: Vec<ObjectId>`, so a block is already the natural aggregation boundary — which
may mean much of this is presentation over existing structure rather than a new object.

**Do not design it as squash.** If the answer turns out to require discarding constituents, that is a
change to what prikk claims to be and belongs to the owner, not to an increment.

### Structured output for tooling — prerequisite for the M4 slice

`prikk` has **no `--format json`** and no machine-readable output of any kind; every command prints prose
(verified 2026-08-04). The CI-publication-gate scenario requires `verify` to emit something a job can
assert on — grepping prose breaks the moment wording changes, and `verify`'s output changed twice in the
week of 2026-08-04 alone.

**Should land with or just before the M4 attestation slice**: a policy-gated publication whose result can
only be read by a human is half a gate. `release-policy`'s existing `--format json` is the precedent.

### Editor, IDE, and file-manager integration — blocked on model gaps, not on API work

Deferred, and the reasons are the point: **no current-branch pointer** (an IDE status bar has nothing to
show — every command resolves `--ref` explicitly), **`worktree-status` cannot run** against any repository
the CLI produces, and **there is no `diff` command**. An integration API today would expose those gaps as
the product.

`diff` itself, when scheduled: **first-party, reusing `text_span`'s authoring computation** — not a
display-only crate. The spans `plan_authored_text_span` produces are identity-bearing and signed; a
display diff computed differently would show the user something other than what gets committed, which is
the wrong failure to design into a tool whose claim is that the repository is the evidence.

### Cross-platform mutation — open question, not scheduled

Read-only commands run on macOS and Windows as of DC-71; **mutation is Linux-only**, so prikk cannot be
*used* off Linux, only inspected — and both roles of the two-role model need mutation.

The cost is smaller than "three implementations": the logic is shared, and what differs is a handful of
primitives (anchored `NOFOLLOW` opens, directory fsync). **macOS is a port; Windows is a rewrite.** The
question to settle first is whether the durability guarantee stays platform-uniform, which is an owner
decision, not a technical preference.

### MSRV policy — to write before packaging is attempted

`rust-version = "1.85"` is the edition-2024 floor, so it cannot go lower. Nothing declares when it may
*rise*. Proposed: **MSRV rises only when a dependency or language requirement forces it, never for
convenience, and a rise is a minor-version event naming the requirement that forced it.** Dependency
pressure (RustCrypto, `rustix`) will force it before "too old" does.

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
| TASK-13 release, versioning & compatibility policy | 2 | Maintainer | DC-35 policy implementation and DC-45 Rust authority cutover committed; signer set empty | Bootstrap the first signer only through its separate reviewed governance transaction, observe the public 72-hour hold, and obtain an explicit hold-lift ruling before the 0.18.0 release candidate. | Reviewed release/compatibility policy and Rust authority gate are committed; signer bootstrap evidence, elapsed hold, and hold-lift ruling are recorded before RC preparation. | `docs/src/reference/release-compatibility.md` |
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
