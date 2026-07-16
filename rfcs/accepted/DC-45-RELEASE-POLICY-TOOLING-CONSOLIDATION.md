# RFC (accepted) - DC-45 Release Policy Tooling Consolidation

**Status.** Accepted after architect design repair re-review v1 on 2026-07-16; staged implementation
may begin with pre-oracle profile hardening and the Python observation adapter.
**Target milestone.** M2 - first tooling increment; cutover required before the 0.19.0 release
candidate.
**Tracks.** Architect direction on DC-35 release-policy tooling debt.
**Touches.** `tools/release-policy/`, root Cargo workspace metadata and lockfile, `release/` policy
contracts and vectors, release-gate documentation, and package/publication selection. No product
behavior, release-signer authority, credential, tag, package, or publication change.

## Problem

DC-35 established an accepted, fail-closed release-policy gate. Its Python implementation is a valid
correctness baseline, but `release/` now mixes public contracts, an executable policy engine, fixture
mutation logic, and synthetic evidence documents. That ownership shape raises maintenance and review
cost, and the focused Python JSON Schema evaluator duplicates a security-sensitive standards boundary.

The project needs one maintained Rust policy tool inside the existing workspace while preserving the
accepted DC-35 behavior and its language-neutral public contract. This is a consolidation and
migration increment, not authority to rewrite DC-35 policy.

## Schedule and dependencies

DC-45 is the first M2 tooling increment. Its design must be accepted before the first release signer is
bootstrapped, but completing the Rust migration is not a signer-bootstrap prerequisite. Until cutover,
the accepted Python command remains the authoritative executable gate and signer bootstrap may use it
under the separately reviewed DC-35 governance transaction.

The Rust cutover must be accepted before the 0.19.0 release candidate. DC-41 and DC-42 measurement and
design may proceed in parallel. DC-43 design may proceed, but DC-43 implementation must consume the
stable post-cutover command rather than extend the Python engine. A newly discovered release-policy
correctness or security defect immediately blocks bootstrap or release under DC-35 regardless of this
schedule.

## Ownership boundary

Add a workspace member at `tools/release-policy/` with package name `prikk-release-policy` and
`publish = false`. It is a focused release-policy binary, not a general `xtask` dispatcher.

The package inherits `version.workspace = true`; its version follows the workspace source line even
though it is not published. It has exactly one binary target named `prikk-release-policy`, inherits
Rust 1.85, edition 2024, and workspace lints, and has no nested `Cargo.lock`. The package must:

- use the root `Cargo.lock`, workspace Rust 1.85 minimum, edition, and lint policy;
- have no dependency edge in either direction between itself and any of the seven product crates;
- participate in workspace test, check, Clippy, build, and dependency-audit gates;
- remain outside root `default-members`, which must explicitly remain the seven product crates; and
- expose one documented command, initially `cargo run --locked -p prikk-release-policy -- check`.

Package and publish procedures must explicitly name the seven product packages. They must not infer the
publication graph from all workspace members. The allowlist, in DC-35 publication order, is
`prikk-error`, `prikk-hash`, `prikk-crypto`, `prikk-object`, `prikk-replay`, `prikk-store`, and `prikk`.
The repository source archive includes the tracked tool, contracts, oracle manifest, and required exact
vectors so the release gate is reproducible; no product `.crate` package may contain the tool. Added
parser and schema dependencies use the shared lockfile, pass the MSRV gate, and enter the project
dependency policy when that policy exists.

### Executable Cargo and archive boundary

The Rust tool provides `boundary-check --format json`. It consumes structured `cargo metadata
--locked --format-version 1` output, structured TOML for inheritance declarations that resolved Cargo
metadata cannot expose, and Cargo package/archive listings. It emits only these stable failure
categories: `workspace-members`, `default-members`, `tool-metadata`, `lockfile-boundary`, `dependency-
boundary`, `publication-allowlist`, `package-contents`, and `source-archive-contents`. Multiple failures
are sorted by that order and then package/path byte order.

