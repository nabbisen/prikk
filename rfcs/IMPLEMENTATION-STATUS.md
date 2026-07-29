# Prikk Implementation Status

Latest released version: 0.17.7 (DC-33 - concurrency and locking reference)
Current release candidate: none
Current accepted increments: DC-34, DC-35, DC-36, DC-37, DC-38, DC-39, DC-40, DC-41, DC-45, DC-46, DC-47,
DC-48, DC-50, DC-51, DC-54, DC-55, DC-57, DC-58, and DC-59
Current implementation increment: none in flight. DC-35 through DC-40 implementation complete; DC-41 all
four stages implemented and accepted; DC-51 complete at `d3e939b`, post-commit review accepted with one
blocking finding, reference-check repair at `4c8b7a3`; DC-54 (operation path validation symmetry) complete
at `e8f780a`, architect post-commit implementation review v1 accepted 2026-07-28, no repair required.
Current development increment: none in flight. **DC-55 (first-party SHA-256 replacement) complete at
`753ebab`** — accepted `a01e628`, swap `8c84bc4`, fixture repairs `083d6c0` and `753ebab`. Architect
implementation review v1 returned one blocking finding; repaired and accepted at re-review v1 on
2026-07-29, verified by fresh clone with a negative control. `prikk-hash::sha256` now runs on `sha2`, with
the outgoing first-party implementation retained test-only as the differential's permanent independent
reference. DC-55 implemented the **replace** decision DC-50 closed with at `4005efb`; DC-50's record is at
`rfcs/handoffs/DC-50-first-party-sha256-roi-decision/decision-record-v1.md`, and DC-50 stays in
`rfcs/accepted/` rather than `done/` because it ships nothing. Next increment per
`rfcs/EXECUTION-ORDER.md` is DC-58 batch 2.

**DC-59 is complete at `a9c2fe0`** — implementation review accepted 2026-07-29 with no findings. It was
split from DC-56 because this workspace had no benchmark infrastructure. Its report measures the full-tree
scan directly (4.22 ms at 10 files rising to 516 ms at 10,000, change set fixed at one file throughout)
and satisfies DC-56's precondition.

**DC-58 batch 1 is complete at `e1d0213`** — accepted; identity artifacts untouched and all test counts
held. Batches continue.

**DC-57 is HELD and its implementation handoff is withdrawn.** The dev team stopped at handoff Step 1 as
instructed and reported that the active WAL is structurally capped at **one record repository-wide**;
architect verification confirmed it at all three append paths. NFR-PERF-02's 800/1000 thresholds are
therefore unreachable by construction and its boundary tests cannot be built. The requirement presupposes
multi-commit queued active sessions, which **line 464 of this file already records as not implemented**.
The same capability gap leaves NFR-PERF-03 vacuously satisfied. **This needs an owner decision** — whether
multi-commit queuing is a scheduled capability, or whether both requirements need reviewed amendment.
DC-56 then follows, gated additionally on an owner ruling over whether NFR-PERF-01 bounds steady-state
commit cost or every commit — unsettled in the requirement text and in tension with NFR-PERF-04. DC-42 was superseded on
2026-07-29 into DC-56, DC-57, and DC-58; its design review found it bundled three unrelated increments and
that both performance requirements it carried are **missed product gates** rather than scheduled work —
NFR-PERF-01 at product M1, NFR-PERF-02 at product M3. See `MILESTONES.md` § "Two milestone schemes"; the
requirements authorities are now tracked in `specs/`.

Standing process note on review independence, first recorded at DC-54 and refined at DC-55: this project
has one architect, so independent *design* review is not achievable for designs the architect authors.
That is the defined process — the organization document's Phase 2 gate assigns design review to the
high-capability model — not a deviation from it. DC-55 demonstrated both sides of the limitation: author
review found a genuine blocking defect, and a second blocking defect still survived into the
implementation, caught only because DC-55's acceptance criteria had been written to be reproducible from
the repository rather than trusted from the implementer's report. Retain that criteria pattern for
identity-bearing increments; independent review remains achievable at the implementation axis and is where
the weight belongs.
Current release activation: parked
Current activated release target: none
Current governance increment: none (no signer bootstrap or hold started)

> Change history is tracked in `CHANGELOG.md`; this file is a status snapshot. The per-PR notes below
> the current-state lists are retained as historical record (PR-014 through PR-030).

## Current Corrective Program

The independent architecture review of released 0.17.7 is a no-go for production use,
repository-format stabilization, and public-preview readiness. The durable corrective schedule is in
`MILESTONES.md`.

- DC-34 is accepted architecture authority. It owns the publication state-machine and
  signature-preimage decisions required by downstream identity-bearing implementation, but does not
  itself authorize DC-38 through DC-40 implementation before their own gates pass.
- DC-35 through DC-40 are accepted M1 designs. DC-37 implementation was accepted and
  committed on 2026-07-15, DC-36 implementation was subsequently accepted, and DC-38 implementation
  was accepted after repair re-review on 2026-07-15. DC-35's governance amendment was accepted after
  design re-review v3; policy/documentation implementation review v1 required repairs. The tracked
  second repair re-review required byte/object, canonical-governance, tag-shape, and attempt-growth
  bindings. Architect repair re-review v3 accepted the completed implementation on 2026-07-16. The
  signer allowlist remains empty; bootstrap and all release work remain blocked. DC-45 design acceptance
  and the Rust authority cutover are complete, but neither authorizes bootstrap. DC-39 is
  complete at `8f565f2` after accepted post-commit evidence. DC-40 architect implementation review v1
  required four format-transition authority and evidence repairs. The repaired candidate closes strict
  format-2 reads, anchored mutation admission, exact legacy cleanup authorization, and the full
  format-1 CLI matrix. Repair re-review v1 accepted it, committed as `70c3902`; post-commit evidence
  review v1 accepted independent checkout/archive and regression evidence on 2026-07-23. DC-40
  implementation delivery is complete. The signer allowlist remains empty and fail-closed. Release
  activation is parked, so no bootstrap transaction or hold has started. If the project owner explicitly
  selects 0.18.0 preparation, the DC-35 bootstrap, mandatory 72-hour hold, literal DC-38 stale-pointer/
  ahead-log reproduction, DC-37-aligned tracked portability correction, explicit hold lift, and
  adversarial RC review become the ordered release gates. Activation first requires one reviewed commit
  that atomically changes the lane state and exact target in this file, `ROADMAP.md`, and
  `MILESTONES.md`. A later first-shipping release inherits these gates while the M1 RFCs remain unshipped.
