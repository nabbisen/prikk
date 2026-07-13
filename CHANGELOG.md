# Changelog

## 0.17.5 — 2026-07-13

DC-31: repository layout and authority reference.

- Adds `docs/src/reference/repository-layout.md`, an authoritative current-state reference for the
  initialized `.prikk/` layout, `.prikk/FORMAT`, persistent object directories, ref pointer/log paths,
  active-session paths, trust-store paths, and authority-vs-pointer/cache boundaries.
- Documents that `.prikk/FORMAT` is a current format gate, not a stable-format or migration guarantee.
- Documents that `cache/` and `quarantine/` are initialized but not current roots of trust, that
  `gc/` is not a current initialized directory, and that runtime files such as WALs, ref pointers, ref
  logs, trust policies, and maintainer keys are written by later operations rather than bare init.
- Links README, the data-model reference, durability/recovery reference, trust/threat reference, and
  signing setup guide to the layout reference.
- Keeps the release documentation-only: no Rust code, CLI behavior, object schema, repository format,
  trust policy, verification behavior, repair behavior, or release semantics are changed.

## 0.17.4 — 2026-07-13

DC-30: key management and signing setup guide.

- Adds `docs/src/guide/security-setup.md`, a current-state operator guide for AUTHOR and MAINTAINER
  signing setup, environment key inputs, repository-local maintainer trust, and sensitive seed
  handling.
- Documents that Prikk currently has no key-generation or public-key-derivation command, that
  operators must obtain matched Ed25519 seed/public-key material externally, and that published sample
  seeds and keys are unsafe for real signing.
- Links README, the trust/threat reference, and the integrity/recovery diagnostics reference to the
  setup guide.
- Cleans public README and ROADMAP wording so durable docs do not direct readers to local scratch
  paths.
- Keeps the release documentation-only: no Rust code, CLI behavior, object schema, repository format,
  signing behavior, trust policy, verify behavior, seal behavior, or release semantics are changed.

## 0.17.3 — 2026-07-13

DC-29: verify and doctor integrity/recovery reference.

- Adds `docs/src/reference/integrity-recovery.md`, an authoritative current-state reference for
  repository verification and doctor diagnostics.
- Documents what `prikk verify` checks, what it does not prove, verify output and failure behavior,
  all six active WAL metadata states, the current doctor issue catalog, and narrow doctor repair
  boundaries.
- Links the data-model, trust/threat, durability/recovery, and rollback-draft verification pages to
  the integrity and recovery diagnostics reference.
- Keeps the release documentation-only: no Rust code, CLI behavior, object schema, repository format,
  trust policy, verify behavior, doctor behavior, or repair behavior is changed.

## 0.17.2 — 2026-07-12

DC-28: durability and crash-recovery reference.

- Adds `docs/src/reference/durability-recovery.md`, an authoritative current-state reference for
  active-WAL persistence, WAL replay/tail handling, active ref metadata, seal publication flow,
  ref-pointer recovery, doctor repair limits, stale-lock limits, and deferred crash/platform
  evidence.
- Links the data-model and trust/threat reference pages to the durability and crash-recovery
  reference.
- Keeps the release documentation-only: no Rust code, CLI behavior, object schema, repository format,
  WAL/ref/seal/verify/doctor behavior, or release semantics are changed.

## 0.17.1 — 2026-07-12

DC-27: patch algebra and merge-evidence concepts reference.

- Adds `docs/src/reference/patch-algebra.md`, an authoritative current-state reference for patch
  algebra, commutation, flat confluence, merge-evidence outcomes, reason codes, proof phases, and
  merge-plan status mapping.
- Links the `merge-evidence` and `merge-plan` guide pages to the concept reference, so command output
  terms such as `op_seq`, `pair_conflict`, `classification`, `Confluent`, and `ConfluentSubset` have a
  reviewed explanation.
- Adds visible claim-to-source anchors tying the reference to released DCs and implementation paths.
- Keeps the release documentation-only: no Rust code, CLI behavior, object schema, merge execution,
  merge-base discovery, persisted proof/witness objects, JSON output, or public Rust API stability is
  changed.

## 0.17.0 — 2026-07-11

DC-25: merge planning surface.

- Adds `prikk merge-plan`, a read-only planning classification over the existing explicit-input
  merge evidence path.
- Requires explicit `--baseline-block` plus exactly one left selector and one right selector from
  `--left-block` / `--left-ref` and `--right-block` / `--right-ref`.
- Shows submitted selectors, resolved target Blocks, operation counts, plan status, underlying
  evidence outcome/reason, action text, evidence item counts, and evidence items.
- Maps `Confluent` evidence to `ConfluentSubset`, explicitly avoiding a whole-merge or executable
  merge claim.
- Maps blocked evidence outcomes to `BlockedConflict`, `BlockedOrderedDependency`,
  `BlockedUnsupported`, `BlockedDeferred`, `BlockedNotConfluent`, `BlockedEvidenceFailure`, and
  `BlockedInvalidCandidate`.
- Keeps process success separate from plan status: a valid blocked plan is still a successfully
  produced plan, while invalid arguments, selector failures, ancestry failures, object failures, and
  ref failures remain command failures.
- Preserves the existing `prikk merge-evidence` command and shares its read-only selector/evidence
  boundary without changing its diagnostic meaning.
- Removes the temporary 0.16.1 FDD-00/FDD-04 compatibility pointer files after their contents moved to
  the authoritative mdBook reference pages in 0.16.1.

Still deferred: `prikk merge`, merge execution, automatic merge-base discovery, branch merge
semantics, branch publication, merge commits, multi-parent Blocks, active-WAL merge drafts, worktree
conflict materialization, conflict resolution UI, persisted plan/proof/witness/evidence objects,
path-scoped merge analysis, display-path filtering, JSON output, schema changes, patch-algebra crate
extraction, and public `prikk-replay` API stabilization.

## 0.16.1 — 2026-07-11

DC-26: documentation home correction.

- Moves the authoritative current-state data model reference into
  `docs/src/reference/data-model.md`, with the full reference body, public caveats, provenance, and
  visible claim-to-source anchor table rendered in the mdBook.
- Moves the authoritative current-state trust/threat reference into
  `docs/src/reference/trust-threat-model.md`, including the security-claim review discipline required
  for trust, threat, verification, signature, key-management, durability, platform-support, and
  production-readiness claims.
- Replaces `rfcs/fdds/FDD-00-DATA-MODEL.md` and
  `rfcs/fdds/FDD-04-TRUST-THREAT-MODEL.md` with temporary compatibility pointers for 0.16.1. They are
  scheduled for removal in 0.17.0 unless a later review extends the window.
- Updates README, ROADMAP, RFC index, and implementation status references so current-state
  architecture/concept documentation is book-owned, while `rfcs/` remains for design-process and
  genuine gating FDD material.
- Updates the documentation-reference backlog homes for TASK-06/07/08/10/12 to the DC-26
  `docs/src/reference/` model.
- Keeps the release documentation-only: no repository format, object schema, trust policy,
  verification, CLI behavior, or RFC lifecycle policy change is introduced.

## 0.16.0 — 2026-07-11

DC-23: public merge evidence UX stabilization, plus reviewed pre-release documentation work.

- Stabilizes `prikk merge-evidence` text output so selector summaries, resolved target Blocks,
  operation counts, full-report outcome, reason code, and item counts are easier to scan.
- Renders cross-side evidence items as explicit `cross:` blocks with separate `left[...]` and
  `right[...]` operation lines, replacing the ambiguous one-line `<->` form.
- Renders report-level items as `report:` without a fake operation label such as `report report`.
- Shows item counts as `items: N displayed of N`. DC-23 does not add display filtering, so the two
  counts are equal in this release.
- Preserves DC-22 command shape, selector semantics, exit-status behavior, read-only behavior, and
  privacy/redaction rules.
- Adds store-level coverage that pins distinct left and right operation summaries for cross-side
  evidence display items.
- Reorganizes the mdBook navigation and source tree by function, adds the `merge-evidence` command
  page, and adds a GitHub Pages workflow with mdBook metadata and ignored generated output.
- Adds current-state FDD and mdBook reference entries for Prikk's data model and trust/threat model,
  with inline public caveats and claim-to-source anchor tables.
- Keeps the DC-24 documentation scope non-behavioral: no repository format, object schema, trust
  policy, verification, CLI behavior, or release semantics change is introduced.

Still deferred: `prikk merge`, merge execution, automatic merge-base discovery, branch merge
semantics, branch publication, merge commits, multi-parent Blocks, active-WAL merge drafts, persisted
proof/witness/merge-evidence objects, display-path filtering, scoped/path-limited merge analysis,
worktree conflict materialization, JSON output, schema changes, trust-store enforcement changes,
patch-algebra crate extraction, and public `prikk-replay` API stabilization.

## 0.15.0 — 2026-07-11

DC-22: public merge evidence UX boundary.

- Adds `prikk merge-evidence`, a read-only public display over the DC-21 merge/conflict evidence
  report contract.
- Requires explicit `--baseline-block` plus exactly one left and one right selector from `--left-block`
  / `--left-ref` and `--right-block` / `--right-ref`.
- Resolves ref selectors through current local branch RefState validation and shows both submitted
  selectors and resolved target Block ids.
- Derives sealed candidate sequences by walking single-parent target ancestry back to the explicit
  baseline, failing closed on missing ancestry, multi-parent chains, cycles, or unreadable evidence.
- Keeps the surface non-mutating: no object writes, ref updates, WAL writes, merge commits, or
  worktree changes.
- Adds CLI regression coverage for stdout/stderr privacy and read-only behavior on both successful and
  failing command paths.

Still deferred: `prikk merge`, merge execution, automatic merge-base discovery, branch merge
semantics, branch publication, merge commits, multi-parent Blocks, active-WAL merge drafts, persisted
proof/witness/merge-evidence objects, worktree conflict materialization, JSON output, schema changes,
patch-algebra crate extraction, and public `prikk-replay` API stabilization.

## 0.14.0 — 2026-07-05

DC-21: merge conflict evidence contract.