The command asserts:

- workspace members are exactly the seven named product package ids at their current `crates/*`
  manifests plus `prikk-release-policy` at `tools/release-policy/Cargo.toml`;
- `workspace_default_members` is exactly the seven product package ids;
- tool metadata reports `publish = false`, Rust 1.85, edition 2024, and exactly the expected binary
  target, while structured manifest parsing proves `version.workspace = true`,
  `edition.workspace = true`, `rust-version.workspace = true`, and `lints.workspace = true`;
- no `Cargo.lock` exists below `tools/release-policy/`;
- no product package depends transitively on the tool, and the tool has no direct or transitive local
  dependency on a product package;
- the publication allowlist and dependency order are exactly the seven-package DC-35 graph above, and
  every package/publish command names packages from that allowlist rather than using `--workspace`;
- `cargo package --locked --list -p <product>` for every product excludes
  `tools/release-policy/`, the oracle manifest, and private policy vectors; and
- a deterministic source-archive listing for the reviewed commit includes
  `tools/release-policy/`, the normative schema, oracle manifest, and every manifest-required exact
  vector.

Filename searches may support diagnostics but cannot replace Cargo metadata and Cargo package listings
as package-graph authority. Implementation review records the selected parser/schema crates, exact
versions/features, license, maintenance and advisory status, Rust 1.85 compatibility, transitive impact,
and offline behavior.

## Contract authority

`release/schemas/release-evidence-v1.schema.json` remains the normative, language-neutral structural
contract. The Rust tool must execute that schema with a mature Draft 2020-12 implementation. Rust data
types and strict deserialization provide defense in depth but cannot generate, replace, weaken, or
silently reinterpret the public schema.

The implementation must keep these layers explicit:

1. JSON Schema assertions define public document structure.
2. Typed local invariants validate values represented within one parsed document.
3. Semantic checks validate cross-document relations and exact-byte identity.
4. External observations cover Git, OpenPGP, filesystem, clock, network, registry, release-page, and
   hosted-service behavior and are not proven by the policy core.

Schema-validator conformance cases remain in the migration oracle until the custom Python evaluator is
removed. Any future schema-generation model requires a separate compatibility and versioning ruling.

### Offline schema-validation profile

The profile is `release-schema-profile-v1`. It compiles the committed schema as Draft 2020-12 using its
literal `$schema`, validates the schema against the implementation's bundled Draft 2020-12 meta-schema,
and fails with `validator-error` before validating instances if compilation or schema self-validation
fails. The selected validator runs offline: network retrieval is disabled, only local fragment
references beginning `#/` are accepted, JSON Pointer unescaping follows RFC 6901, and an unresolved,
nonlocal, unsupported, or cyclic reference is `validator-error`. Resolution starts from the exact
committed schema bytes named by the oracle manifest, never a fetched `$id` resource.

The profile enables `format` assertions. Every schema location carrying `format: "date-time"` also
receives the project profile assertion for exactly `YYYY-MM-DDTHH:MM:SSZ` and a real Gregorian date/time.
Offsets, lowercase `z`, fractional seconds, missing seconds, impossible dates/times, and leap seconds
are invalid. Boundary vectors include a valid UTC-second value and each rejected form. This preserves
the accepted Python canonical-time rule without replacing the normative schema; changing that rule or
encoding it into the schema requires a separate reviewed contract amendment.

Before schema compilation, a recursive vocabulary preflight permits exactly the keywords used by the
accepted evaluator: `$schema`, `$id`, `$ref`, `$defs`, `title`, `type`, `additionalProperties`,
`required`, `properties`, `const`, `enum`, `pattern`, `items`, `minItems`, `maxItems`, `uniqueItems`,
`allOf`, `oneOf`, `if`, `then`, `minLength`, `minimum`, and `format`. An unknown keyword at any schema
position is `validator-error`; adding vocabulary requires a new reviewed profile version.