- DC-39 is accepted after architect design re-review v1 on 2026-07-22. Architect review v1 required
  authority over the public canonical envelope serializer and algorithm-specific signature shape.
  The design repair now adds strict 64-byte
  Ed25519 admission, serializer rejection before output, invalid-predecessor handling for
  `add_signature`, and deterministic diagnostic multiplicity/order to the existing DC-34 vector,
  format boundary, persistence inventory, and RefUpdate no-clock rule. The bounded implementation
  applies strict admission at all inventoried new-write boundaries, preserves format-1
  bytes while reporting deterministic warning-only diagnostics, and pins the required vectors and
  compatibility matrix. Architect implementation repair re-review v2 accepted the candidate, committed
  as `8f565f2`; architect post-commit evidence review v1 accepted independent no-hardlink checkout and
  deterministic-archive evidence on 2026-07-22. DC-39 implementation is complete but remains under
  `accepted/` until the 0.18.0 release. No release authority follows from this closure.
- DC-41 is accepted after design review v1 (Needs changes: B1, B2, B3), design re-review v1 (Needs
  changes: B4), and design re-review v2 (Accept) on 2026-07-23. It is four independently staged and
  implementation-reviewed workstreams (crash-matrix audit, hash vectors, hash differential, property/fuzz),
  all completable inside the parked development lane; no stage may be bundled with another, and none
  discharges the M1 literal DC-38 reproduction, which is rerun when an RC is explicitly activated. A
  fifth workstream (platform matrix) was descoped from DC-41's accepted scope into its own future
  increment, recorded in the RFC's Follow-up section, triggered once the M1 portability-claim doc
  correction ships. DC-56, DC-57, DC-58 (superseding DC-42, archived 2026-07-29) and DC-43 remain proposed
  and do not authorize implementation before their individual design reviews are accepted; their
  development order is DC-59, DC-56, DC-57, DC-58, then DC-43. Release-specific evidence must still be rerun when an RC is explicitly selected.
  DC-45 through DC-48 are preparatory work already landed before M1 release and do not displace that
  remaining sequence. DC-45 was accepted after architect design repair re-review v1 on 2026-07-16.
  Duplicate-name profile
  hardening was accepted and committed as profile-contract identity `ea427df`. The separately scoped
  Python observation adapter review v1 required independent final projection and negative assurance.
  The repair also checks top-level identity and was accepted after implementation repair re-review v1
  on 2026-07-16 and committed as adapter identity `6be65af`. An isolated `12c137d` comparison found zero
  mismatches across 145 common cases and exactly nine profile-contract additions. The 154-case exact-
  byte oracle implementation review v1 found five blocking closure/contract defects. The repaired
  candidate materializes release-state governance dependencies, separates kebab-case oracle IDs from
  fixture-visible IDs, binds exact two-snapshot sequences, corrects reason precedence, and enforces
  exact coverage membership. Architect implementation repair re-review v1 accepted the freeze on
  2026-07-17 with no semantic blockers. Project-owner acceptance was initially withheld because that
  candidate added 247 files, including 237 per-case vectors. Architect footprint QA conditionally
  approved three strict suite-level JSON packs and required an explicit decoding/location/closure/archive amendment.
  Architect design amendment re-review v1 accepted that contract on 2026-07-17. Compact implementation
  was then prepared without staging: the candidate had 13 files, retained all 237 logical vectors across
  three packs, and implementation review v1 found one blocking raw dot-segment grammar defect. The repaired
  schema and shared lexical validator plus focused direct/packed/registry/physical-pack tests were
  accepted after architect repair re-review v1 on 2026-07-17. Explicit project-owner acceptance of the
  exact 13-file inventory was committed with the reviewed design/status update as stage-1 freeze commit
  `47aec9c` on 2026-07-17. Architect design repair re-review v1 accepted the explicit retirement
  schedule, satisfying the lifecycle-design condition for that owner action. Its five Python authoring
  and verification files remain through the first Rust-gated 0.19.0 release. The first later release-
  candidate increment is blocked until architect acceptance of a later-commit stability rerun; the
  following release-candidate increment is blocked until an exhaustive five-file decommissioning review
  removes each file or records an individual owner-approved, event-bound exception. The accepted Rust
  implementation was required to replace the complete manifest verifier and self-test matrix, and later
  did so. Its other eight frozen contract/evidence
  files remain until a later equivalence-backed replacement/consolidation review or an explicit final-
  retirement review closes migration and rollback needs. These blockers stay tracked even if DC-45
  moves to `done/`. Two deterministic archives matched; checkout and extracted-archive normal
  verification/self-test passed all 154 cases; all 19 manifest-bound direct dependencies were present;
  the manifest, manifest schema, verifier, generator, and three packs matched byte-for-byte between
  checkout and archive; and all seven product package listings excluded oracle/tool paths. Architect
  post-commit evidence review v1 accepted the isolated freeze and this evidence on 2026-07-17. Stage-2
  Rust implementation was then authorized while Python stayed authoritative. Architect stage-2
  implementation re-review v1 found six blockers: Pages progression parity, per-case input identity, complete Python
  self-test replacement, independent policy invariants, fail-closed reference scanning, and independent
  publication-procedure authority. Architect repair re-review v1 closed Pages parity, self-test
  replacement, and independent invariants, but kept input identity and both command scanners open.
  Architect repair re-review v2 accepted consumed-byte input identity but found quoted-comment and
  long Python-option escapes in the shared parser. The third repair uses quote-aware shell tokenization,
  explicit command boundaries, arbitrary leading Python flags plus `--`, and fail-closed malformed or
  unsupported executable command handling. Repair re-review v3 accepted those cases but found an
  empty-quoted-word comment escape and dynamic Cargo authority gaps. The fourth repair tracks shell
  word start independently from token bytes and enforces literal inventory-backed Cargo authority by
  rejecting dynamic Cargo executables, dynamic Cargo subcommands, and Cargo-less Rust-policy shapes.
  Repair re-review v4 closed reference scanning but found that generic dynamic executable and phase
  names could still hide publication. The fifth repair rejects every dynamic command head after
  bounded YAML/wrapper/assignment prefixes, independent of identifier spelling, while retaining
  literal-inventory-only publication authority. Repair re-review v5 found incomplete YAML sequence
  positioning and wrapper-option arity. The sixth repair uses a closed prefix grammar for `run:` and
  `- run:`, explicit `env` option operands, bounded `command` options, and fail-closed unsupported or
  incomplete wrapper prefixes. Repair re-review v6 accepted those forms but found that unrecognized
  exec wrappers could still hide dynamic publication. The seventh repair rejects every unrecognized
  literal head carrying dynamic arguments, with only an explicit inert command/metadata set exempted.
  Repair re-review v7 accepted that wrapper-class closure but found backtick-substituted heads and
  opaque shell command strings. The eighth repair preserves executable backtick evidence through
  tokenization and fails closed on backticks, `sh`/`bash`/`dash -c`, and `eval` command strings rather
  than interpreting them. Repair re-review v8 closed backticks but found the opaque-string class open
  through other interpreters and nested wrappers. The ninth repair replaces denylist closure for
  governed procedures with structural YAML `run:` extraction and a default-closed head model; only
  policy commands, inventory publication forms, exact current CI commands, `mdbook build`, and inert
  heads are accepted. Repair re-review v9 accepted the policy model but found valid YAML `run` shapes
  silently omitted. The tenth repair accepts arbitrary post-dash whitespace, locates `run` anywhere in
  flow mappings, and fails closed on malformed or unsupported recognized command forms. Repair
  re-review v10 closed those B7.3 cases but found equivalent quoted and whitespace-separated block
  `run` keys silently omitted. The eleventh repair uses the same normalized key path for block and flow
  mappings, fails closed on recognized empty command values, and explicitly preserves GitHub Actions'
  non-command `defaults.run` mapping. It keeps the frozen dependency identity and records that a YAML
  dependency requires separate review and lockfile re-freeze. Architect repair re-review v11 accepted
  B7.4 and authorized the isolated stage-2 implementation commit on 2026-07-17. Post-commit identity,
  deterministic archive, extracted-archive, checkout/archive, and product-package evidence were
  collected after the accepted implementation was committed as `6a65a35` on 2026-07-21; architect
  post-commit evidence review v1 accepted them on 2026-07-21. Preparation of the isolated
  authoritative-command cutover candidate and disposable rollback rehearsal was then authorized for
  separate architect review. Preparation found that `tools/release-policy/src/reference.rs` requires
  the Python executable, command, and all three Python live references as immutable constants. Merely
  switching the RFC-permitted inventory and documentation therefore fails `reference-check`, while
  changing the Rust gate would exceed the stated command-only cutover scope. Architect QA v1 accepted a
  separate pre-cutover repair with exact immutable descriptors for the Python path/command and Rust
  manifest/Cargo command. The repair implements exact pair selection, regular-file anchors, exactly
  one selected live registration at each required path, and fail-closed mixed, unknown, missing,
  duplicate, extra-path, and classification-substituted states. Architect implementation review v1
  accepted it, and it was committed as `2bfb7cc` on 2026-07-21. Its post-commit preservation evidence
  was accepted after architect review v1 on 2026-07-21. The exact inventory/live-reference cutover was
  committed as `6a8e365`; deterministic archive, clean checkout/extraction, full gate, and committed-
  identity rollback evidence was accepted after final architect ruling v1 on 2026-07-21. The Rust
  command is governance-authoritative. Before Python retirement,
  follow-up controls must restrict `defaults.run` nested keys to the GitHub Actions configuration
  contract, classify extractor/allowlist changes as policy changes, and close responsibility-map
  executable correspondence. Python and the frozen oracle remain required through the first Rust-gated
  0.19.0 release and an accepted later-commit stability rerun.
  **Current DC-45 disposition:** the compact oracle, Rust implementation, reference transition, and
  authoritative-command cutover are committed and accepted. Only event-bound obligations remain:
  retain Python and the frozen oracle through the first Rust-gated 0.19.0 release, obtain accepted
  later-commit stability evidence, and complete the separately reviewed Python/oracle retirement or
  consolidation events.