**Release scope.** This release adds an internal, read-only merge/conflict evidence report contract
over the existing patch-algebra commutation and flat confluence analyzers. It introduces
reviewable outcome categories, release-stable diagnostic reason codes for tests/future display, required
baseline identity, sequence summaries, evidence-scope mapping, and privacy-preserving report entries.
Reason codes are diagnostic vocabulary, not persisted object schema.

- **Evidence report vocabulary.** Adds internal report types for `Confluent`, `Conflict`,
  `OrderedDependency`, `Unsupported`, `Deferred`, `NotConfluent`, `EvidenceFailure`, and
  `InvalidCandidate`.
- **Analyzer adapters.** Adds read-only adapters from pair commutation and flat two-sequence confluence
  results into merge evidence reports, preserving required sealed evidence failures and optional
  unsealed candidate failures as distinct public outcomes.
- **Privacy and determinism tests.** Adds focused tests for baseline identity, outcome mapping,
  evidence-scope mapping, deterministic item ordering, narrow `NotConfluent` handling, and report debug
  output that avoids raw text spans, replacement text, blob bytes, and absolute host paths.
- **Flatness diagnostic scope.** Current flatness violations use `SequenceInternalDependencyDeferred`
  as the release-stable diagnostic reason; a separate `flatness_required` reason code remains deferred
  until a later DC introduces a broader flatness-reporting surface.
- **Replay crate publication note.** `prikk-replay` remains internally scoped and non-stable as an
  external Rust API, but its manifest no longer disables crate publication so workspace release
  packaging can publish it consistently with the rest of the crates.

Still deferred: merge execution, `prikk merge`, branch merge, multi-parent Blocks, persisted
proof/witness objects, schema changes, worktree conflict materialization, public conflict UX,
patch-algebra crate extraction, and public `prikk-replay` API stabilization.

## 0.13.0 — 2026-07-05

DC-20: replay boundary stabilization.

**Release scope.** This release stabilizes the post-DC-19 `prikk-replay` boundary without adding CLI
behavior, object schema changes, repository layout changes, or new public APIs. `prikk-replay` remains
internally scoped; at the time of 0.13.0 it was publication-disabled in the manifest, and 0.14.0 later
removed that packaging block without stabilizing its external API. `prikk-store` remains the repository
integration crate.

- **Boundary documentation.** Updates `prikk-replay` docs to use version-neutral
  replay-boundary-stabilization wording and keeps public Rust items explicitly non-stable for external
  API purposes.
- **Compatibility-wrapper inventory.** Records `crates/prikk-store/src/node_lifecycle.rs` and
  `crates/prikk-store/src/path.rs` as retained compatibility surfaces, with semantic ownership in
  `prikk-replay` and no duplicate lifecycle/path implementation in the wrappers.
- **Lexical path boundary.** Keeps `RepoPath` lexical in `prikk-replay`; root joining for
  worktree/repository materialization is owned by `prikk-store`.
- **Focused tests.** Adds direct replay path tests and a store-owned root-joining test, keeping tests
  outside implementation files.

Still deferred: `text_span` extraction, patch-algebra extraction, store-backed resolver movement,
lifecycle-cache persistence movement, worktree extraction, public `prikk-replay` API stabilization,
and public merge, confluence, and conflict surfaces.

## 0.12.0 — 2026-07-05

DC-19: replay/lifecycle crate boundary.

**Release scope.** This release introduces `prikk-replay` as a workspace-internal semantic
replay/lifecycle crate and moves the node lifecycle substrate plus its direct tests out of
`prikk-store`. `RepoPath` moves with the lifecycle substrate as the minimal lexical path leaf required
by `NodeLifecycleState`, while `prikk-store` keeps compatibility wrappers and continues to own
repository layout, refs, WAL, active sessions, lifecycle-cache persistence, verification, doctor,
object storage, and store-backed resolver construction. This release still does **not** add CLI
behavior, object schema changes, text-span extraction, patch-algebra extraction, worktree extraction,
merge execution, public confluence APIs, persisted proof/conflict-witness objects, rollback refs,
rollback authorization, branch switching, key lifecycle, or sync behavior.

- **Workspace-internal replay crate.** Adds `crates/prikk-replay` as an internal/experimental crate
  during DC-19. It was initially publication-disabled in the manifest; 0.14.0 later removed that
  packaging block without stabilizing its external API. Its dependency tree is limited to
  `prikk-error`, `prikk-hash`, and `prikk-object`, with no `prikk-store` dependency.
- **Lifecycle substrate extraction.** Moves `NodeLifecycleState`, `LiveNode`, `NodeContent`,
  `Tombstone`, lifecycle validation helpers, and direct lifecycle tests into `prikk-replay`, preserving
  existing behavior and structured lifecycle errors.
- **Path leaf extraction.** Moves repository-relative lexical `RepoPath` validation into
  `prikk-replay` because lifecycle state stores paths. Filesystem layout, materialization policy, and
  worktree ownership remain in `prikk-store`.
- **Store compatibility bridge.** Keeps `crates/prikk-store/src/node_lifecycle.rs` and
  `crates/prikk-store/src/path.rs` as compatibility import modules so existing internal call sites and
  the public `prikk_store::RepoPath` surface continue through the new crate boundary.
- **Test and file-layout cleanup.** Moves the direct lifecycle tests into `prikk-replay` and splits
  them into focused test submodules under the project line-count and test-placement guidelines.

## 0.11.0 — 2026-07-05

DC-18: patch algebra commutation and confluence contract.

**Release scope.** This release adds an internal, library/test-only commutation and flat confluence
contract for the DC-16/DC-17 patch-algebra subset. Pair commutation now requires classifier
independence plus replay-both-orders proof, and two flat candidate sequences can be proven confluent
only when all cross-pairs commute and composed replay reaches the same authoritative lifecycle state.
This release still does **not** add CLI behavior, merge execution, persisted proof or conflict-witness
objects, object schema changes, public confluence APIs, rollback refs, rollback authorization,
multi-parent publication, semantic merge, or user-facing conflict resolution.

- **Replay-backed commutation.** Adds an internal replay oracle for candidate operation pairs, so
  `Independent` becomes usable commutation evidence only when both operation orders replay from the
  common baseline and yield identical lifecycle state without rewriting operation identity.
- **Flat confluence contract.** Adds internal two-sequence confluence analysis for flat candidate
  sequences, preserving sequence order, rejecting concrete ordered dependencies/conflicts, and proving
  final-state equality through composed replay.
- **Candidate evidence validation.** Validates candidate blob evidence needed by replay, including
  `CreateFile.blob_id` and `ReplaceBinary.new_blob_id`, with required sealed evidence failures surfaced
  as `EvidenceError` and optional unsealed candidate gaps remaining fail-closed as `Unknown`.
- **Evidence precedence hardening.** Confluence scans candidate sequences so sealed-candidate evidence
  errors are not hidden by earlier algebraic `Unknown` or deferred-operation results.
- **Module/test split.** Keeps patch-algebra implementation and tests split across focused files under
  the project line-count discipline.

## 0.10.0 — 2026-07-05

DC-17: patch algebra evidence contract.

**Release scope.** This release turns the internal patch-algebra classifier's evidence boundary into an
explicit store-backed contract. Classification now separates ordinary unsupported algebra from
repository evidence failures, carries required-vs-optional evidence scope through resolver calls, and
keeps conflict witnesses as internal diagnostics rather than public schema. This release still does
**not** add CLI behavior, merge execution, persisted conflict-witness objects, object schema changes,
production confluence checks, rollback refs, rollback authorization, semantic merge, or user-facing
conflict resolution.

- **Scoped evidence contract.** Adds internal `EvidenceScope`, evidence state, and evidence-error
  types so sealed-baseline and sealed-candidate facts fail as integrity errors while explicitly
  optional unsealed-candidate evidence can remain fail-closed as `Unknown`.
- **Store-backed resolver boundary.** Adds a read-only resolver that derives baseline text and create
  blob text from replay/lifecycle state plus validated object-store blob evidence, with no
  default-empty fallback for sealed baselines.
- **Classifier result split.** Pair classification now returns an integrity-aware result surface,
  preserving `Unknown` for unsupported or intentionally deferred algebra while surfacing corrupt,
  missing, malformed, or unreadable required evidence separately.
- **Evidence-backed relation coverage.** Pins same-node `CreateFile -> ChangePerm` ordering/conflict
  behavior and keeps create-before-content-mutation decisions tied to validated blob kind/text
  evidence.
- **Internal witness policy.** Removes loose expected/actual witness fields and keeps witness facts
  deterministic, internal, and test-stable without adding persisted or public diagnostic schema.

## 0.9.0 — 2026-07-05

DC-16: patch algebra foundation.

**Release scope.** This release adds an internal, library/test-only pair-classification foundation for
future patch algebra work. It defines and tests `Independent`, `OrderedDependency`, `Conflict`, and
`Unknown` pair classes, structured path effects including `required_free`, internal diagnostic witness
kinds, baseline preimage validation, and both-order replay oracles for every independent fixture. This
release still does **not** add a CLI surface, merge execution, persisted conflict-witness objects,
Patch/Block/RefState/RefUpdate schema changes, production confluence checks, rollback refs, rollback
authorization, semantic merge, or user-facing conflict resolution.

- **Internal pair classifier.** Adds the private `patch_algebra` module in `prikk-store` with
  crate-internal classifier types and fail-closed handling for unsupported/deferred cases. Rename,
  symlink, malformed, and insufficient-evidence cases classify as `Unknown` or `Conflict`, never
  silently skipped or treated as independent.
- **Baseline preimage validation.** `Independent` now requires each operation's own baseline preimage to
  be proven. The classifier checks baseline path occupancy for `CreateFile.required_free`, live
  path/kind/blob/mode for file deletion and mutation preimages, and text-span localization when text
  resolver evidence is required.
- **Create/mutate evidence boundary.** Same-node create-before-content-mutation ordering requires
  blob-kind/content evidence: `CreateFile -> ReplaceBinary` is ordered only when the created blob is
  proven binary, and `CreateFile -> EditText` is ordered only when the created blob is proven text and
  the edit localizes against that created content. Missing evidence remains `Unknown`.
- **Oracle-backed vectors.** Adds 24 patch-algebra vectors covering same-path create/delete
  dependencies, stale preimages, same-node mode/content interactions, same-node text non-independence,
  resolver-proven text cases, and operation-identity preservation without `op_seq` renumbering.
