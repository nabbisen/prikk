# RFC (done) - DC-21 Merge Conflict Evidence Contract

**Status.** Released in v0.14.0.
**Target release.** v0.14.0.
**Tracks.** First reviewable/user-facing evidence contract for M2+ merge/conflict analysis after DC-16
through DC-18 patch algebra and DC-20 replay-boundary stabilization.
**Touches.** Patch-algebra evidence vocabulary, merge/conflict diagnostic surfaces, read-only analysis
APIs, review-package expectations, and future FDD-01 merge/conflict wording.

## Context

DC-16 introduced Prikk's internal patch-algebra classifier: `Independent`, `OrderedDependency`,
`Conflict`, and `Unknown`. DC-17 then separated required evidence failures from ordinary unsupported
algebra. DC-18 made commutation and flat two-sequence confluence meaningful by requiring replay-backed
proof, not only classifier labels. DC-20 stabilized the `prikk-replay` boundary and kept resolver,
repository, worktree, cache, and patch-algebra ownership in `prikk-store`.

The next gap is not merge execution. The next gap is the evidence contract a future merge surface can
show to users or callers. Today the implementation has internal diagnostics and tests, but no approved
reviewable/user-facing vocabulary that says:

- what kind of conflict was detected;
- what evidence was used;
- whether the result is a real conflict, an ordered-dependency case, an unsupported/deferred case, or
  an evidence/integrity failure;
- which facts are safe to expose without freezing object schema or committing to a conflict-witness
  object format.

DC-21 defines that contract. It should remain read-only and evidence-focused. It must not execute
merges, publish merge commits, create durable proof objects, or move patch algebra into a new crate.

## Design Goals

1. Define a reviewable/user-facing merge/conflict evidence vocabulary backed by the existing internal
   DC-16/DC-18 algebra.
2. Preserve the DC-17 distinction between required evidence failures and ordinary `Unknown` algebra.
3. Define what facts may appear in a reviewable conflict/merge evidence report without becoming
   persisted object schema.
4. Define deterministic diagnostic precedence for user-facing evidence summaries.
5. Keep the first production surface read-only and non-mutating.
6. Keep merge execution, branch switching, multi-parent publication, conflict materialization, and
   user conflict resolution out of scope.
7. Keep `prikk-store` responsible for store-backed resolver/evidence construction.
8. Keep `prikk-replay` workspace-internal and avoid patch-algebra crate extraction in this DC.

## Non-goals

DC-21 does not add:

- merge execution;
- `prikk merge`, branch merge, branch switching, branch copy/fork, or merge-base commands;
- multi-parent Block publication or merge commits;
- persisted conflict-witness, proof, merge-evidence, or merge-plan objects;
- Patch, Block, RefState, RefUpdate, trust, WAL, or repository-layout schema changes;
- worktree conflict files, conflict markers, or checkout conflict materialization;
- same-node text operational transforms;
- semantic/language-aware merge;
- rollback refs or rollback authorization;
- extraction of `text_span`, `patch_algebra`, resolver construction, lifecycle-cache persistence, or
  worktree behavior into `prikk-replay` or another crate;
- public stability for `prikk-replay`.

## Proposed Design

### Evidence Vocabulary

DC-21 should standardize these reviewable/user-facing evidence categories:

| Category | Meaning |
|---|---|
| `Confluent` | Two candidate sequences are proven to converge under the DC-18 flat confluence contract. |
| `Conflict` | A concrete conflict was found for a supported pair or sequence relation. |
| `OrderedDependency` | The relation may be valid in one order, but it is not commutative evidence. |
| `Unsupported` | The operation kind or relation is intentionally outside the supported algebra subset. |
| `Deferred` | The relation needs a later design, such as same-node text transforms or contextual adjacent-swap proof. |
| `EvidenceFailure` | Required sealed evidence was missing, malformed, unreadable, wrong-type, or otherwise invalid. |
| `InvalidCandidate` | Unsealed candidate input is malformed or incomplete before it can be analyzed. |

`Unknown` remains useful internally, but a reviewable/user-facing evidence report must not expose a
single ambiguous `Unknown` bucket. The primary outcome must classify the result as one of the
categories above, or as `NotConfluent` for replay/final-state mismatch after supported analysis.
Required sealed evidence failures remain outside ordinary unsupported/deferred analysis and must
surface as `EvidenceFailure`.

### Evidence Report Shape

The first implementation should add a read-only report shape. Exact Rust names are implementation
details, but the design shape is:

```text
MergeEvidenceReport {
  baseline_block_id,
  replay_horizon,
  left_sequence,
  right_sequence,
  outcome,
  items,
}

MergeEvidenceOutcome =
  Confluent
  Conflict
  OrderedDependency
  Unsupported
  Deferred
  NotConfluent
  EvidenceFailure
  InvalidCandidate
```