- DC-44 is the proposed post-M2 migration/backup/restore RFC; no release target is assigned.
- DC-47 is the accepted stable Clippy gate-alignment increment. Its implementation preserved DC-35's
  public `--all-features` release contract and aligned stable CI, contributor guidance, and one
  exact default-closed non-authority command-classifier vector before the 0.19.0 release candidate.
  Architect design review v1 accepted the bounded design on 2026-07-21. Architect legacy-vector
  test-contract QA v1 resolved the retained-vector contradiction. Architect implementation review v1
  accepted the bounded candidate on 2026-07-21, committed as `ea95e92`. Architect post-commit evidence
  review v1 accepted its clean checkout/archive evidence on 2026-07-21, completing DC-47. DC-48 design
  was accepted after architect review v1 on 2026-07-22 and required retirement of both unconsumed legacy
  Clippy productions before the 0.19.0 release candidate. Exact bare and bounded-prefix A/B rejection
  evidence was binding. Architect implementation review v1 accepted the bounded candidate, committed as
  `383e503`. Architect post-commit evidence review v1 accepted its clean checkout/archive evidence on
  2026-07-22, completing DC-48 and the legacy-Clippy-production blocker.
- DC-46 is the accepted Rust 1.85 compatibility corrective increment. Architect design rereview v1
  accepted three bounded `prikk-store` control-flow rewrites, focused production-path trust tests,
  pinned locked MSRV/stable CI commands, unchanged dependencies and lockfile, and separate current-
  stable Clippy on 2026-07-21. Architect command-grammar amendment QA v1 authorized the five exact
  ordinary-Cargo vectors and existing scanner tests needed by the accepted CI contract. Architect
  implementation review v1 accepted the candidate, committed as `0d221af`, and architect post-commit
  evidence review v1 accepted its clean checkout/archive evidence on 2026-07-21. DC-46 and the declared
  Rust 1.85 compatibility blocker are complete. DC-47 and DC-48 subsequently closed the separately
  tracked `--all-features` documentation/classifier reconciliation and legacy-production retirement.