The manifest, schema, fixture tables, and raw evidence inputs reject duplicate JSON object names at any
nesting depth before ordinary decoding. Because commit `12c137d` does not enforce that raw-input rule,
the duplicate-name preflight and its boundary vectors are a separate reviewed pre-oracle contract
hardening. It must prove all `12c137d` corpus outcomes remain identical while adding fail-closed
duplicate-name cases. The frozen manifest records both `python_baseline_commit = "12c137d"` and the
accepted `profile_contract_commit`; Rust implementation cannot begin before that review.

## Frozen migration oracle

Before Rust policy implementation, freeze `release/oracle/oracle-manifest-v1.json` and validate it
against a committed strict schema. Its top-level `schema_version` is the literal
`oracle-manifest-v1`; it records `python_baseline_commit = "12c137d"`, the accepted
`profile_contract_commit`, `reason_taxonomy_version = 1`, and the normative schema's repository-relative
path, byte length, and lowercase SHA-256.

Cases are keyed by a globally unique `(suite_id, case_id)` pair. Both identifiers are nonempty ASCII
lowercase kebab-case, cases are strictly sorted by that pair, and duplicate, missing, or extra cases
fail closed. Each `inputs` array is nonempty and sorted by unique zero-based `ordinal`. Every input has
an explicit closed `role`, ordinal, repository-root-relative UTF-8 path, byte length, and lowercase
SHA-256. The role enum is `authority`, `fixture-table`, `schema`, `challenge`, `prior-snapshot`,
`current-snapshot`, or `expected-output`.

Paths use `/`, contain no empty, `.` or `..` segment, are not absolute, and after symlink-aware
canonicalization remain beneath the repository root. Missing inputs, extra declared/observed inputs,
duplicate roles where a suite requires one, digest/length mismatches, unsupported versions, and
unrecognized fields fail before policy validation.

Every case records `expected.structural`, `expected.semantic`, `expected.final`, and
`expected.case_outcome`. Structural and semantic stage values are `valid`, `invalid`, `not-run`, or
`validator-error`; final is `valid`, `invalid`, or `validator-error`. `case_outcome` preserves the
fixture-visible token and is `valid`, `valid-local-only`, `invalid`, or `validator-error`.
`valid-local-only` is allowed only for the release-state suite and has final `valid`. `not-run` is
required for every stage after the first `invalid` or `validator-error`. Final is `validator-error` if
any executed stage has that value, otherwise `invalid` if any executed stage is invalid, otherwise
`valid`.

`expected.primary_reason` uses this closed version-1 enum, in precedence order:

1. `manifest-contract`
2. `input-identity`
3. `json-syntax-or-duplicate-name`
4. `schema-profile-or-compilation`
5. `schema-instance`
6. `authority-grammar`
7. `challenge-grammar-or-binding`
8. `challenge-time-window`
9. `governance-transition-or-proof`
10. `governance-review-or-hold`
11. `release-state`
12. `evidence-byte-identity-or-link`
13. `evidence-transition-or-attempt-prefix`
14. `evidence-tag-or-artifact`
15. `evidence-completion`
16. `none`

Validators collect applicable failures and choose the earliest enum member; ties use normalized JSON
Pointer byte order and then rule-id byte order. `none` is valid only when final is `valid`. Human
diagnostic wording is not contractual.

Sequence cases include a `sequence` array whose members are strictly ordered by snapshot sequence and
carry the input ordinal, predecessor name or `null`, current name, byte length, and digest. Sequence
inputs are materialized after mutations so neither engine's interpretation of the current fixture
mutation DSL is authority at gate time. Both engines consume the same exact raw challenge and release-
evidence bytes rather than independently re-serializing them.