`baseline_block_id` is required. The selected baseline is part of the meaning of every conflict,
ordered-dependency, deferred, unsupported, and confluence result. If the implementation already has an
explicit replay horizon available, it should include that horizon as internal evidence; otherwise the
RFC accepts keeping the replay horizon resolver-internal for this slice, but `baseline_block_id` must
not be deferred.

`NotConfluent` is reserved for replay or final-state mismatch after otherwise supported analysis. It
must not be used as a generic bucket for conflict, ordered dependency, unsupported, deferred, evidence
failure, or invalid-candidate cases.

`items` are deterministic evidence entries. Each entry may include:

- sequence side (`left`, `right`, or `cross`);
- operation index and, when available, `op_seq`;
- operation kind;
- node id;
- repository path;
- relation category;
- evidence scope (`sealed-baseline`, `sealed-candidate`, or `unsealed-candidate`);
- proof phase, such as classification, replay-both-orders, flatness, composed replay, or final-state
  comparison;
- stable reason code.

The report must not include raw text bytes, full blob bytes, arbitrary debug strings, or filesystem
paths outside repository-relative `RepoPath` values.

The first implementation remains internal or workspace-visible. DC-21 defines vocabulary intended for
review, tests, and future UI preparation; it does not create an external crate API stability promise.
Future CLI display should use a separate view model unless a later DC explicitly freezes the report
type as the display contract.

### Reason Codes

Reason codes should be stable enough for CLI/API display tests, but not persisted object schema.
Initial reason codes should cover:

- `proven_confluent`;
- `pair_conflict`;
- `ordered_dependency`;
- `unsupported_operation`;
- `same_node_text_transform_deferred`;
- `sequence_internal_dependency_deferred`;
- `flatness_required`;
- `pair_replay_failed`;
- `composed_replay_failed`;
- `final_state_mismatch`;
- `missing_required_evidence`;
- `malformed_required_evidence`;
- `wrong_type_required_evidence`;
- `unreadable_required_evidence`;
- `invalid_unsealed_candidate`;
- `insufficient_unsealed_candidate_evidence`.

Reason-code names may change during design and implementation review, but the taxonomy must preserve
the distinctions above. After v0.14.0 release, accepted reason codes are release-stable diagnostic
vocabulary for tests and future display unless superseded by a later DC. They are not persisted object
schema.

### Evidence Scope Mapping

The report category must preserve the DC-17/DC-18 evidence-scope distinction:

| Evidence scope / condition | Public report category |
|---|---|
| `SealedBaselineRequired` failure | `EvidenceFailure` |
| `SealedCandidateRequired` failure | `EvidenceFailure` |
| malformed unsealed candidate input before analysis | `InvalidCandidate` |
| insufficient optional unsealed candidate evidence | `InvalidCandidate`, unless the reason is genuinely algebraic; then `Unsupported` or `Deferred` |

Required sealed evidence failures include missing, malformed, unreadable, wrong-type, or identity
mismatching evidence. They must not be reported as `Unsupported`, `Deferred`, `Conflict`,
`OrderedDependency`, `NotConfluent`, or `Confluent`.

### Diagnostic Precedence

When multiple issues apply, a report must choose deterministic primary outcome precedence:

1. `EvidenceFailure` for required sealed evidence failures.
2. `InvalidCandidate` for malformed/incomplete unsealed candidate input.
3. `Unsupported` for operation kinds outside the supported subset.
4. `Deferred` for supported-domain cases intentionally not designed yet.
5. `Conflict` for concrete conflicts.
6. `OrderedDependency` for non-commuting ordered relations.
7. `NotConfluent` for replay/final-state mismatch after otherwise supported analysis.
8. `Confluent` only when all DC-18 requirements are satisfied.

The report may include secondary entries after the primary outcome, but the primary outcome must not
depend on map iteration, filesystem order, debug formatting, or nondeterministic resolver behavior.

### Supported Analysis Scope

DC-21 inherits the DC-18 supported subset:

- `CreateFile`;
- file `DeleteNode`;
- `EditText`;
- `ReplaceBinary`;
- `ChangePerm`.

The following remain unsupported or deferred unless a later DC widens the subset:

- `RenamePath`;
- `CreateSymlink`;
- symlink deletion beyond existing replay validation;
- same-node text transform/operational transform;
- contextual adjacent-swap proof for internally dependent sequences;
- semantic/language-aware merge;
- future operation preconditions.

### Public Surface

The first implementation should expose a library-level read-only analysis surface, not a CLI merge
command.

Acceptable surfaces:

- crate-internal or workspace-visible report types used by tests and future commands;
- a read-only helper that converts existing internal commutation/confluence diagnostics into the
  evidence report shape;
- focused display formatting helpers if needed for tests, as long as they do not become CLI commands.

Deferred surfaces:

- `prikk merge`;
- `prikk merge --dry-run`;
- persisted witness/proof objects;
- worktree conflict files;
- branch publication of merge results;
- public stable API for external crates.

### Resolver and Crate Boundary

`prikk-store` remains responsible for:

- resolving sealed baselines;
- reading objects and validating object identity/type;
- constructing store-backed evidence/resolvers;
- preserving DC-17 evidence scope distinctions;
- mapping repository integrity failures into evidence failures.

`prikk-replay` remains responsible only for replay/lifecycle semantic substrate already assigned to it.
DC-21 must not move resolver construction, patch algebra, text-span code, lifecycle-cache persistence,
or worktree behavior into `prikk-replay`.

Patch algebra should remain internal in `prikk-store` for this DC. A later crate-boundary RFC may
revisit extraction after a production caller proves the stable API shape.

### Security and Privacy

Evidence reports must avoid leaking more content than needed:

- no raw blob bytes;
- no raw text spans;
- no absolute host filesystem paths;
- no signer secrets or key material;
- no arbitrary debug dumps of object payloads;
- no trust-policy private state beyond stable issue/reason codes.

Blob ids, node ids, repository-relative paths, operation kinds, sequence labels, and evidence scope are
acceptable.

### FDD-01 Update

DC-21 should include an FDD-01 handoff that adds:

- reviewable/user-facing evidence categories;
- reason-code taxonomy;
- primary outcome precedence;
- distinction between `EvidenceFailure`, `Unsupported`, `Deferred`, `Conflict`, and
  `OrderedDependency`;
- the rule that evidence reports are not persisted proof schema;
- the rule that merge execution remains a later design.

## Migration Plan

### Phase 1 - Evidence Vocabulary and Types

- Add internal/workspace-visible evidence report types.
- Add reason-code taxonomy.
- Add deterministic ordering rules for evidence entries.
- Keep all new surfaces non-mutating.

### Phase 2 - Adapter from Existing Algebra

- Adapt existing DC-18 confluence/commutation outputs into evidence report entries.
- Preserve outer evidence errors and scope.
- Do not rewrite operation identity, `op_seq`, node ids, paths, anchors, or payload fields.

### Phase 3 - Focused Tests

- Add tests for each category and primary precedence rule.
- Add tests that required sealed evidence failures do not collapse into `Unsupported` or `Deferred`.
- Add tests that same-node text remains `Deferred`, not `Confluent`.
- Add tests that ordered dependencies are not reported as conflicts or confluence evidence.
- Add tests that reports do not include raw text/blob bytes.
- If `Debug`, `Display`, or formatting helpers are added for reports, add tests that formatted output
  contains no raw text spans, raw blob bytes, absolute host paths, or arbitrary object debug dumps.

### Phase 4 - Documentation and FDD Handoff

- Add FDD-01 handoff wording.
- Update implementation status and roadmap.
- Keep release notes explicit about non-goals.

## Release and Compatibility Rules

DC-21 must not change:

- object ids or canonical payload bytes;
- patch identity or replay order;
- lifecycle semantics;
- text-span identity or inverse behavior;
- repository layout;
- ref/WAL/trust behavior;
- verification or doctor semantics;
- worktree behavior;
- CLI commands or output, unless a later review explicitly approves a read-only display command.

## Test and Review Requirements

Implementation review should include:

- `cargo test --workspace`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo fmt --check`;
- `git diff --check`;
- focused patch-algebra evidence-report tests;
- evidence-boundary tests for sealed and unsealed candidate scopes;
- determinism tests for report entry ordering and primary outcome precedence;
- line-count and test-module placement audit;
- audit that no raw blob/text bytes or absolute host paths appear in evidence reports;
- display/formatting privacy tests if `Debug`, `Display`, or formatting helpers are added;
- explicit statement that no CLI/schema/repository-layout/ref/WAL/trust/worktree behavior changed.

## Open Questions

1. Should the first report API be `pub(crate)` inside `prikk-store`, workspace-visible, or public within
   the crate but documented as unstable? Initial answer: use crate-internal or workspace-visible only.
2. Should future CLI display consume the same report type directly, or should there be a separate view
   model to avoid freezing internal fields? Initial answer: use a later view model for CLI display.
3. How much operation identity should be exposed: `op_seq` only, operation index only, or both?
   Initial answer: expose both sequence-local operation index and `op_seq` when present; do not expose
   full operation payloads.
4. Should evidence reports include baseline/block ids immediately, or leave that to a later branch
   merge planning DC? Answer: include `baseline_block_id` immediately.
5. Which reason-code names should be considered stable for tests versus provisional for review?
   Initial answer: reason-code names may change before implementation acceptance; after v0.14.0 release
   they are release-stable diagnostic vocabulary unless superseded by a later DC.

## Acceptance Criteria

DC-21 design is accepted when review agrees on:

- evidence categories and primary outcome precedence;
- report shape, explicit primary outcomes, baseline identity, and allowed/disallowed facts;
- reason-code taxonomy;
- evidence-scope-to-category mapping;
- resolver and crate-boundary ownership;
- explicit deferral of merge execution, durable proof objects, CLI merge, schema changes, and conflict
  materialization;
- FDD-01 handoff requirements;
- implementation test and review gates.