- No release candidate is active. No proposed RFC is implementation authority until it moves to
  `rfcs/accepted/` under RFC-000.

## Current State (0.17.7)

- DC-33 shipped in 0.17.7. It adds `docs/src/reference/concurrency-locking.md`, a current-state
  reference for active-session locking, ref-specific publication locks, compare-and-swap behavior,
  narrow ref repair locking, and manual stale-lock limits. It does not change code, schema, CLI
  behavior, lock behavior, repository behavior, verification, doctor, trust, release semantics, or
  repository-format stability claims.

## Previous State (0.17.6)

- DC-32 shipped in 0.17.6. It adds `docs/src/reference/path-safety.md`, a current-state reference for
  repository path validation, checkout/worktree materialization safety, worktree authoring safety, and
  deferred path/platform gaps. It does not change code, schema, CLI behavior, checkout behavior,
  materialization behavior, worktree authoring behavior, repository behavior, or repository-format
  stability claims.

## Previous State (0.17.5)

- DC-31 shipped in 0.17.5. It adds `docs/src/reference/repository-layout.md`, a current-state
  reference for initialized `.prikk/` paths, `.prikk/FORMAT`, object/ref/active/trust paths, and
  authority-vs-pointer/cache boundaries. It does not change code, schema, CLI behavior, repository
  behavior, trust policy, verification, repair, or repository-format stability claims.

## Previous State (0.17.4)

- DC-30 shipped in 0.17.4. It adds `docs/src/guide/security-setup.md`, a current-state operator guide
  for AUTHOR and MAINTAINER signing setup, environment key inputs, repository-local maintainer trust,
  sensitive seed handling, key-generation/public-key-derivation absence, current failure hints, and
  deferred key-management work. It does not change code, schema, CLI behavior, repository format,
  signing behavior, trust policy, verify behavior, or seal behavior.

## Previous State (0.17.3)

- DC-29 shipped in 0.17.3. It adds `docs/src/reference/integrity-recovery.md`, a current-state
  reference for repository verification and doctor diagnostics. It documents verify scope and limits,
  output/failure behavior, all six active WAL metadata states, the current doctor issue catalog, narrow
  doctor repair boundaries, rollback verification relationship, deferred work, and source anchors. It
  does not change code, schema, CLI behavior, repository format, trust policy, verify behavior, doctor
  behavior, or repair behavior.

## Previous State (0.17.2)

- DC-28 shipped in 0.17.2. It adds the current-state
  `docs/src/reference/durability-recovery.md` reference for active-WAL persistence, WAL replay/tail
  handling, active ref metadata, seal publication flow, ref-pointer recovery, doctor repair limits,
  stale-lock limits, and deferred crash/platform evidence. It does not change code, schema, CLI
  behavior, repository format, WAL, refs, seal, verify, doctor, trust, or release semantics.

- DC-27 shipped in 0.17.1. It adds the current-state
  `docs/src/reference/patch-algebra.md` reference for patch algebra, commutation, confluence,
  merge-evidence outcomes, reason-code/proof-phase vocabulary, and merge-plan mapping. It does not
  change code, schema, CLI behavior, merge execution, merge-base discovery, branch publication,
  persisted proof/witness objects, JSON output, or public Rust API stability.

- DC-25 shipped in 0.17.0. It adds `prikk merge-plan`, a read-only planning surface over the existing
  explicit-input merge evidence path. The command resolves explicit baseline/left/right selectors,
  derives sealed candidate sequences, preserves the underlying evidence outcome and reason, maps it to
  a plan status/action, and reports that no merge commit, ref update, WAL write, object write, or
  worktree change was performed. It does not add automatic merge-base discovery, merge execution,
  branch publication, multi-parent Blocks, persisted plan/evidence objects, schema changes,
  scoped/path-limited analysis, JSON output, or public `prikk-replay` API stabilization.

- The temporary `rfcs/fdds/FDD-00-DATA-MODEL.md` and
  `rfcs/fdds/FDD-04-TRUST-THREAT-MODEL.md` compatibility pointers from 0.16.1 were removed in 0.17.0.
  The authoritative current-state references remain `docs/src/reference/data-model.md` and
  `docs/src/reference/trust-threat-model.md`.

## Previous State (0.16.1)

- DC-26 shipped in 0.16.1 as a documentation-only release. It moves the authoritative current-state
  data-model and trust/threat references into `docs/src/reference/`, replaces
  `rfcs/fdds/FDD-00-DATA-MODEL.md` and `rfcs/fdds/FDD-04-TRUST-THREAT-MODEL.md` with temporary
  compatibility pointers for the 0.16.1 window, and updates README/ROADMAP/RFC/status references so
  current-state references are book-owned. It does not change repository format, object schema, trust
  policy, verification, CLI behavior, or RFC lifecycle policy.