- **Release-gate cleanup.** Removes legacy test-only `unwrap()` usage that blocked
  `cargo clippy --workspace --all-targets -- -D warnings`.

## 0.8.0 — 2026-07-04

DC-15: active-session integrity and verification hardening.

**Release scope.** Repository health checks now report active-WAL ref metadata integrity explicitly,
rollback-draft append re-checks the target ref tip before writing, the ref publication primitive rejects
non-local branch refs at its own boundary, and signature key-id validation is shared by AUTHOR,
MAINTAINER, and trust-policy paths. This release still does **not** add rollback refs, rollback
authorization, AUTHOR trust-store enforcement, branch switching, multi-commit active sessions,
commutation, confluence, conflict witnesses, semantic merge, or new object schemas.

- **Active-WAL metadata diagnostics.** `verify` records whether active ref metadata is valid, missing,
  malformed, or stale relative to the active WAL. Non-empty WALs with missing or malformed metadata are
  active-session integrity issues; empty-WAL metadata debris is reported as a warning/local-debris
  condition.
- **Doctor diagnostics.** `doctor` surfaces non-empty-WAL metadata faults as error-severity issues and
  empty-WAL metadata debris as warning-severity issues without adding automatic metadata repair.
- **Rollback-draft freshness.** `rollback-draft --append-inverse` snapshots the published target tip
  before lock-free inverse planning, then re-reads the tip under the active lock and refuses to append if
  the ref changed during planning.
- **Ref publication boundary.** `RefStore::publish` now validates `heads/*` local branch refs directly,
  preventing lower-level publication of tags, remotes, rollback refs, or malformed branch names.
- **Shared signature input validation.** `Signature::signed_bytes` is fallible and validates key ids
  through the same ASCII/length policy used by AUTHOR signing, MAINTAINER signing, and maintainer trust
  policy loading.

## 0.7.0 — 2026-07-03

DC-14: arbitrary-span text direct inverse and rollback exposure.

**Release scope.** Supported arbitrary-span `EditText` operations now have
deterministic direct inverses. Inverse planning, rollback preview, rollback draft append, and rollback
draft verification can handle supported text edits by recomputing inverse anchors, duplicate index, and
`span_id` against post-forward text. This release still does **not** add rollback refs, rollback
authorization, AUTHOR trust-store enforcement, worktree rollback mutation, commutation, confluence, or
conflict witnesses.

- **Direct `EditText` inverse.** The shared text-span layer derives inverse records by localizing the
  forward span, applying the forward splice, selecting the exact replacement range in post-forward
  text, swapping old/replacement text, recomputing identity bytes, re-localizing the inverse to the
  exact intended range, and proving byte-exact recovery.
- **Rollback surfaces.** Existing `inverse-plan`, `rollback-preview`, `rollback-draft --append-inverse`,
  and `rollback-draft-verify` surfaces now accept supported arbitrary-span text edits while preserving
  fail-closed behavior for unsupported operations.
- **Rollback draft verification.** Verification compares canonical `PatchPayload` bytes, rejects
  normal-purpose drafts with byte-identical operations, rejects stale inverse identity bytes and
  generated presentation hints, rejects legacy marker-key rollback authority, and structurally requires
  rollback AUTHOR signatures to be Ed25519 records with 64-byte signature payloads. This remains a
  structural check, not AUTHOR trust-store or rollback-authorization enforcement.
- **Deep-review durability repairs.** Seal now refuses to create a Root publication when the target ref
  pointer is missing but ref-log history or a partial ref log exists. Retrying after a crash that
  published the current active WAL but did not drain active metadata now recognizes the already-published
  tip and drains the WAL/ref metadata without appending a duplicate ref update.
- **Pinned vectors.** Added replacement, insertion, deletion, repeated text, CRLF, UTF-8 widening,
  multi-hunk enclosing span, and same-node multi-edit reverse-order vectors.

## 0.6.0 — 2026-07-03

DC-13: non-default ref genesis.

**Release scope.** `prikk commit --ref heads/<branch>` can explicitly create an unborn local branch as
an independent Root history from the current worktree, and `prikk seal --ref heads/<branch>` publishes
that branch through the existing signed `RefState` / `RefUpdate` path. This release still does **not**
claim branch switching, branch copy/fork from an existing tip, merge-base semantics, branch deletion or
rename, tag or remote ref creation, rollback refs, multi-commit queued active sessions, or per-ref
active WALs.

- **Explicit unborn branch genesis.** A valid unpublished `heads/*` ref with no pointer and an absent or
  empty ref log authors against an empty baseline, so all worktree files become `CreateFile` records.
  Pointer absence plus non-empty, malformed, unreadable, or partial ref-log history remains
  recovery/corruption, not genesis.
- **Active-WAL ref ownership.** The active WAL now records `.prikk/active/default/ref-name` before the
  first WAL append. Non-empty WALs with missing, malformed, or mismatched ref metadata fail closed;
  empty WAL metadata debris is cleaned under the active lock. Seal holds the active lock through
  metadata validation, publication, WAL drain, and metadata removal.
- **`seal --ref`.** Seal can publish the queued active WAL to an explicit local branch ref and reports
  the actual advanced ref. A WAL authored for `heads/topic` cannot be sealed to `heads/main`.
- **Publication ordering.** Ref publication now journals the signed `RefUpdate` before pointer
  promotion under the ref lock, preserving recovery evidence for created refs.
- **Upgrade note.** Repositories upgraded with a pre-DC-13 non-empty active WAL that lacks ref metadata
  fail closed. Seal or clear active sessions before upgrading, or inspect the active WAL and metadata
  before retrying.

## 0.5.0 — 2026-07-03

DC-12: arbitrary-span text edits.

**Release scope.** Worktree text edits are authored and replayed as deterministic, content-anchored
arbitrary spans through the shared text-span identity primitives. This release still does **not** claim
commutation, confluence, conflict witnesses, multi-operation diff minimization, semantic merge,
rollback authorization, rollback refs, worktree rollback mutation, or arbitrary-span inverse/rollback.

- **Arbitrary-span authoring.** Modified existing `TextFile` nodes now author one deterministic
  enclosing `EditText` span instead of a whole-file span. Span selection uses byte LCP/LCS, widens to
  UTF-8 character boundaries, and derives anchors, `old_span_hash`, `dup_index`, and `span_id` through
  the shared `text_span` module.
- **Arbitrary-span replay/materialization.** Patch replay and patch materialization apply supported
  `EditText` records by resolving the live `node_id`, validating text preconditions, localizing with
  `locate_text_span`, and splicing with `splice_text`.
- **Pinned vectors.** Added DC-12 byte-level vectors for replacement, insertion, deletion, sub-character
  UTF-8 widening (`é` -> `è` and CJK), CRLF preservation, and multi-hunk enclosing spans.
- **Deferred inverse/rollback.** Inverse planning now fails closed on arbitrary-span `EditText` until
  the direct-inverse round-trip vector set lands.

## 0.4.0 — 2026-07-03

DC-11: publication signing and minimal trust store.

**Release scope.** Production publication objects (`Block`, `RefState`, and inline `RefUpdate`) now
carry real role-bound Ed25519 MAINTAINER signatures verified against a repository-local trust policy.
This is still not full PKI: no key rotation, revocation, expiration, thresholds above one, remote trust,
hardware signing, audit-plugin policy, or repository-format stability guarantee. Pre-DC-11 histories
sealed with `dev-placeholder-maintainer` are treated as pre-publication-trust artifacts and report
publication-trust failures under v0.4.0 verification.

- **Minimal trust store.** `init` creates `.prikk/trust/keys/maintainer/`; `prikk trust maintainer add`
  writes the single-key `required = 1` trust policy through the production path with strict validation.
- **Real MAINTAINER signing.** `seal --allow-no-audit` requires `PRIKK_MAINTAINER_KEY_ID` and
  `PRIKK_MAINTAINER_SEED`, verifies the signer key id and seed-derived public key against local trust
  before publication, and signs Block, RefState, and RefUpdate with role-bound Ed25519 signatures.
- **Publication trust verification.** `verify` checks trusted MAINTAINER signatures for reached Blocks,
  RefStates, and inline RefUpdates, reporting publication-trust failures separately from structural
  corruption. `doctor` diagnoses trust failures but does not auto-trust keys or repair signatures.
- **Compatibility.** `RefUpdatePayload.author_key_id` now records the real MAINTAINER key id, so new
  RefUpdate identities differ from placeholder-era output. Existing PATCH anchors are unchanged.

## 0.3.0 — 2026-07-02

DC-10: rollback-draft identity and AUTHOR signing.

**Release scope.** AUTHOR-role Patch signatures produced by production commands are real role-bound
Ed25519 signatures. Rollback drafts are identified by `PatchPurpose::RollbackDraft`, not by a reserved
AUTHOR key id, and `prikk rollback-draft --append-inverse` signs the draft Patch through the same
injected AUTHOR signer boundary used by worktree commits. This release still does **not** include
publication-grade MAINTAINER signing, trust-store enforcement, key management/rotation, rollback
authorization policy, or stable repository-format guarantees.

- **Rollback-draft purpose marker.** Adds an optional canonical Patch payload `purpose` field. The
  absent field decodes as normal Patch purpose, explicit default encoding is rejected, and
  `RollbackDraft` is pinned by a deterministic hard vector.
- **Real rollback-draft AUTHOR signatures.** `rollback-draft --append-inverse` now requires AUTHOR key
  material, marks the inverse payload as `PatchPurpose::RollbackDraft`, and signs the unsigned Patch
  object id with a real role-bound Ed25519 AUTHOR signature.
- **Purpose-based verification and history classification.** Active rollback-draft verification and
  sealed rollback history classification now inspect payload purpose, fail closed on malformed purpose
  encoding, and report the real AUTHOR key id instead of recognizing the old development marker.
- **Documentation and design records.** Adds DC-10 design and handoff updates, and updates rollback draft,
  sealed history, README, roadmap, and implementation-status documentation to describe the new release
  scope.

## 0.2.0 — 2026-07-02