The frozen corpus includes every current authority, challenge, release-state, schema, governance,
transition, exact-byte, tag, hold, completion, and sequence case; all 16 overall-status transition
pairs; and every DC-35 repair-regression case. The complete corpus remains intact through cutover.
The oracle-freeze review receives the strict manifest schema, complete manifest, all materialized bytes,
a case/transition/repair coverage inventory, and the observed command
`python3 release/oracle/verify-manifest.py --format json`. That standard-library-only verifier owns only
manifest grammar, path containment, input identity, ordering, and coverage inventory; it does not
reimplement policy semantics. Rust later implements the same verifier contract and differential checks
its output before the narrow Python verifier is eligible for deletion. A generator may assist authoring,
but the committed reviewed bytes and digests are authority; the gate never regenerates expected inputs
from the mutation DSL.

### Python observation and reason strategy

A separately reviewed Python observation adapter may call the accepted validators to expose per-case
structural, semantic, and final values they genuinely compute. It must not assign policy reasons or
change validation branches. For the original corpus, a harness compares commit `12c137d` with the
adapter revision case by case and proves identical Boolean/final outcomes and identical top-level exit
status. Profile-hardening cases are explicitly identified as additions.

The differential gate compares Python and Rust only for stages and verdicts the Python adapter actually
exposes. It does **not** claim Python/Rust reason equivalence. Rust independently derives
`primary_reason`, which is compared with the reviewed manifest expectation and covered by Rust unit and
property tests. Any future reason-producing Python adapter requires its own design and behavior review.

## Differential migration gate

During migration, one command executes both engines against the frozen oracle and fails on any
difference in:

- case set or input digest;
- structural, semantic, or final verdict;
- each validation stage genuinely exposed by the Python observation adapter.

The same command separately fails when Rust's reason category differs from the manifest expectation.
Both engines independently execute the same public schema and profile. Human-readable diagnostic text
may differ.
A negative self-test deliberately introduces a disagreement and must demonstrate a nonzero differential
gate result. Independent Rust unit and property tests derive from DC-35 invariants rather than treating
Python output as the only specification. Before and after each policy command, capture and compare
`git status --porcelain=v1 --untracked-files=all` and the exact `Cargo.lock` digest while using
`--locked`. Ignored Cargo outputs are not claimed unchanged; stronger filesystem-clean evidence requires
an isolated `CARGO_TARGET_DIR` and is reported separately.

## Staged implementation and cutover

Implementation proceeds through separately reviewable stages:

1. Freeze and review the exact-byte oracle manifest and materialized vector set without changing the
   authoritative command. Any pre-oracle profile hardening and Python observation adapter are isolated,
   behavior-reviewed prerequisites to this freeze.
2. Add the Rust workspace package, mature schema validator, typed semantic checks, independent tests,
   metadata/package-graph gates, and dual-engine differential command.
3. Obtain architect acceptance of the Rust implementation and differential evidence while Python
   remains authoritative.
4. Switch the documented authoritative command to Rust in an isolated architect-reviewed cutover.
5. Remove the Python engine and its custom schema evaluator in a separate reviewed change only after
   cutover stability is demonstrated.
6. Compact or reorganize fixtures only in a later reviewed change with exact-byte and coverage-
   equivalence evidence.

Directory cleanup follows ownership and comprehension cost, not a target file count. `release/`
retains its concise explanation, public contracts, exact canonical raw vectors, and compact case
metadata. Executable Rust belongs under `tools/release-policy/`.

### Cutover, stability, and rollback authority

The current authoritative command locations are the executable `release/check-policy.py` and its
invocations in `release/README.md`, `docs/src/reference/release-compatibility.md`, and
`docs/src/contributing/development.md`. Implementation adds one machine-readable inventory containing
those exact paths and command strings. A stale-reference gate scans tracked text and fails on any
unregistered Python or Rust primary-command reference. Inventory entries classify `primary-executable`,
`live-invocation`, and `historical-or-explanatory` locations. Historical RFC/review prose is permitted
only through explicit path/section classification and never becomes invocation authority merely because
it contains a command string. Tests must prove that an unregistered contributor/CI invocation fails
while registered historical text does not produce a false positive.