- DC-24 documentation work shipped in 0.16.0. It adds current-state FDD references for the data model
  and trust/threat model, plus mdBook reference entry points with inline public caveats and
  claim-to-source anchor tables. This is documentation-only: no repository format, object schema, trust
  policy, verification, CLI behavior, or release semantics change.

- DC-23 shipped in 0.16.0 after the 0.15.0 release and post-release DC-22 test hardening. It
  stabilizes the public `prikk merge-evidence` text UX with clearer selector summaries, unambiguous
  cross-side item display, displayed/total item counts, and report-level output cleanup. It does not
  add merge execution, merge-base discovery, branch publication, merge commits, persisted evidence
  objects, display-path filtering, scoped/path-limited merge analysis, JSON output, schema changes, or
  public `prikk-replay` API stabilization.

- As part of the 0.16.0 release, the mdBook navigation and source tree were reorganized by function, a
  `merge-evidence` command page was added, GitHub Pages publishing was configured and hardened with
  explicit `book.toml` metadata, ignored generated output, verified action tags, and path-scoped deploy
  triggers. The DC-23 display repair also has store-level coverage that pins distinct left and right
  operation summaries for cross-side evidence items.

- Node-addressed worktree patch authoring wired into `prikk commit`: against a **published** local
  branch baseline reconstructed from authoritative replay — or, on a valid unborn `heads/*` ref, a
  **genesis** first commit against an empty baseline (all files authored as `CreateFile`) — worktree changes author
  node-addressed §9.3 operations (`CreateFile`, `DeleteNode`, `EditText`, `ReplaceBinary`, `ChangePerm`)
  with CSPRNG-minted node ids in canonical order, normalized file modes, and shared text-span identity.
  Modified text-file nodes author deterministic arbitrary-span `EditText` records selected by byte
  LCP/LCS widened to UTF-8 character boundaries. Existing-node kind is authoritative; rename inference,
  symlink authoring, branch copy/fork, branch switching, and text↔binary transitions are out of scope.
  Genesis is selected only when the target ref has never been published (pointer absent, ref log absent
  or empty, and active WAL empty); a missing pointer with log history, or a non-empty active WAL, fails
  closed and points at seal/doctor rather than re-authoring.
- Active-WAL ref ownership metadata (`.prikk/active/default/ref-name`) is written before the first WAL
  record and removed after successful seal. Non-empty active WALs with missing, malformed, or mismatched
  ref metadata fail closed, preventing cross-ref publication; `verify` and `doctor` now surface missing
  or malformed metadata on a non-empty WAL as active-session integrity errors, while empty-WAL metadata
  debris is warning/local-debris state. Seal refuses unborn Root publication when the ref pointer is
  missing but ref-log history or a partial ref log exists, and retrying a seal after the current WAL has
  already become the published tip drains the active WAL/ref metadata instead of appending a duplicate
  ref update. The active-WAL model remains single-commit-per-active-WAL.
- Role-bound Ed25519 AUTHOR signing for production Patch authoring paths through an injected
  `AuthorSigner` (`Ed25519AuthorSigner`, key material via `PRIKK_AUTHOR_KEY_ID` /
  `PRIKK_AUTHOR_SEED`). The broken `commit --allow-empty` scaffold was removed (R1R). DC-10 removes the
  rollback-draft fake AUTHOR marker: rollback identity is now `PatchPurpose::RollbackDraft`, and
  rollback-draft Patches carry real AUTHOR signatures.
- Role-bound Ed25519 MAINTAINER signing for production publication objects through an injected
  `MaintainerSigner` (`Ed25519MaintainerSigner`, key material via `PRIKK_MAINTAINER_KEY_ID` /
  `PRIKK_MAINTAINER_SEED`). `seal` validates the signer against the local
  `.prikk/trust/keys/maintainer/` and `.prikk/trust/policy.toml` policy before publication, signs
  Block/RefState/RefUpdate envelopes with real MAINTAINER signatures, and writes the real MAINTAINER
  key id into RefUpdate payload identity. AUTHOR, MAINTAINER, and trust-policy key ids share one
  role-bound signature key-id validator, and signature preimage construction is fallible on the shared
  sign/verify path. Verification reports publication-trust failures separately from structural
  corruption.
- Supported patch replay/materialization and direct inverse planning apply deterministic arbitrary-span
  `EditText` through shared localization, splice, and direct-inverse primitives. Rollback preview,
  rollback draft append, and rollback draft verification expose that supported text-edit inverse path
  without adding rollback refs, rollback authorization, or worktree rollback mutation. Rollback-draft
  append snapshots the published target tip before lock-free inverse planning and re-reads it under the
  active lock before appending, rejecting stale plans if the ref changed during derivation.
  Rollback-draft AUTHOR verification remains structural at this layer: it rejects missing, legacy
  marker, wrong-role, wrong-algorithm, and malformed Ed25519 records, including signatures whose byte
  payload is not 64 bytes, without claiming AUTHOR trust-store enforcement.
- Internal patch algebra is present for the DC-16/DC-18 foundation subset. Pair classification models
  `Independent`, `OrderedDependency`, `Conflict`, and `Unknown`, with structured path effects including
  `required_free`, baseline preimage validation for the supported operation subset, scoped evidence
  handling, store-backed resolver facts for lifecycle/text/blob evidence, evidence-backed
  create-before-content-mutation and create-before-mode-change ordering, and oracle-backed vectors.
  Pair commutation now requires classifier independence plus replay-both-orders proof, and flat
  two-sequence confluence requires individual replay-validity, commuting cross-pairs, composed replay,
  and final lifecycle-state equality. Required sealed-baseline/candidate evidence failures, including
  replacement blob evidence, surface separately from ordinary `Unknown` algebra cases and are not
  hidden by earlier sequence-level `Unknown`; explicitly optional unsealed-candidate evidence remains
  fail-closed. This is library/test-only: no CLI, merge execution, persisted witness/proof object,
  object schema change, public conflict UX, public confluence API, or production merge surface is added.