DC-09 Phase 4.4: node-addressed worktree authoring, genesis first-commit, and role-bound Ed25519
`AUTHOR` signing.

**Release scope.** Node-addressed `prikk commit` patches are role-bound Ed25519 `AUTHOR`-signed. This
release does **not** include trust-store enforcement, key management, `MAINTAINER`/publication signing,
or publication-grade `rollback-draft` signing; symlink authoring is fail-closed; and whole-file reads are
subject to the current large-file limits. The repository format is not yet stable.

`prikk commit` consumes node-addressed worktree authoring (R1/R1R) and supports **genesis / first-commit**
on a fresh repository (4.4b): a never-published ref authors an empty baseline (all `CreateFile`), and seal
publishes a Root block, so `init → commit → seal` works end to end. The earlier layers (4.4-2c-*) remain
internal replay/cache plumbing. Identity anchors unchanged (empty-PATCH `510ab866…5157`, populated
`24031b48…c854`).

- **Release-prep — runtime PR-030-era string cleanup.** `prikk --version` now derives from
  `CARGO_PKG_VERSION` (was the stale `0.1.0-pr030` literal; now prints the crate version, e.g.
  `prikk 0.2.0`); the `checkout`
  mode-flag error and the `status` diagnostic line no longer reference PR-030; stale `PR-030`-prefixed
  module docs (CLI, store, rollback) reworded to describe current scope. No behavior change beyond the
  version string; PATCH-framing anchors unchanged.

- **4.4b P2-1 — CLI end-to-end genesis harness.** Adds `crates/prikk-cli/tests/genesis_end_to_end.rs`, a
  permanent integration test that drives the compiled `prikk` binary through `init → commit → seal → log →
  verify` on a fresh repository (asserts a two-operation genesis commit, a Root block at `update-seq: 1`,
  and clean verify). Guards the release-facing first-commit flow at the CLI boundary. Test-only; no behavior
  change; PATCH-framing anchors unchanged.

- **4.4b — genesis / first-commit authoring.** Enables `init → commit → seal` on a fresh repository.
  Worktree authoring now resolves its baseline through `resolve_worktree_baseline`: when the target ref is
  **published** it authors against replay-derived node lifecycle state (unchanged); when the ref has
  **never** been published it authors against an empty `NodeLifecycleState::new()` baseline, so every
  worktree file becomes a fresh node-addressed `CreateFile` (canonical order, CSPRNG-minted ids, normalized
  modes, real role-bound Ed25519 AUTHOR signature) — a baseline-selection change only, reusing the entire
  existing signed authoring path (review E3). Seal already publishes the first block as `BlockKind::Root`
  (empty parents, `update_seq = 1`, `previous_ref_state_id = None`); no seal change was needed. Genesis is
  selected **only** when the ref pointer is absent **and** the ref log is readable and empty; a missing
  pointer with any ref-log history — or an unreadable/partial log — is treated as corruption, fails closed,
  and points at `doctor` (never silently re-genesis; design §4, review E2). Genesis additionally requires an
  **empty active WAL** — no records **and** no trailing partial bytes (review E1 + 4.4bR P1b): a second
  `commit` before the first `seal` fails closed ("active WAL already contains patches on an unpublished ref;
  run `prikk seal`…"), and a trailing partial WAL tail fails closed pointing at `doctor --repair-wal-tail`,
  rather than authoring a duplicate or ambiguous genesis patch. Empty worktree, worktree symlinks/non-regular
  files, and invalid/non-UTF-8 paths remain fail-closed (genesis synthesizes no zero-operation patch).
  Genesis is **enforced** to the default `heads/main` ref (review Q2 + 4.4bR P1a): a first commit onto any
  other unpublished ref fails closed pending branch-creation design. The Root block inherits the existing
  `scaffold_state_root` pending the real state-Merkle design (review Q3). The active-WAL guard and the WAL
  append are held in **one critical section under the active-session lock** (4.4bR2): the whole
  `commit_worktree_changes_signed` path acquires `ActiveLock` before the guard and holds it through append,
  so concurrent commits cannot both pass the guard and append (the loser fails via lock conflict or the
  post-lock "seal first" guard). Seven new store tests (all-`CreateFile` genesis + real signature;
  empty-worktree, second-pre-seal-commit, missing-pointer-with-log, non-default-ref, and trailing-partial-WAL
  fail-closed; concurrent-genesis serializes to one WAL record); prikk-store 288→295. Identity-neutral to
  existing objects; PATCH-framing anchors unchanged.

- **4.4a-1 — production `NodeIdGenerator` (CSPRNG node-id minting).** Adds the fail-closed minting
  primitive that future worktree authoring will use to assign fresh node identities. A `NodeId` is an
  opaque 256-bit value drawn from the OS CSPRNG — never derived from path, content, operation
  position, timestamp, counter, or baseline state — because it must survive rename/edit/chmod/binary
  replacement and is part of the text `span_id` preimage. The entropy source and the trusted minter
  are deliberately separate (E1): a `NodeIdEntropySource` produces *candidate bytes* (production:
  `OsEntropySource` over `getrandom`), and `NodeIdGenerator` is the only minting authority — it
  constructs an id only through the canonical checked constructor `NodeId::try_from_bytes` (rejecting
  the reserved all-zero value) and rejects any candidate already in the baseline's complete known-id
  set via `NodeLifecycleState::contains_seen_node_id` (E2, over `seen_ids`). Retry is bounded (E3):
  on an all-zero or colliding draw it redraws exactly once, then fails closed with a structured
  `NodeIdMintError` (E4: `EntropyUnavailable` / `ZeroNodeIdDraw` / `NodeIdCollision`) — no weak/seeded
  fallback, no placeholder, no unbounded loop. **Dependency-map change:** `getrandom` is added to
  `prikk-store` only; `prikk-object` stays pure (no entropy/IO). Covered by seven generator tests
  (E5: deterministic nonzero emission; entropy failure; zero rejected-then-redrawn; repeated zero
  fails closed; baseline collision rejected-then-redrawn; repeated collision fails closed; minted id
  nonzero). Also folds in the 2c-4 carry-#1 `splice_text` invalid-range tests (E6: `start > end` and
  `end > text.len()` reject). Unwired: no command path or worktree authoring consumes the generator
  yet, and the four DEV-only worktree-authoring tests remain ignored pending the 4.4a-2 design pass.
  Identity-neutral; PATCH-framing anchors unchanged.