The isolated cutover may change only that command inventory, authoritative command wrapper/references,
and contributor/CI documentation. It cannot change either engine, dependency selection, schema,
profile, manifest, expected stage/verdict/reason, or materialized vector. On one cutover commit, evidence
must run Python, Rust, differential comparison, deliberate-disagreement self-test, oracle verification,
Cargo/package/archive boundary checks, the Git-visible-state and lockfile comparison, and the full
applicable repository gates: format, workspace check, Clippy, test, build, mdBook, and diff check. An
architect cutover ruling is the only authority that makes Rust primary.

Before cutover acceptance, rehearse rollback in a disposable worktree or equivalent isolated checkout.
Restore only the command-switch diff, then observe the Python oracle, manifest verification, stale-
reference check, metadata/package boundary, and applicable repository gates pass. Record both commit
identities and prove the main worktree and signer authority are unchanged.

Stability means an architect-accepted rerun from a later commit, with no unresolved differential or
policy defect; elapsed time alone is insufficient. Python is retained through the first Rust-gated
0.19.0 release and its accepted post-release stability rerun. Deletion is forbidden before that point
and remains a separate architect-reviewed change. Fixture compaction occurs only after deletion review.

Until architect cutover acceptance, Python remains authoritative. If differential evidence or cutover
regresses, restore only the command-switch diff and continue using the committed Python oracle. Such a
rollback does not admit a signer, change signer authority, or authorize a release.

## Later formal-verification pilot

Verus is not part of DC-45 implementation or cutover. After the Rust core, differential migration, and
cutover are accepted and stable, a separate time-boxed design may evaluate a bounded pilot. That design
must define an effort and CI-cost budget, pinned toolchain, success criteria, maintenance owner, and
explicit deletion/abandonment path before proof code lands.

The pilot may address only pure policy obligations such as the transition relation and terminal
`superseded` state, strict attempt-prefix extension, monotonic observed-tag and governance fields,
active-hold exclusion of `complete`, the abstract 72-hour lift condition, and introduced-fingerprint/
proof-record cardinality. It may not claim proof over parsing, Git/OpenPGP commands, filesystems,
clocks, networks, registries, or hosted services. It is abandoned if it creates a second proof-only
policy implementation, weakens the stable Rust API, cannot pin reproducibly, or costs more than the
bounded obligations justify. Normal Rust and integration tests remain mandatory.

## Non-goals

- No DC-35 policy, governance, signer-authority, hold, transition, or evidence-semantics redesign.
- No signer bootstrap, fingerprint admission, private key, credential, release candidate, tag,
  package, archive, publication, SBOM, provenance, or production-readiness action.
- No product-crate dependency on release tooling and no publication of the tooling crate.
- No replacement of the public JSON Schema with generated Rust schema or Rust types.
- No immediate Python deletion, fixture reduction, broad `release/` rewrite, general task runner, or
  mandatory formal verification.
- No scope transfer from DC-41, DC-42, DC-43, or DC-44.

## Acceptance criteria

DC-45 is complete only when the reviewed Rust tool preserves the frozen exact-byte oracle, independently
executes the normative Draft 2020-12 schema, passes invariant and differential tests including the
deliberate-disagreement self-test, leaves the worktree unchanged, and satisfies the workspace/package-
graph boundary. Policy commands must preserve Git-visible state under `--untracked-files=all` and the
locked `Cargo.lock` digest; ignored build outputs are not part of that claim. The authoritative-command
cutover and rollback rehearsal must be accepted before the 0.19.0 release candidate. Python is retained
through the first Rust-gated 0.19.0 release and an accepted later-commit stability rerun. Python removal
and any fixture compaction require their own later evidence and reviews; neither is implied by accepting
the Rust implementation or cutover.

## Lifecycle completion boundary

DC-45's primary implementation may move to `done/` with the first accepted Rust-gated 0.19.0 release
even though Python is intentionally retained for rollback. In that transition, the RFC status and RFC
index must explicitly record Python deletion and later fixture compaction as separately reviewed
deferred work. Movement to `done/` must not claim that decommissioning, stability, deletion, or
compaction evidence passed unless each was actually observed and accepted.