- DC-21 is released in 0.14.0 as an internal, read-only merge/conflict evidence report contract
  over the existing patch-algebra analyzers. The report vocabulary exposes
  `Confluent`, `Conflict`, `OrderedDependency`, `Unsupported`, `Deferred`, `NotConfluent`,
  `EvidenceFailure`, and `InvalidCandidate` rather than the internal `Unknown` bucket; every report
  carries a required `baseline_block_id`, optional replay horizon, sequence summaries, deterministic
  evidence entries, proof phases, evidence scopes, and release-stable diagnostic reason codes. Reports
  do not store raw operation payloads, raw text spans, replacement text, blob bytes, absolute host
  paths, arbitrary object debug dumps, or signer key material. Reason codes are diagnostic vocabulary
  for tests/future display, not persisted object schema. DC-21 still does not add CLI merge, merge
  execution, branch publication, multi-parent Blocks, persisted proof/witness objects, schema changes,
  worktree conflict materialization, patch-algebra crate extraction, or public `prikk-replay` API
  stabilization.
- DC-22 is released in 0.15.0 as a public UX boundary: `prikk merge-evidence` requires an
  explicit `--baseline-block`, accepts exactly one left and one right target selector from block ids or
  current local branch refs, resolves refs through existing RefState validation, derives sealed
  candidate sequences by walking single-parent target ancestry back to the baseline, and displays
  DC-21 outcomes and reason codes without writing objects, refs, WAL records, merge commits, or
  worktree files. It still does not add automatic merge-base discovery, `prikk merge`, merge
  execution, branch publication, multi-parent Blocks, persisted proof/witness objects, schema changes,
  worktree conflict materialization, patch-algebra extraction, or public `prikk-replay` API
  stabilization.
- `prikk-replay` is introduced as an internally scoped semantic replay/lifecycle crate. It owns the
  node lifecycle substrate (`NodeLifecycleState`, `LiveNode`, `NodeContent`, `Tombstone`, lifecycle
  validation helpers, and direct lifecycle tests) plus the lexical repository-relative `RepoPath` leaf
  required by lifecycle state. `prikk-store` depends downward on `prikk-replay` and keeps compatibility
  import modules for existing call sites and the public `prikk_store::RepoPath` surface. Repository
  layout, object storage, refs, WAL, active sessions, lifecycle-cache persistence and trust rules,
  verification, doctor, worktree integration, and store-backed resolver construction remain in
  `prikk-store`.
- DC-20 stabilizes the post-DC-19 replay boundary without changing CLI, schema, repository layout,
  refs/WAL, trust, worktree behavior, lifecycle semantics, or object/replay/text identity. It keeps
  `prikk-replay` internally scoped and non-stable as an external Rust API, uses version-neutral crate
  documentation, keeps `crates/prikk-store/src/node_lifecycle.rs` as an import-only compatibility
  surface, keeps `crates/prikk-store/src/path.rs` as the integration compatibility surface, and keeps
  filesystem root joining in `prikk-store` rather than on `prikk-replay::RepoPath`.
- DC-20 explicitly keeps these deferred: `text_span` extraction, patch-algebra extraction,
  store-backed resolver movement, lifecycle-cache persistence movement, worktree extraction, public
  `prikk-replay` API stabilization, and public merge, confluence, and conflict surfaces.

## Implemented

- Rust workspace scaffold.
- RFC lifecycle policy is now tracked as RFC-000 in `rfcs/done/`, with `rfcs/README.md` pointing to it
  as the directory authority.
- Shared error taxonomy.
- First-party SHA-256 implementation for early object identity tests.
- Deterministic canonical TLV encoder seed.
- Object IDs and object envelopes.
- Core payload shape seeds.
- Persistent `.prikk/` repository layout.
- File-backed object store with identity verification on read.
- Active-session lock scaffold.
- Active-session WAL append/replay for signed patch envelopes.
- Ref-specific lock scaffold.
- RefState object publication primitive.
- Flat hashed ref pointer paths under `refs/by-id/`.
- Inline signed RefUpdate log append/replay.
- Read-only repository verification for persisted object files, sealed block references, sealed
  rollback Patch classification, ref pointers, ref logs, active WAL records, active-WAL metadata health,
  and publication-trust checks.
- Doctor diagnostics that convert verification outcomes, publication-trust failures, and active-WAL
  metadata health into actionable issue codes.
- ActiveSession append API that holds `active.lock` while writing the active WAL.
- Node-addressed worktree patch authoring (`prikk commit`) with role-bound Ed25519 AUTHOR signing; see Current State above.
- Local no-audit seal scaffold that persists WAL patches, creates a Block, publishes `heads/main` or an
  explicit `--ref heads/<branch>`, signs publication objects with a trusted MAINTAINER key, and clears
  the WAL plus active ref metadata after publication.
- Canonical decoding for RefState, RefUpdate, and Block payloads used by verification.
- Read-only sealed-history inspection from the current RefState chain, including rollback block classification.
- Read-only checkout planning that validates current RefState, Block, parent Block, Patch, and optional snapshot Blob references.
- Snapshot-manifest validation and conservative repository path-safety checks for snapshot materialization and status.
- Read-only worktree status against snapshot-backed baselines.
- Explicit deletion planning and opt-in deletion for files removed by supported patch replay.
- Content-anchored `EditText` validation with fixed 32-byte span hashes, deterministic arbitrary-span
  authoring, and arbitrary-span replay/materialization for supported text edits.