- **4.4a-2a — node-addressed worktree authoring (content operations).** Re-enables
  `commit_worktree_changes[_with_options]` to author node-addressed §9.3 content operations
  (`CreateFile`, `DeleteNode`, `EditText`, `ReplaceBinary`) against a baseline reconstructed from
  authoritative replay, replacing the prior fail-closed stub. Baseline policy is **Option A**: the
  baseline node lifecycle state comes only from `replay_derived_state` over the ref's node-addressed
  lineage (`resolve_node_lineage_bounds`); the snapshot manifest is never an identity authority, and a
  snapshot-only baseline (empty node state with a `snapshot_blob_ref`) **fails closed** (review E3).
  Existing paths resolve to their persisted `node_id` via the replay-derived `NodeLifecycleState`;
  existing-node `NodeKind` is **authoritative** — a `TextFile` modification authors a whole-file
  `EditText`, a `BinaryFile` modification authors kind-preserving `ReplaceBinary`, and a text↔binary
  transition fails closed (`UnsupportedKindTransition`, review E4). Fresh nodes are minted through the
  production `NodeIdGenerator` in **canonical create order** (candidates sorted by `RepoPath` bytes
  before minting, each inserted into a working `NodeLifecycleState` immediately so same-session draws
  cannot alias), making path→`node_id` assignment independent of filesystem traversal order (review
  E1). Operations are emitted in a **canonical order** (kind rank `DeleteNode` < `CreateFile` <
  `ChangePerm` < `ReplaceBinary` < `EditText`, then `RepoPath` bytes, then `node_id` bytes) before
  `op_seq` assignment, so patch identity does not depend on traversal/map iteration. All `EditText`
  span identity (anchors, `span_id`, splice, derived text blob id) is computed through the shared
  `prikk-store::text_span` module — no authoring-local span logic — so authoring and replay agree
  byte-for-byte (covered by an authoring↔replay symmetry test). Created files record a **normalized**
  canonical mode (4.4a-2aR): any executable bit set → `0o100755`, otherwise `0o100644`; symlink mode
  `0`; non-Unix defaults to `0o100644` (read/write/setuid/sticky bits and platform attributes are
  ignored). **Deferred to 4.4a-2b:** `ChangePerm` / mode-change detection for *existing* nodes (this
  increment preserves a modified file's baseline mode and emits no `ChangePerm`); the normalization
  rule it will reuse is the one ratified and landed here. Rename inference (moves author as
  delete+create) and symlink authoring (fails closed) also remain out of scope. The four previously
  DEV-only worktree-authoring tests are migrated to node-addressed `CreateFile` baselines and
  re-enabled (no `#[ignore]` remain), alongside witnesses for E1/E3/E4, deterministic patch identity,
  deletion, canonical mixed-operation `op_seq` ordering, created-file mode normalization (regular and
  executable), structured error classes, and authoring↔replay symmetry. The 4.4a-1 carry to remove
  `node_id_gen`'s module `#[allow(dead_code)]` is discharged now that the production path consumes the
  generator. Identity-neutral to existing objects; PATCH-framing anchors unchanged.

- **4.4a-2b — `ChangePerm` authoring (existing-node mode-change detection).** Completes node-addressed
  worktree authoring of the §9.3 mutation set by detecting permission changes on existing regular file
  nodes and emitting `ChangePerm`. Detection reuses the single `normalize_file_mode` rule landed in
  4.4a-2aR (no second normalization implementation): the worktree file's normalized canonical mode is
  compared against the replay-derived baseline node mode, and a difference emits exactly one
  `ChangePerm` with `old_mode` = baseline mode and `new_mode` = normalized worktree mode. Mode
  detection is independent of content, so a mode-only change authors a lone `ChangePerm`, while a
  content+mode change authors `ChangePerm` plus the content op; the existing canonical operation sort
  places `ChangePerm` before any `ReplaceBinary`/`EditText` (full kind order `DeleteNode` <
  `CreateFile` < `ChangePerm` < `ReplaceBinary` < `EditText`). Scope stays narrow (no rename
  inference, symlink authoring, or text↔binary transition): symlink nodes never reach mode detection
  (they live in the symlink baseline view and already fail closed), and symlink mode remains
  normatively `0`. New witnesses: mode-only → single `ChangePerm` with correct old/new modes;
  content+mode → `ChangePerm` before `EditText`; the mixed-operation ordering test extended to all
  five kinds (`[Delete, Create, ChangePerm, ReplaceBinary, EditText]`). Stale rustdoc/comments flagged
  in review (snapshot-baseline wording, "rule not yet ratified", `node_id_gen` "unwired") are
  cleaned. Identity-neutral to existing objects; PATCH-framing anchors unchanged.

- **4.4a release-prep (pass 1) — authoring path-handling hardening + threat-model delta.** Worktree
  enumeration now converts OS→repo paths **strictly**: a non-UTF-8 OS path fails closed at the
  conversion boundary (`to_str().ok_or(...)`) instead of being lossily replaced before
  `RepoPath::parse`, so identity-bearing paths never derive from lossy bytes (review N2). Added a
  binary content+mode witness (`ChangePerm` before `ReplaceBinary`, review N1) and a non-UTF-8
  path-rejection test. A threat-model delta for the `worktree → authoring → object-store blobs → WAL
  patch` data flow was produced against FDD-04 v1.3 (no new trust boundary or asset class; existing
  controls cover it; residuals flagged: author signing is a dev placeholder pending real AUTHOR-role
  signing, and symlink boundary-1 wiring when symlink authoring lands). Identity-neutral; PATCH-framing
  anchors unchanged.

- **4.4a R1 — role-bound Ed25519 AUTHOR patch signing.** Closes the release-prep-1 R1 residual: authored
  patches are no longer signed with a development placeholder. Signing goes through an injected
  `AuthorSigner` boundary (`author_signing.rs`): the authoring engine builds the role-bound preimage via
  `Signature::signed_bytes(Ed25519, Patch, <unsigned patch object id>, Author, <caller key id>)` and the
  provider returns the detached signature bytes. The production provider `Ed25519AuthorSigner` produces a
  real Ed25519 signature through `prikk-crypto`; the sole worktree-authoring production entry
  `commit_worktree_changes_signed` requires an injected signer, and tests use an explicit deterministic
  Ed25519 signer. (Scope: this covers the node-addressed worktree/commit AUTHOR path. The seal/publication
  MAINTAINER signing path is a separate role handled in a later phase and is not part of this claim.) A
  verification test proves the authored signature verifies against the signer's public key and fails if
  the object id, signer role, or key id changes (the algorithm negative is vacuous in v1 — `Ed25519` is
  the only `SignatureAlgorithm` — but the algorithm is bound in the preimage). Trust stores, key
  persistence/rotation, and signature policy remain out of scope (later phases). Identity-neutral to
  existing objects; PATCH-framing anchors unchanged.

- **4.4a R1R — remove the broken `commit --allow-empty` scaffold.** The `--allow-empty` empty-commit path
  built a **zero-operation** patch, which canonical encoding rejects ("patch operations must contain at
  least one operation") *before* signing — so it never produced a valid patch, and it was the last
  remaining AUTHOR placeholder-signature production path (`dev_author_signature` in `prikk-cli`). Because a
  zero-operation patch is not representable and cannot be signed, the scaffold could not be converted to
  real signing; it is removed instead (`empty_patch_envelope`, the `--allow-empty` flag, the `CommitMode`
  enum, and the placeholder helper are deleted). `prikk commit` now always authors a node-addressed patch
  from the worktree (`--from-worktree` accepted as a no-op for compatibility) with a real role-bound
  Ed25519 AUTHOR signature. This removes the AUTHOR placeholder from the `commit` path. (An AUTHOR-role
  marker remains on the rollback-draft path and is scoped in R1R2 below; the seal MAINTAINER placeholder is
  unaffected and remains a later-phase item.) Identity-neutral; PATCH-framing anchors unchanged.

- **4.4a R1R2 — rollback-draft AUTHOR signing scoped as internal (non-publishable).** Review R1R found a
  second AUTHOR-role placeholder: `rollback_draft.rs` signs the inverse Patch with
  `dev-placeholder-rollback-author` (a `SignerRole::Author` sha256 marker, not a real Ed25519 signature),
  on the `prikk rollback-draft --append-inverse` production path. Converting it to real AUTHOR signing is
  **design-blocked**: that key is a *load-bearing marker* — `rollback_verify` (`is_rollback_draft_envelope`,
  `verify_rollback_marker`) uses it to distinguish rollback-draft patches from ordinary authored patches in
  the active WAL. Signing with a real key would erase the marker and break rollback verification, and every
  clean replacement (a payload/precondition marker, an intent field — which the design mandates be
  advisory-only — or a WAL-record kind) is an identity-bearing/FDD-level decision, not a signature swap. So
  for this cut the rollback-draft path is **explicitly scoped as an internal development scaffold that is
  not publication-grade authoring**, per the review's accepted fallback. The accurate release-scope claim is
  therefore narrowed: node-addressed **worktree `commit`** patches are role-bound Ed25519 AUTHOR-signed;
  rollback-draft patches are an internal scaffold and are excluded from the publishable-authoring surface, as
  are MAINTAINER publication signing and trust-store enforcement. A proper fix (separate the rollback-draft
  marker from the author signature, then sign with the real key) is deferred to a design pass in the later
  crypto/policy phase. No code identity change; PATCH-framing anchors unchanged.

- **4.4-2c-4 — shared `text_span` module + public §5.1 golden vectors.** Promotes the
  identity-bearing §5.1 text-span primitives out of the replay module into a single shared
  `prikk-store::text_span` module — `TEXT_ANCHOR_WINDOW`, `anchor_hash`/`left_anchor`/`right_anchor`,
  `compute_span_id`, `occurrences`, `locate_text_span`, `text_blob_id`, the `TextSpanResolutionFailure`
  taxonomy, and a new **bounds-checked** `splice_text` (E1) — so authoritative replay and (later)
  worktree authoring compute the full `text → anchor-filtered localization → splice → BlobPayload(Text,
  new_text) id` chain through one implementation and cannot drift. Replay's `apply_edit_text` now calls
  the shared module; no §5.1 primitive remains in `replay.rs`. Lands public golden conformance vectors
  (`text_span/vectors.rs`, FDD-01 §5.1 naming) pinning literal anchor hashes, span ids, localized
  ranges, resulting text, and derived blob ids across boundary clamps, empty/zero-length insertion,
  overlapping occurrences, duplicate-raw/different-anchor and duplicate-anchor-filtered cases, plus
  `AnchorMismatch`/`NoMatchingSpanId` negatives. Pure move: existing replay EditText tests pass
  unchanged against the shared module; PATCH-framing anchors unchanged.
- **4.4-2c-3 — payload-retaining single-read lineage walk (E4).** The shared lineage walk now
  carries an associated `Block` type, so each lineage block is read **once** and the walk returns
  what it read: `ReaderLineage::Block = BlockPayload` (replay applies patches from the retained
  payload, no second `read_block`), `ResolverLineage::Block = Vec<ObjectId>` (provenance maps to ids
  for the window hash). This removes the prior ids-then-re-read double read, closing the file-backed
  concurrent-mutation hazard before any command-path consumer. The single shared walk rule
  (single-parent, cycle, terminus = horizon, apply order) and all acceptance/rejection behavior are
  unchanged. Witnessed by a counting-reader test (one read per lineage block) and a guard that
  panics on any second block read. Identity-neutral; anchors unchanged.
- **4.4-2c-2eR2 — baseline-mismatch classification (review erratum).** `certified_compared_cache`
  now binds the caller's intended baseline explicitly up front and returns
  `CacheCertificationError::BaselineMismatch` directly, symmetric with `HorizonMismatch`, instead of
  letting a caller/cache baseline mismatch fall through the validator as `CacheRejected`. Test
  updated to assert `BaselineMismatch`. Identity bytes unchanged.
- **4.4-2c-2eR — certification errata (review carry).** Folds review errata E1–E3 on the 2c-2e
  producers. E1: cache certification now returns a structured `CacheCertificationError`
  (`BaselineMismatch` / `HorizonMismatch` / `CacheRejected` / `ReplayUnavailable` / `ContentMismatch`)
  instead of flat integrity strings, so a future consumer can branch a droppable cache fault from
  authoritative-history unavailability; a `From<CacheCertificationError> for PrikkError` keeps the
  flattened boundary. E2: `certified_compared_cache` now binds the caller's intended
  `lineage_horizon_id` explicitly (fails closed up front) just as it binds the baseline. E3:
  documents that the compare certifies only the live/tombstone lifecycle state — `snapshot_blob_id`
  is **not** certified and must not back materialization acceleration without its own validation.
  Identity bytes unchanged; producers remain `pub(crate)` and unwired. E4 (double block-read
  stability before mutable file-backed command use) is carried as an explicit pre-command-path gate.
- **4.4-2c-2e — replay-derived state exposure + compared-cache wiring + unified lineage walk.**
  Adds the sanctioned producers `replay_derived_state` (rung 3: authoritative replay wrapped through
  `ReplayDerivedLifecycleState::from_replay`, which validates internal consistency before exposure)
  and `certified_compared_cache` (rung 4: validate → replay → full compare; the only cache-derived
  rung permitted to accelerate identity decisions, and only because it is proven equal to replay —
  never a root of trust). Unifies cache provenance and authoritative replay on a single lineage
  definition: both now walk via the shared `walk_single_parent_chain` over a `LineageBlockReader`
  seam (reader-backed for replay, parent-resolver-backed for provenance), so the two cannot drift on
  which blocks are in the window or in what order. Provenance's "genesis-before-horizon" and
  "horizon-not-genesis" failures now collapse to the single `HorizonNotInLineage` terminus rule
  (behavior identical — both still fail closed; only the message changed). Also folds review
  carry-forward C3 (symmetric saturating `right_anchor` arithmetic). Producers are `pub(crate)` and
  unwired by design.

- **4.4-2c-2d — EditText state effect (forward).** Replay now applies `EditText` exactly:
  materializes the node's current text (lazily, via a new blob-content resolver; cached per pass),
  localizes the span with the FDD-01 §5.1 64-byte anchor-filtered rule, splices in
  `replacement_text`, derives the new `BlobPayload(Text, new_text)` content id, and records it
  (`NodeLifecycleState::set_text_blob`), preserving `node_id`, path, and mode. Adds the structured
  `TextSpanResolutionFailed { node_id, span_id, reason }` class. **All** lifecycle-affecting
  operations now have exact effects — no operation maps to `UnsupportedLifecycleEffect`. The 64-byte
  anchor window is recorded in the FDD-01 §5.1 clarification note. Folds in the E1 carry-forward
  (`ReplaceBinary` old-side blob negatives).
- **4.4-2c-2c — ReplaceBinary state effect.** Replay now applies `ReplaceBinary` exactly: both
  `old_blob_id` and `new_blob_id` are resolved and required to be `BlobKind::Binary` (missing →
  fail-closed, non-binary → inconsistent), the live node must be a `BinaryFile` currently
  referencing `old_blob_id`, and its blob is swapped to `new_blob_id` with mode preserved (new
  `NodeLifecycleState::replace_file_blob`). Only `EditText` now remains fail-closed
  (`UnsupportedLifecycleEffect`).
- **4.4-2c-2bR — DeleteNode/RenamePath persisted old-state assertions.** Exact replay now verifies
  a `DeleteNode` record's full preimage (path, kind, blob/mode or symlink target) and a `RenamePath`
  record's `old_path` against the replayed live node before mutating — via new
  `NodeLifecycleState::delete_node_checked` / `rename_node_checked`. A record whose old-state
  assertion disagrees with replayed reality is rejected (`InconsistentLifecycleEffect`) rather than
  silently tombstoning/renaming from live state. Closes review P1-1/P1-2.
- **4.4-2c-2b — lifecycle state-effect interpreter (Create/CreateSymlink/Delete/Rename/ChangePerm).**
  Replay now applies exact existence/path/kind/mode effects into a `NodeLifecycleState`:
  `CreateFile` (node kind resolved from its blob via the real store-backed resolver — the explicit
  boundary where authoritative store access enters the trust ladder, E1), `CreateSymlink`,
  `DeleteNode` (tombstone recorded from the live node, so it carries post-mutation content/mode per
  O1), `RenamePath` (preserves `node_id`), and `ChangePerm` (new `NodeLifecycleState::change_file_mode`,
  exact mode, old-mode cross-checked). `EditText` and `ReplaceBinary` still fail closed
  (`UnsupportedLifecycleEffect`); node-lifecycle apply failures map to the new
  `InconsistentLifecycleEffect` class. The reconstructed state is still **not** exposed as
  `ReplayDerivedLifecycleState` and consumed by no caller (that is 2c-2e). Adds a malformed/wrong-type
  patch negative (E2).
- **4.4-2c-2a — authoritative lifecycle replay: lineage walker + dispatch skeleton.** Walks the
  v1 single-parent block lineage from a baseline back to a genesis horizon over the real object
  store, failing closed on missing/unreadable blocks, merge windows, cycles, and a genesis that is
  not the claimed horizon. Dispatches each block's patch operations; per the O1 ruling no state
  effect is implemented yet, so every operation fails closed (`UnsupportedLifecycleEffect`) and no
  `ReplayDerivedLifecycleState` is produced. Lands the structured replay error taxonomy (P2-3)
  ahead of any caller branching on it.
- **4.4-2c-1 — store-backed lifecycle resolvers.** Real implementations of the lifecycle-cache
  `BlockParentResolver` and `BlobKindResolver` over the object store (generic over
  `ObjectReader`). Closes P2-1: a missing or unreadable block is an error, never genesis — only
  a decoded `Block` with zero parents is genesis. A missing blob returns the fail-closed
  `Ok(None)` sentinel; a present-but-wrong-type object is an error. No replay, no cache use, no
  identity decision in this increment.

## 0.1.3 — Documentation / release hygiene

Documentation-only release. No source code change; identity anchors unchanged
(empty-PATCH `510ab866…5157`, populated `24031b48…c854`).

- Replaced `README.md` (maintainer-updated).
- Folded the v0.1.2 release-note errata: the four ignored `prikk-store` tests are now
  explained as DEV-ONLY worktree-authoring checkpoint tests, and worktree-authoring re-enable
  is added to the carry-forwards.
- CHANGELOG hygiene: removed a duplicate top heading and consolidated the v0.1.2 sub-slices
  under a single `0.1.2` release heading.

## 0.1.2 — DC-09 Phase 4.3 / 4.4 internal node-model groundwork

Internal/unwired node-model groundwork: store decode-model promotion, the node-lifecycle
substrate, and the lifecycle-cache trust ladder. Not consumed by any command path; identity
anchors unchanged.

### Phase 4.4 step 2b.2R-2 — create_node nonzero guard

Pre-threading substrate hardening from the 2b.2R review (P2): `NodeLifecycleState::create_node`
now rejects the reserved all-zero `node_id` at the central node-introduction boundary,
matching `seed_live_node` / `seed_tombstone`, instead of relying on decode/generator
correctness. Validation-only; both anchors unchanged. Test: `create_node_rejects_all_zero_node_id`
(restoration-equivalent re-create with a nonzero id continues to clear the tombstone).

### Phase 4.4 step 2b.2R — live/tombstone overlap closure

Closes a substrate P1 found in the steps 3–4 review: `NodeLifecycleState` could hold a node
as both live and tombstoned after delete → restoration-equivalent re-create, which violated
the cache's no-overlap invariant and would make replay-and-compare reject a correct post-
restore cache. Model/validation correction only; both anchors unchanged.

- `create_node` now clears any tombstone for the node on a restoration-equivalent
  reintroduction, so live and tombstone sets stay disjoint (no-op for a fresh node_id).
- `NodeLifecycleState::validate_internal_consistency` now rejects any node_id present in both
  the live and tombstone sets.
- `ReplayDerivedLifecycleState::from_replay` now returns `Result` and validates internal
  consistency, so the compared rung cannot certify against a malformed reference state.
- Tests: substrate `create -> delete -> restore` leaves the node live with no overlap and
  passes consistency; a post-restore baseline cache (node live, no tombstone) compares equal
  to the replayed state.

### Phase 4.4 step 2b.2-3/4 — replay-derived + compared rungs

Adds the top trust rungs and the decisive right-provenance/false-tombstone guarantee. Still a
private, unwired slice — no apply/seal/verify path consumes a cache. Additive; both anchors
unchanged.

- **`ReplayDerivedLifecycleState`** — an authoritative replay-derived `NodeLifecycleState`
  bound to a baseline. Must be produced only by authoritative replay over the walked chain;
  the real producer arrives with threading, so this slice constructs it via `from_replay`.
- **`ComparedLifecycleCache`** — a validated cache **proven equal** to authoritative replay
  for the same baseline. `from_validated_and_replay` checks the baseline matches, rebuilds a
  `NodeLifecycleState` from the validated cache, and requires it to equal the replayed state.
  This is the only cache-derived rung that may participate in restoration-equivalence /
  `node_id` reuse decisions once wired — and only because it equals replay.
- **Decisive guarantee:** a cache with correct provenance but false live/tombstone contents
  is rejected — the rebuilt state will not equal the replayed state (test:
  `compared_rejects_false_tombstone`).
- **P2-2 closed:** `ValidatedLifecycleCache::from_decoded_for_baseline` binds a cache to the
  caller's intended baseline, so a cache valid for one checkpoint cannot be accepted where a
  different baseline was meant.
- `NodeLifecycleState` now derives `PartialEq`/`Eq` for the replay comparison.
- Carry-forwards still open: P2-1 (real store resolver must distinguish a missing/unreadable
  block from genesis — applies when the real resolver lands in threading) and P2-3 (structured
  error taxonomy before recovery/doctor branches on classes).

### Phase 4.4 step 2b.2-2 — walked-chain provenance

Makes lifecycle-cache provenance **operational**: the `replay_window_hash` is recomputed
over the actually walked single-parent block chain, never over cache-supplied data. Still a
private, unwired slice — no apply/seal/verify identity decision uses a cache. Additive; both
anchors unchanged.

- **`BlockParentResolver`** — a `block_id -> Vec<ObjectId>` seam (parents in seal order;
  empty at genesis), mirroring `BlobKindResolver`. Keeps the walk testable without a store
  handle; the real `Block`-reading resolver arrives with threading.
- **`DecodedLifecycleCache::verify_window_against_chain`** — walks `baseline_block_id` back
  to `lineage_horizon_id` over single-parent links and recomputes the window hash from the
  walked chain. Fails closed on a merge (multi-parent) block, a cycle, reaching genesis
  before the claimed horizon, a horizon that is not repository genesis (v1 adequate-horizon
  rule), or a hash mismatch.
- **`ValidatedLifecycleCache::from_decoded`** now also runs provenance verification (it takes
  both a blob and a block-parent resolver), so the `Validated` rung means structural +
  operational-provenance + blob-kind verified — design-v3's definition — and cannot exist
  with merely syntactic provenance.
- Tests: matching walked chain accepted; window-hash mismatch, merge block, non-genesis
  horizon, cycle, and genesis-before-horizon each rejected.

### Phase 4.4 step 2b.2-1 — blob-kind verification + Validated rung

Opens 4.4-2b.2 proper with the first blob-kind verification step and the first trust rung.
Still a private, unwired codec/import slice — no apply/seal/verify identity decision uses a
cache. Additive; both anchors unchanged.

- **`BlobKindResolver`** — a small `blob_id -> Option<BlobKind>` trait. `Ok(None)` means the
  blob is absent/unreadable and fails closed. Keeps verification testable without a store
  handle; a real store resolver arrives with the threading slice.
- **`ValidatedLifecycleCache`** — the first trust rung: a `DecodedLifecycleCache` whose every
  file entry's `NodeKind` has been checked against the referenced blob's `BlobKind`, reusing
  the canonical `NodeKind::from_file_blob_kind` rule. `from_decoded` **re-runs structural
  validation itself** (the input is not trusted to have come from `decode`, since fields are
  `pub(crate)`), then verifies blob kinds; a missing blob, a kind disagreement, a `SNAPSHOT`
  blob, or a resolver error fails closed. It is documented and structured as **not authority**
  for `node_id` reuse or restoration-equivalence — there is no method that yields such a
  decision; those wait for the replay-derived / replay-compared rungs.
- Tests: structural-invalid input rejected even when blob kinds resolve; Text and Binary
  matches accepted; Text↔Binary disagreement rejected; `SNAPSHOT` blob rejected; missing blob
  rejected; tombstone blob-kind mismatch rejected; resolver error propagated fail-closed.
- Review follow-ups: added explicit tombstone kind/content production-encode negatives (N1);
  verified `read_enum_u16` guards the wire type exactly once — the apparent double was two
  distinct call sites (node_kind tag 3, parent_policy tag 4), nothing to remove (N2).

### Phase 4.4 step 2b.2 — lifecycle cache codec hardening

Corrective patch opening 4.4-2b.2, closing the 4.4-2b.1 review errata. Validation-only;
no new data, no wiring into replay; both anchors unchanged.

- **P1 — production `encode()` validates before writing.** `encode()` now runs the same
  structural/cross-set `validate()` as `decode()` before serializing, so an internal
  caller cannot persist a cache the importer would later reject. `validate()` is now
  **structurally equivalent** to the decode path: beyond schema/policy/sorting/uniqueness
  and `seen_ids == live ∪ tombstoned`, it rejects the reserved all-zero `node_id` in live,
  tombstone, and `seen_ids` sets and rejects any kind/content discriminator mismatch,
  reusing the substrate's `ensure_node_id_nonzero` and `validate_kind_content_shape`
  (promoted to `pub(crate)`) rather than a parallel rule. The raw serializer is private and
  reachable in production only through the validated `encode`; a `#[cfg(test)]`
  `encode_unchecked` is used to craft malformed fixtures for decode negatives. Production
  encode is proven to reject unsorted live entries, a `seen_ids` mismatch, merge policy,
  all-zero ids (live/tombstone/seen), and file↔symlink kind/content mismatches.
- **P2-1 — non-canonical TLV tag order rejected.** Decode now requires nondecreasing field
  tags at both the top level and inside each node record (repeated tag 10/11 entries still
  allowed in-region), so a persisted cache has one canonical byte form. Tests cover a
  header field and a node-record field presented out of order.
- **P2-2 (error taxonomy)** remains a message-class mapping for now, per the review — to be
  promoted to a structured class before any recovery/doctor path depends on the outcomes.

### Phase 4.4 step 2b.1 — lifecycle cache codec

Adds the persisted lifecycle-cache wire format and its decoder/importer
(`lifecycle_cache`), a derived, rebuildable accelerator for `NodeLifecycleState`. Per
design v3 §0 the decoded value is **not validation authority**: it cannot seed a
`node_id`-reuse decision. Additive and identity-neutral; not wired into replay; both
anchors unchanged.

- `DecodedLifecycleCache::{encode, decode}` over `PRIKK-NODE-LIFECYCLE-CACHE-v1\0` magic
  plus canonical `FieldRecord` TLV. Wrong/short magic is rejected before TLV decode;
  repeated live/tombstone entries use `record_list_item` (`0x21`) and an entry sent as a
  plain `record` (`0x20`) is rejected.
- Fail-closed structural + cross-set validation: unknown top-level/nested tags, duplicate
  singleton fields, file/symlink discriminator (files require `blob_id`+`normalized_mode`
  and forbid a target; symlinks require a target and forbid `blob_id`/field 5 even when
  zero), live entries strictly sorted by canonical `repo_path` with unique path and id,
  tombstones strictly sorted by raw `node_id`, `seen_ids` a multiple of 32 / strictly
  ascending / nonzero, no id both live and tombstoned, and `seen_ids == live ∪ tombstoned`.
- `compute_window_hash` fixes the exact `replay_window_hash` preimage
  (`PRIKK-LIFECYCLE-CACHE-WINDOW-v1 || u64be(count) || raw32(block_id)…`): deterministic,
  count-bearing, order-sensitive, domain-separated.
- Blob-kind verification, provenance-vs-baseline staleness, replay reconstruction, and
  replay-and-compare are deferred to the next slice; no `ValidatedLifecycleCache` /
  `ReplayDerivedLifecycleState` / `ComparedLifecycleCache` ladder is exposed yet, so no
  type here can be mistaken for replay-derived authority.

### Phase 4.4 step 2a — baseline seeding substrate

Adds the baseline-seeding API to `NodeLifecycleState` and closes the substrate-level
4.4-2 errata, so a baseline cache cannot inject node state an operation could not.
Additive and identity-neutral; both anchors unchanged.

- `seed_live_node` / `seed_tombstone` seed the live clean tree and the non-live
  lifecycle history (`seen_ids` + `latest_tombstone_by_id`) needed for
  restoration-equivalence across a snapshot boundary. Both reject the reserved
  all-zero `node_id` (erratum P1-3), validate the kind/content discriminator through a
  shared `validate_kind_content_shape` (erratum P2-2), and reject duplicate live ids,
  duplicate live paths, and tombstones for currently-live nodes.
- `validate_internal_consistency` now also requires every live and every tombstoned
  `node_id` to be recorded as seen (erratum P1-4, whole-state check).
- `rename_node` gains the same path-index lockstep guard as `delete_node` (erratum
  P2-1), failing closed rather than silently healing a desynchronised index.
- Tests raised to 24: cross-boundary restoration-equivalence accept and non-equivalent
  reject (the identity-resurrection case), all-zero seed rejection, duplicate id/path
  rejection, tombstone-for-live rejection, and seed kind/content rejection.
- Deferred to the next slice (cache format + threading): cache provenance/staleness
  binding (P1-1), the materialization-bytes vs lifecycle-identity payload split (P1-2),
  the symlink `normalized_mode == 0` parse check, and threading `NodeLifecycleState`
  through replay/inverse/rollback.

### Phase 4.4 step 1 — node lifecycle substrate

Introduces the node-aware replay substrate. Additive and identity-neutral: a new
isolated module with no changes to any object/payload/encode path; both identity
anchors are unchanged.

- Added `prikk-store::node_lifecycle`: a derived, rebuildable `NodeLifecycleState`
  (`live_by_id` / `path_to_id` / `latest_tombstone_by_id` / `seen_ids`) that is
  explicitly **not a root of trust** (FDD-02 §12). It centralises the node rules so
  replay/inverse/rollback cannot diverge on them: per-`CleanTree` live-node
  uniqueness, rejection of currently-live `node_id` reuse, restoration-equivalence of
  a non-live reintroduced `node_id` to its latest deletion preimage (kind, content
  payload, mode, path — non-liveness necessary but not sufficient, DC-09a §4), and
  `node_id` preservation across rename.
- Review errata: `create_node` fails closed on an inconsistent kind/content
  discriminator (symlink-as-file or file-as-symlink); the path index is keyed by the
  canonical `RepoPath`; `delete_node` enforces `live_by_id`/`path_to_id` lockstep; and
  a `validate_internal_consistency()` helper checks the live-node bijection (for
  assertions and the 4.6 deep-verify validator).
- 17 unit tests covering uniqueness, live-reuse rejection, restoration-equivalence
  (file accept plus blob/mode/path/kind-mismatch rejects; symlink target match and
  mismatch), kind/content discriminator rejection, rename id-preservation and
  occupied-target rejection, non-live delete/rename rejection, and the consistency
  helper.
- The module is `dead_code`-allowed at declaration: it is threaded into the replay
  pipeline in the next 4.4 step (which first settles how the clean-tree baseline
  carries node identity).

### Phase 4.3 — store decode-model promotion

Promotes the store patch decoder from a two-variant, path-keyed supported subset
into a typed node-addressed stream over all seven FDD-03 §9.3 operation kinds.
Identity-neutral: the empty-PATCH anchor and populated framing vector are unchanged.
Applies design-review errata P1 (decode success must not imply apply support) and P2
(retain validated `op_seq`).

- Replaced `SupportedPatchOperation` with `DecodedPatchOperation { op_seq, kind }` and a
  seven-variant `DecodedOperationKind` (plus a discriminated `DecodedDeletePreimage`),
  and renamed `decode_supported_patch_operations` -> `decode_patch_operations`. Every
  well-formed §9.3 kind now decodes into its typed variant; symlink `DeleteNode` and the
  four other node-addressed kinds are no longer rejected at decode time.
- Added `ensure_apply_supported` as the single apply-support gate (erratum P1): decode
  is structural, applicability is a separate decision. Audited all callers — `patch_replay`
  apply and `patch_inverse` derivation gate before matching; `rollback_verify` now gates
  each decoded operation explicitly rather than relying on decode success to prove
  replayability.
- Retained validated `op_seq` in the decoded wrapper (erratum P2).
- Migrated decode tests: malformed/oneof/all-zero-`node_id`/wrong-wire negatives remain
  decode errors; each of the seven well-formed kinds asserts its typed decoded variant
  **and full decoded field values** (review erratum E1, so 4.4 application can depend on
  them), and the not-yet-wired kinds assert `UnsupportedObjectType` at the apply gate.

## 0.1.1 Housekeeping

Repository structure and developer-ergonomics pass. No identity-byte or behavior
changes; the empty-PATCH anchor and populated framing vector are unchanged.

- Relocated `prikk-store` unit tests from the central `src/tests/` directory to the
  project-standard co-located layout: `src/<module>/tests.rs` (and
  `src/patch_replay/tests/` for the three patch-decode test modules). Shared fixtures
  and cross-module harnesses moved to a single `src/test_support.rs`.
- Added `rfcs/proposed/` with a node-model plan RFC capturing the deferred
  application work (4.3–4.6) and the tracked carry-forward items (symlink target
  validator, duplicate scalar-field rejection, preconditions).
- Aligned the workspace `Cargo.toml` version (`0.1.0` -> `0.1.1`) with the active
  CHANGELOG line.
- Made the worktree-patch test module pass the CI clippy gate
  (`cargo clippy --workspace --all-targets -- -D warnings`): targeted
  `#[allow(clippy::indexing_slicing)]` on the four DEV-ONLY authoring-checkpoint tests
  (deliberate `Vec` indexing in assertions) and removed a needless borrow on a
  byte-slice literal.

## 0.1.0 DC-09 Phase 4.2

Operation-record identity reconciliation to FDD-03 §9.3 (code reconciliation effort,
architect-ratified across increments 4.2a–4.2e). Identity/read-validation surface
only; application of node-addressed operations is deferred to the node model.

- Reconciled all seven operation-kind payloads to their FDD-03 §9.3 records: `CreateFile`,
  `DeleteNode` (was `DeleteFile`), `EditText` (node-addressed, span-anchored, 9-field),
  `ReplaceBinary` (node-addressed), `RenamePath`, `ChangePerm`, `CreateSymlink` — all
  node-bearing records reject an all-zero `node_id` on encode and decode.
- Enforced the FDD-03 §9.2 operation-kind oneof on the read path (a record with more
  than one kind field is rejected as malformed, not decoded last-wins).
- Enforced the FDD-03 §9.2.1 `op_seq` canonical invariant on the read path
  (one-based, contiguous, unique, physical order == ascending `op_seq`).
- Added the `ReplaceBinary` binary-only blob-kind enforcement primitive
  (`ensure_blob_kind_is_binary`); wiring into real application is deferred to the node model.
- Retired the pre-FDD full-file `EditText` apply/inverse path and its worktree generation.
- **Worktree patch authoring (`commit --from-worktree`) is fail-closed in this snapshot**
  for create/delete/modify/text changes: every §9.3 mutation operation is node-addressed
  and requires node-id tracking and minting (deferred to increments 4.4/4.4a). This release
  does not support worktree authoring; replay of node-addressed operations is likewise deferred.
- Byte-level `(tag, value_type)` layout tests and read-side validator negatives added for
  every operation record; empty-PATCH anchor and the populated framing vector held throughout.

## 0.1.0 PR-030

Sealed rollback block/history classification after normal seal.

- Added sealed rollback block classification after rollback drafts are sealed by the existing seal path.
- `load_ref_history()` now reports `rollback_patch_count` and `is_rollback_block` for each history entry.
- `prikk log` now displays rollback block classification for sealed history entries.
- `verify_repository()` now counts sealed rollback blocks and sealed rollback-marked Patch objects.
- `prikk verify` now displays sealed rollback block and rollback Patch counts in addition to active rollback draft WAL records.
- Shared rollback Patch payload validation between active WAL verification and sealed Block/history classification.
- Fixed an obvious duplicate-parameter transcription defect in inverse planning source while touching rollback-adjacent code.
- Kept rollback-specific ref publication, rollback authorization, worktree rollback writes, arbitrary-span rollback, commutation, confluence, audit plugins, and sync deferred.

## 0.1.0 PR-029

Active rollback draft verification before seal.

- Added active rollback draft verification for the supported patch-operation subset.
- Added `verify_active_rollback_draft()` and `RollbackDraftVerification`.
- Added CLI command `prikk rollback-draft-verify [path] [--ref REF]`.
- Rollback drafts now use a dedicated development signature marker key: `dev-placeholder-rollback-author`.
- Repository verification now counts rollback draft WAL records and validates that rollback draft payloads decode under the supported replay subset.
- Kept seal publication, rollback refs, rollback authorization, worktree mutation, arbitrary-span rollback, commutation, confluence, audit plugins, and sync deferred.

## 0.1.0 PR-028

Conservative rollback draft append to an empty active WAL.

- Added conservative rollback draft append for the supported patch-operation subset.
- Added `append_rollback_draft()` and `RollbackDraftReport`.
- Added CLI command `prikk rollback-draft --append-inverse [path] [--ref REF] -m <message>`.
- Requires an explicit `--append-inverse` flag, a non-empty message, an empty active WAL, and no partial WAL tail.
- Appends a signed inverse Patch envelope to the active WAL only; ref publication remains the existing `seal --allow-no-audit` path.
- Kept rollback ref policy, authorization, worktree mutation, arbitrary-span rollback, commutation, confluence, audit plugins, and sync deferred.

## 0.1.0 PR-027

Non-mutating rollback preview for the supported patch-operation subset.

- Added non-mutating rollback preview for the supported patch-operation subset.
- Added `prepare_rollback_preview()` and `RollbackPreviewPlan`.
- Added CLI command `prikk rollback-preview [path] [--ref REF]`.
- Combines unsigned inverse planning with supported patch replay validation.
- Compares the current replayed target state with the latest snapshot baseline and reports `would-create`, `would-delete`, and `would-replace` file-level changes.
- Kept rollback refs, authorization policy, worktree writes, commutation, confluence, arbitrary-span rollback, audit plugins, and sync deferred.

## 0.1.0 PR-026

Read-only inverse planning for the supported patch-operation subset.

- Added read-only inverse planning for the supported patch-operation subset.
- Added `prepare_patch_inverse_plan()` and `PatchInversePlan`.
- Added CLI command `prikk inverse-plan [path] [--ref REF]`.
- Derives unsigned inverse Patch payloads for supported `CreateFile`, `DeleteFile`, `ReplaceBinary`, and full-file `EditText` operations.
- Reports an unsigned inverse Patch ID hint without writing or publishing it.
- Kept rollback refs, authorization policy, conflict witnesses, commutation, confluence, arbitrary-span inverse handling, audit plugins, and sync deferred.

## 0.1.0 PR-025

Opt-in full-file `EditText` generation from UTF-8 worktree modifications.

- Added opt-in full-file `EditText` generation from snapshot-baseline worktree modifications.
- Added `WorktreePatchCommitOptions` and `commit_worktree_changes_with_options()`.
- Added CLI support for `prikk commit --from-worktree --text-edits -m <message>`.
- Kept default `commit --from-worktree` behavior compatible: modified tracked files still emit `ReplaceBinary` unless text mode is requested.
- Text mode emits `EditText` only when both baseline and current file bytes are valid UTF-8; binary or invalid UTF-8 modifications fall back to `ReplaceBinary`.
- Added worktree patch tests for text edit emission and binary fallback.
- Kept arbitrary span discovery, text diff minimization, inverse, commutation, conflict witnesses, audit plugins, and sync deferred.

## 0.1.0 PR-024

Conservative full-file `EditText` replay for exact-span replacements.

- Added conservative `EditText` replay for full-file exact-span replacements.
- Added canonical decode support for `EditText` patch operations in the supported patch replay decoder.
- Added `full-file` anchor replay validation: current file bytes must be valid UTF-8 and must hash to the recorded `old_span_hash`.
- Split supported patch-operation decoding into `patch_replay/decode.rs` to keep the replay module within the project file-size guidance.
- Added a patch replay test for full-file text edit replay.
- Kept worktree text diff generation, arbitrary span discovery, inverse, commutation, conflict witnesses, audit plugins, and sync deferred.

## 0.1.0 PR-023

Explicit patch deletion planning and opt-in removal of files deleted by supported patch replay.

- Added a content-anchored text edit payload validation scaffold.
- Added fixed `TEXT_SPAN_HASH_BYTES = 32` and `text_span_hash(bytes)`.
- Added `validate_text_anchor_id()` for v1 anchor identifier validation.
- Changed `EditText.old_span_hash` to a fixed 32-byte value.
- Added tests for anchor validation, stable span hashing, and invalid anchor rejection.
- Fixed a replay-source transcription defect in the supported `ReplaceBinary` branch.
- Kept worktree text diff generation, text replay, inverse, commutation, conflict witnesses, audit plugins, and sync deferred.

## 0.1.0 PR-022

Explicit patch deletion planning and opt-in removal of files deleted by supported patch replay.

- Added read-only explicit deletion planning via `prikk checkout --patch-delete-plan`.
- Added opt-in deletion during supported patch materialization via `prikk checkout --patch-materialize-delete`.
- Deletion is limited to files explicitly removed by replayed `DeleteFile` operations.
- Deletion is refused unless the current worktree file bytes still match the operation's old Blob bytes.
- Arbitrary untracked files and modified deleted files are never removed.
- Added deletion planning/materialization tests and documentation.
- Kept general destructive pruning, text edits, renames, chmod, symlinks, merge/conflict algebra, audit plugins, and sync deferred.

## 0.1.0 PR-021

Opt-in supported patch replay materialization without destructive removals.

- Added opt-in supported patch replay materialization via `prikk checkout --patch-materialize`.
- Added `materialize_patch_checkout()` and `PatchMaterializationReport`.
- Reuses the PR-020 supported replay subset: `CreateFile`, `DeleteFile`, and `ReplaceBinary`.
- Writes only validated replay-result files through the same conservative materializer used by snapshot checkout.
- Refuses conflicting existing files and never deletes extra worktree files.
- Keeps destructive removal, content-anchored text edit replay, renames, chmod, symlinks, merge/conflict algebra, audit plugins, and sync deferred.

## 0.1.0 PR-020

Minimal worktree-to-patch draft generation for missing, modified, and untracked files, still without patch replay or full algebra.

- Added read-only supported patch replay planning via `prikk checkout --patch-plan`.
- Added `prepare_patch_replay_plan()` and `PatchReplayPlan`.
- Replays single-parent block chains from oldest to newest.
- Loads snapshot Blob baselines and applies supported `CreateFile`, `DeleteFile`, and `ReplaceBinary` operations.
- Verifies `old_blob_id` preconditions for delete/replace operations.
- Keeps text-span edits, renames, chmod, symlinks, merge/conflict algebra, and worktree writes deferred.

## 0.1.0 PR-019

Minimal worktree-to-patch draft generation for missing, modified, and untracked files, still without patch replay or full algebra.

- Added minimal worktree-to-patch draft generation from snapshot-baseline changes.
- Added `prikk commit --from-worktree -m <message>`.
- Emits file-level `CreateFile`, `DeleteFile`, and `ReplaceBinary` operations only.
- Writes Blob objects referenced by generated operations before appending the Patch envelope to WAL.
- Keeps rename detection, content-anchored text-span edits, patch replay, audit plugins, and sync deferred.

## Earlier PRs

See `rfcs/IMPLEMENTATION-STATUS.md` and `rfcs/done/PR-*-HANDOFF.md` for earlier implementation history.