- Read-only unsigned inverse planning, non-mutating rollback preview, conservative rollback draft append and verification, and sealed rollback block classification for the supported patch-operation subset, including deterministic direct inverse for arbitrary-span `EditText`. Rollback drafts are identified by `PatchPurpose::RollbackDraft` and AUTHOR-signed with real Ed25519 key material.
- Internal patch-algebra commutation and flat two-sequence confluence analysis for the DC-16/DC-18
  supported subset, with replay-backed pair proofs and scoped evidence-error precedence; see Current
  State above.
- Internally scoped `prikk-replay` crate for replay/lifecycle semantics, with no dependency on
  `prikk-store`; see Current State above.
- Replay-boundary stabilization for the internally scoped `prikk-replay` crate; see Current State
  above.
- Internal read-only merge/conflict evidence reports for the DC-21 vocabulary; see Current State above.
- Current-state data-model and trust/threat-model reference docs:
  `docs/src/reference/data-model.md` and `docs/src/reference/trust-threat-model.md`.
- Minimal CLI for `init`, `trust maintainer add`, `commit [--from-worktree] [--text-edits] [--ref heads/<branch>] -m`, `seal --allow-no-audit [--ref heads/<branch>]`, `status`, `log`, `checkout --plan-only`, `checkout --snapshot-plan`, `checkout --snapshot-materialize`, `checkout --patch-plan`, `checkout --patch-materialize`, `checkout --patch-delete-plan`, `checkout --patch-materialize-delete`, `merge-evidence --baseline-block`, `merge-plan --baseline-block`, `inverse-plan`, `rollback-preview`, `rollback-draft --append-inverse`, `rollback-draft-verify`, `worktree-status`, `verify`, `doctor`, `doctor --repair-wal-tail`, and `--version`. The former `doctor --repair-main-ref` input is retained only to return an explicit compatibility refusal.

## Not Implemented Yet

- General destructive worktree pruning and full patch-based checkout semantics.
- Branch switching, branch copy/fork from an existing tip, merge-base semantics, branch deletion/rename,
  tag or remote ref creation, rollback refs, multi-commit queued active sessions, and per-ref active WALs.
- Key management/rotation, revocation, expiration, multi-maintainer thresholds, remote trust, hardware signing, and broader signature policy beyond the DC-11 local trust store.
- Policy-aware audit/attestation publication from seal.
- Production patch algebra surfaces: rollback ref publication, persisted public conflict witnesses,
  merge state, merge execution, and user-facing conflict resolution. Read-only public merge evidence
  and merge planning are implemented.
- WASM plugin host.
- Audit publication policy.
- Remote sync.
- DC-09 carry-forward items that still need dedicated future DCs: symlink static target validation,
  duplicate scalar-field rejection, and `Operation.preconditions` / `PatchPayload.preconditions`
  migration to the FDD-03 §9.2.2 discriminator model.

## Conservative Repair Boundary

- `prikk doctor --repair-wal-tail` truncates only incomplete trailing active-WAL bytes after verification confirms that all preceding records are valid.
- Format-1 missing-pointer reconstruction is refused in the 0.18.0 corrective implementation; exact interrupted publication completion requires signer-backed `seal` and matching retained active state.
- Repair refuses to mutate the repository when verification reports integrity errors.
- Missing-object repair, checksum-mismatch repair, object quarantine, GC, and malformed-log repair remain deferred.

## Gate Discipline

DC-19 stays within the approved crate-boundary first slice: `prikk-replay` owns the moved lifecycle
substrate and the minimal `RepoPath` leaf, while `prikk-store` continues to own repository IO, refs,
WAL, active sessions, lifecycle-cache persistence, verification, doctor, object storage, worktree
integration, and resolver construction. It does not move `text_span`, `patch_algebra`, lineage
traversal, refs/WAL, active sessions, cache persistence, verify/doctor, CLI behavior, object schema, or
merge/confluence public surfaces.

DC-18 stays within the approved commutation/confluence contract boundary: internal pair commutation
requires classifier independence plus replay-both-orders proof, and flat two-sequence confluence checks
individual replay-validity, cross-pair commutation, composed replay, and final lifecycle-state equality.
Required sealed candidate evidence failures remain outer evidence errors and are not hidden by
algebraic `Unknown`. It does not add CLI behavior, merge execution, persisted proof/conflict-witness
objects, object schema changes, public confluence APIs, rollback refs, rollback authorization,
multi-parent publication, semantic merge, or user-facing conflict resolution.

DC-17 stays within the approved evidence-contract boundary: internal pair classification distinguishes
required sealed evidence failures from ordinary unsupported algebra, uses scoped resolver evidence from
replay/lifecycle state and validated object-store blobs, and keeps conflict witnesses internal. It does
not add CLI behavior, merge execution, persisted conflict-witness objects, object schema changes,
production confluence checks, rollback refs, rollback authorization, semantic merge, public merge
evidence, or user-facing conflict resolution.

DC-16 stays within the approved foundation boundary: it adds internal pair classification, structured
path effects, baseline preimage validation, and test-level both-order replay oracles. It does not add
CLI behavior, merge execution, persisted conflict-witness objects, object schema changes, production
confluence checks, rollback refs, rollback authorization, semantic merge, or user-facing conflict
resolution.

DC-15 stays within the approved hardening boundary: repository verification and doctor report
active-WAL metadata health, rollback-draft append re-checks the target ref tip under the active lock,
ref publication validates local branch refs at its lower-level boundary, and signature key-id/preimage
validation is shared across AUTHOR, MAINTAINER, and trust-policy paths. It does not add rollback refs,
rollback authorization, AUTHOR trust-store enforcement, branch switching, multi-commit active sessions,
commutation, confluence, conflict witnesses, semantic merge, or new object schema.

DC-11 stays within the approved boundary: production publication objects (Block, RefState, RefUpdate)
carry real role-bound Ed25519 MAINTAINER signatures verified against a repository-local trust policy.
It does not add key rotation/revocation/expiration, thresholds above one, hardware signing, remote trust
distribution, audit plugin execution, rollback authorization, AUTHOR signature verification in
repository-wide `verify`, or repository-format stability. Pre-DC-11 placeholder-sealed histories are
reported as publication-trust failures, not structural corruption.

DC-12 stays within the approved boundary: worktree text edits are authored and replayed as
deterministic content-anchored arbitrary spans through shared identity primitives. It does not add
multi-operation diff minimization, direct inverse/rollback for arbitrary spans, rollback refs,
rollback authorization, worktree rollback mutation, commutation, confluence, conflict witnesses, or
semantic merge.

DC-13 stays within the approved boundary: explicit `heads/*` unborn refs can be created as independent
Root histories through `commit --ref` and `seal --ref`, with active-WAL ref ownership metadata and
branch-ref validation. It does not add branch switching, branch copy/fork from existing tips, merge-base
semantics, branch deletion/rename, tag/remote refs, rollback refs, multi-commit queued active sessions,
or per-ref active WALs.

DC-10 stays within the approved boundary: production Patch AUTHOR signatures from `commit` and
`rollback-draft --append-inverse` are real role-bound Ed25519 signatures, and rollback-draft identity is
carried by `PatchPurpose::RollbackDraft` rather than by a reserved AUTHOR key id. It does not add
publication-grade MAINTAINER signing, trust-store/policy enforcement, rollback authorization, rollback
refs, rename inference, symlink authoring, commutation, conflict resolution, audit plugin execution, or
remote sync.

PR-030 stays within the approved foundation boundary by classifying rollback-marked Patches after normal seal. It does not publish rollback-specific refs, authorize rollback, modify the worktree, discover arbitrary spans, minimize text diffs, commute patches, resolve conflicts, or implement audit plugin execution, policy enforcement, or remote sync.

## 0.1.0 PR-022

Added explicit deletion planning and opt-in deletion during supported patch materialization. This is an M2 bridge scaffold, intentionally limited to files removed by replayed DeleteFile operations whose current bytes still match the recorded old Blob. It does not implement algebraic commutation, conflicted states, text edits, or general destructive pruning.

## 0.1.0 PR-021

Added opt-in supported patch replay materialization. This is an M2 bridge scaffold, intentionally limited to file-level operations that already exist in PR-019/PR-020. It does not implement algebraic commutation, conflicted states, text edits, or destructive worktree removals.

## 0.1.0 PR-023

Added a content-anchored text edit validation scaffold. `EditText` now uses a fixed 32-byte old-span hash, anchor IDs are validated, and tests pin basic span-hash stability. Text diff generation, text replay, inverse, commutation, and conflicted merge states remain deferred.


## 0.1.0 PR-024

Added conservative full-file `EditText` replay. Only `anchor_id = "full-file"` is supported, and replay requires the current full file bytes to match the recorded `old_span_hash`. Arbitrary content-span discovery, text-diff generation, inverse, commutation, and conflict witnesses remain deferred.


## 0.1.0 PR-025

Added opt-in full-file `EditText` generation from worktree modifications. The default worktree commit path remains coarse file-level `ReplaceBinary` for modified tracked files. With `--text-edits`, modified tracked files become `EditText` only when both old and new bytes are valid UTF-8; otherwise they fall back to `ReplaceBinary`. Arbitrary span discovery, minimized text diffs, inverse generation, commutation, and conflict witnesses remain deferred.

## 0.1.0 PR-026

Added read-only inverse planning for the supported patch-operation subset. `prikk inverse-plan [path] [--ref REF]` validates the sealed single-parent chain, derives an unsigned inverse Patch payload for `CreateFile`, `DeleteFile`, `ReplaceBinary`, and full-file `EditText`, and reports a deterministic unsigned Patch ID hint. Rollback refs, authorization policy, commutation, confluence, arbitrary-span inverse handling, audit plugins, and sync remain deferred.


## 0.1.0 PR-027

Added non-mutating rollback preview for the supported patch-operation subset. `prikk rollback-preview [path] [--ref REF]` derives the unsigned inverse plan, validates supported replay, and reports file-level `would-create`, `would-delete`, and `would-replace` changes against the latest snapshot baseline. Rollback refs, authorization policy, worktree writes, commutation, confluence, arbitrary-span rollback, audit plugins, and sync remain deferred.


## 0.1.0 PR-028

Added conservative rollback draft append for the supported patch-operation subset. `prikk rollback-draft --append-inverse [path] [--ref REF] -m <message>` derives the supported inverse Patch, validates rollback-preview consistency, requires an empty active WAL, and appends one signed inverse Patch envelope to the active WAL. Rollback refs, authorization policy, worktree writes, arbitrary-span rollback, commutation, confluence, audit plugins, and sync remain deferred.

## 0.1.0 PR-029

Added active rollback draft verification for the supported patch-operation subset. `prikk rollback-draft-verify [path] [--ref REF]` requires an active WAL containing exactly one rollback draft, validates the dedicated rollback signature marker, decodes the Patch payload under the supported replay subset, and compares it with the inverse Patch currently derived from the selected ref. It performs no writes and leaves seal publication, rollback refs, authorization policy, worktree writes, arbitrary-span rollback, commutation, confluence, audit plugins, and sync deferred.


## 0.1.0 PR-030

Added sealed rollback block classification. `prikk log` marks history entries whose target Blocks contain rollback-marked Patch objects, and `prikk verify` counts sealed rollback Blocks and sealed rollback Patch references. Active rollback draft verification remains available before seal. Rollback-specific ref publication, rollback authorization policy, worktree writes, arbitrary-span rollback, commutation, confluence, audit plugins, and sync remain deferred.
