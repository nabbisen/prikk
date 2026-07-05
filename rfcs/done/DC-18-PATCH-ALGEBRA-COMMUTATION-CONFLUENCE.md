# RFC (done) - DC-18 Patch Algebra Commutation and Confluence Contract

**Status.** Implemented and released as v0.11.0.
**Target release.** v0.11.0.
**Tracks.** Internal production contract for patch commutation and sequence confluence after the DC-16
classifier foundation and DC-17 evidence boundary.
**Touches.** Patch algebra analysis APIs, replay oracle helpers, lifecycle-state equality, conflict
diagnostic taxonomy, and the FDD-01 patch-algebra vocabulary.
**Companion FDD updates.** `../handoffs/DC-18-patch-algebra-commutation-confluence/fdd-01-update.md`.

## Context

DC-16 introduced Prikk's first patch-algebra vocabulary and pair classifier. DC-17 then made the
classifier's evidence boundary explicit: required sealed evidence failures are integrity errors, while
unsupported algebra remains ordinary `Unknown`.

The next gap is the production meaning of commutation and confluence. A pair labeled `Independent` is
useful only if later code can trust what that means. Before Prikk can design public merge evidence,
conflict UX, branch merge behavior, or rollback refs, it needs an internal contract that answers:

- when two supported operations may be swapped without changing the final authoritative state;
- when two supported candidate sequences from the same sealed baseline are proven to converge;
- when evidence, unsupported operation kinds, ordered dependencies, or text perturbation uncertainty
  must fail closed.

DC-18 defines that contract. It remains internal and read-only. It does not publish merges or create a
public conflict object.

## Design Goals

1. Define the production meaning of operation-pair commutation for the DC-16/DC-17 supported subset.
2. Require replay-oracle proof for every `Independent` pair used as commutation evidence.
3. Define a small sequence-level confluence check for two candidate operation sequences from one sealed
   baseline.
4. Preserve the DC-17 evidence boundary: integrity failures are not `Unknown`, and optional unsealed
   candidate evidence is explicit.
5. Keep same-node text transforms, persisted merge evidence, CLI merge behavior, and multi-parent
   publication out of scope.
6. Produce deterministic internal diagnostics that can feed a later public conflict/merge evidence DC
   without becoming public schema now.

## Non-goals

DC-18 does not add:

- CLI merge, branch merge, or branch switching commands;
- multi-parent Block publication or merge commits;
- persisted conflict-witness, confluence-proof, or merge-evidence objects;
- Patch, Block, RefState, RefUpdate, or object-schema changes;
- rollback refs or rollback authorization;
- semantic/language-aware merge;
- same-node text operational transforms;
- rename, symlink, tag, remote, key-lifecycle, audit/plugin, or sync behavior.

## Proposed Design

### Vocabulary

DC-18 should standardize these terms:

| Term | Meaning |
|---|---|
| Common sealed baseline | The validated Block/lifecycle state against which all candidate operations are analyzed. |
| Candidate operation | A decoded operation under analysis. It may come from a sealed candidate Patch or an unsealed candidate Patch, with DC-17 evidence scope attached. |
| Candidate sequence | An ordered list of candidate operations whose internal order is preserved by analysis. |
| Flat candidate sequence | A candidate sequence whose operations are each individually analyzable and replay-valid from the common sealed baseline under the DC-18 pair oracle. A sequence with internal ordered dependencies is not flat in DC-18. |
| Pair commutation | A proof that applying `left` then `right` and `right` then `left` from the same baseline both succeeds and yields the same authoritative state, using the same operation identity bytes. |
| Sequence confluence | A proof that two candidate sequences from the same sealed baseline can be composed in either sequence order and yield the same authoritative state. |
| Authoritative state equality | Equality of replay-derived lifecycle facts, path occupancy, node kind/path/mode/content ids or symlink targets, tombstone/seen-id facts, and provenance needed by lifecycle validation. It excludes caches and prose diagnostics. |
| Diagnostic proof | Internal explanation of why commutation/confluence was proven or rejected. It is not a persisted or signed object in DC-18. |

The rule is intentionally strict: a classifier result is not enough by itself. `Independent` is usable
as commutation evidence only when backed by a replay-both-orders oracle.

### Pair Commutation Contract

Add an internal pair-commutation analysis shape:

```text
Result<CommutationResult, EvidenceError>

where CommutationResult =
  Commutes { proof }
  DoesNotCommute { pair_class }
  Unknown { reason }
```

The exact Rust names are implementation details. The required behavior is:

- `Commutes` is returned only when the pair classifier returns `Independent` and the replay oracle proves
  both application orders produce the same authoritative state.
- `DoesNotCommute` is returned for `Conflict` and `OrderedDependency`.
- `Unknown` is returned for unsupported/deferred algebra cases, including same-node text pairs without
  an approved transform rule.
- required sealed evidence failures remain an outer evidence/integrity error, not `Unknown`.
- no operation bytes, `op_seq`, node ids, paths, anchors, or payload fields are rewritten to make a
  commutation proof pass.

`OrderedDependency` means both operations may be valid in one order. It does not mean the pair commutes.
An ordered pair may be useful to a later merge planner, but it is not DC-18 confluence evidence.

### Pair Replay Oracle

The pair replay oracle must:

1. start from the same common sealed baseline lifecycle state;
2. apply `left` then `right` using each operation's original decoded identity;
3. apply `right` then `left` using each operation's original decoded identity;
4. reject either order if replay validation fails;
5. compare authoritative state equality after both orders;
6. return deterministic diagnostics for the first failed proof condition.

The oracle is an internal analysis primitive. It must not mutate refs, active WAL state, object storage,
or the worktree.

The oracle may use DC-17 store-backed evidence for blob/text facts. It must not infer content from the
filesystem or from caller-supplied summaries.

### Sequence Confluence Contract

Add an internal sequence-confluence analysis shape:

```text
Result<ConfluenceResult, EvidenceError>

where ConfluenceResult =
  Confluent { proof }
  NotConfluent { witness }
  Unknown { reason }
```

The check covers two flat candidate sequences, `A` and `B`, from the same common sealed baseline.

`Confluent` requires all of the following:

1. both sequences are individually valid from the common baseline;
2. every operation in both sequences is individually analyzable and replay-valid from the common
   baseline, not only after an earlier operation in the same sequence;
3. every operation is inside the DC-18 supported algebra subset;
4. every required evidence read is `Known`;
5. every cross-sequence pair that must swap between `A+B` and `B+A` has a proven pair-commutation
   result;
6. replaying `A` then `B` succeeds;
7. replaying `B` then `A` succeeds;
8. the final authoritative states are equal.

If a sequence contains an internal ordered dependency, such as `CreateFile -> ChangePerm` where the
second operation applies only after the first, DC-18 does not attempt contextual adjacent-swap proof.
That sequence is `Unknown { sequence_internal_dependency_deferred }` for confluence analysis, unless a
later DC defines contextual swap rules.

If a required cross-sequence pair is a concrete `OrderedDependency` or `Conflict`, the result is
`NotConfluent`, not `Unknown`. `Unknown` is reserved for unsupported, deferred, or unproven algebra. A
later merge-planning DC may reason about ordered integration, but DC-18 should not treat ordered
dependency as commutation.

### Scope of Candidate Sequences

DC-18 should keep the first implementation narrow:

- exactly two candidate sequences;
- one explicit common sealed baseline;
- flat candidate sequences only: every operation must be analyzable from the common sealed baseline;
- internal sequence dependencies are deferred, not treated as confluence evidence;
- no nested merge bases;
- no multi-parent Blocks;
- no operation transform/rebase step;
- no conflict materialization;
- no durable proof object.

The implementation may expose helpers that operate on decoded operations and a resolver-bound baseline.
It should not add a CLI command until public merge/conflict evidence is designed.

### Supported Operation Subset

DC-18 inherits the DC-16/DC-17 supported subset:

- `CreateFile`;
- file `DeleteNode`;
- `EditText`;
- `ReplaceBinary`;
- `ChangePerm`.

The following remain `Unknown` unless a later DC widens the subset:

- `RenamePath`;
- `CreateSymlink`;
- symlink `DeleteNode` beyond existing replay validation;
- future precondition records;
- unknown operation kinds;

Malformed operation handling is provenance-sensitive:

- well-formed but unsupported operation kinds are `Unknown { unsupported_operation }`;
- malformed operation payloads in sealed baseline or sealed candidate evidence are outer
  `EvidenceError` / repository-integrity failures;
- malformed or incomplete unsealed candidate input is `Unknown` or invalid candidate input, but never
  `Commutes` or `Confluent`.

### Same-Node Text Boundary

Same-node `EditText` commutation remains out of scope. Same-node text pairs must not return
`Commutes`, even when their spans appear disjoint, unless a later DC defines transform rules and
perturbation vectors.

Different-node `EditText` pairs may commute only when:

- the pair classifier proves independence;
- baseline text evidence is available under the required DC-17 scope;
- both replay orders localize and splice successfully;
- the final authoritative lifecycle/content state is equal.

This avoids accidental claims of operational transform, CRDT behavior, or semantic merge.

### Authoritative Equality

Authoritative state equality must compare replay-derived state, not debug formatting. At minimum it
includes:

- live node ids and tombstoned/seen ids;
- live path occupancy and path-to-node mapping;
- node kind, path, mode, blob id, and symlink target where applicable;
- tombstone facts needed by lifecycle reintroduction rules;
- lifecycle provenance needed to prove the state belongs to the selected baseline/horizon.

It excludes:

- cache insertion order;
- resolver memoization state;
- diagnostic strings;
- source-file system metadata outside the canonical operation/lifecycle model.

Blob identity equality is sufficient for content equality only because object storage already verifies
object identity on read. If future code compares raw bytes, it must do so after DC-17 object validation.

### Diagnostic Proof Policy

DC-18 may add internal diagnostic proof/witness values, but they must not become public schema.

Allowed diagnostic facts:

- baseline id / horizon id;
- left/right sequence labels;
- operation positions and `op_seq`;
- pair class and reason;
- oracle failure phase;
- witness kind from the existing internal taxonomy;
- evidence error identity from the outer error channel.

Disallowed diagnostic facts:

- raw text bytes;
- arbitrary prose expected/actual values;
- persisted canonical proof bytes;
- user-facing conflict resolution instructions.

When multiple proof failures apply, diagnostics must be deterministic. The recommended precedence is:

1. evidence/integrity error;
2. malformed or unsupported operation;
3. individual sequence replay failure;
4. cross-sequence non-commuting pair;
5. composed-order replay failure;
6. final authoritative-state inequality.

### Fail-Closed Mapping

The result surface must distinguish:

| Condition | Behavior |
|---|---|
| Required sealed evidence missing/malformed/unreadable | outer evidence/integrity error |
| Optional unsealed candidate evidence missing | `Unknown { missing_candidate_evidence }` |
| Unsupported operation kind | `Unknown { unsupported_operation }` |
| Malformed operation in sealed baseline or sealed candidate evidence | outer evidence/integrity error |
| Malformed or incomplete unsealed candidate input | `Unknown` or invalid candidate input; never `Commutes` / `Confluent` |
| Same-node text pair without transform rule | `Unknown { same_node_text_commutation_deferred }` |
| Concrete pair conflict | `DoesNotCommute` / `NotConfluent` with witness |
| Concrete ordered dependency across swapped sequences | `NotConfluent { ordered_dependency }` or `DoesNotCommute`; not `Unknown` or `Confluent` |
| Candidate sequence with internal ordered dependency | `Unknown { sequence_internal_dependency_deferred }` |
| Both orders replay and final authoritative states match | `Commutes` / `Confluent` |

No missing or unsupported case may be treated as `Commutes` or `Confluent`.

## Implementation Guidance

A reasonable v0.11.0 implementation slice is:

1. introduce internal pair-commutation and sequence-confluence result types;
2. add replay-oracle helpers for pair and two-sequence analysis;
3. wire the helpers to the existing classifier and DC-17 evidence resolver;
4. keep the module private/internal and test-compiled if no production caller exists yet;
5. add focused tests for pair commutation, ordered dependency rejection, sequence confluence, evidence
   errors, and same-node text deferral.

The implementation should not add broad abstractions for future merge planning until a caller exists.

## Required Tests

DC-18 implementation should pin:

- every `Commutes` pair has a replay-both-orders oracle test;
- different-node mode/content and content/content pairs commute only when preimages match;
- same-path create/create and create-to-occupied-path never commute;
- delete-frees-path plus create-occupies-path is ordered or conflicting, not commutative;
- `CreateFile -> ChangePerm` remains ordered, not commutative;
- same-node `ChangePerm` plus content mutation commutes only with matching baseline preimages;
- same-node text pairs remain non-commuting/unknown;
- two candidate sequences with only proven cross-pair commutations are `Confluent`;
- a cross-sequence ordered dependency maps deterministically to `NotConfluent { ordered_dependency }`;
- a sequence with internal ordered dependency maps to
  `Unknown { sequence_internal_dependency_deferred }`;
- unsupported rename/symlink/precondition cases are `Unknown`;
- malformed sealed candidate operations surface through the outer evidence-error channel;
- malformed or incomplete unsealed candidate input remains `Unknown` or invalid candidate input, never
  `Commutes` or `Confluent`;
- required sealed evidence failures surface through the outer evidence-error channel;
- optional unsealed candidate evidence remains `Unknown`;
- final authoritative-state inequality is detected even if pair diagnostics are incomplete.

## Acceptance Criteria

The DC-18 release is acceptable when:

- the RFC/FDD vocabulary for commutation and confluence is explicit;
- all `Commutes` and `Confluent` outcomes require replay-oracle proof;
- required evidence errors cannot be collapsed into `Unknown`;
- sequence confluence is limited to two sequences from one sealed baseline;
- no CLI, object schema, merge publication, or persisted proof/witness object is added;
- tests cover the required fail-closed boundaries.

## Open Questions for Architect Review

1. Is two-sequence confluence the right v0.11.0 slice, or should DC-18 stop at pair commutation only?
2. Should the flat-sequence restriction be tightened further to pair commutation only?
3. Should a diagnostic proof type be introduced now, or should tests inspect only result enums and
   witnesses?
4. Is authoritative lifecycle-state equality sufficient, or should the oracle also re-read final blob
   bytes for text/binary content?
5. Should `patch_algebra` remain test-compiled until a production caller exists, or should DC-18 add a
   crate-internal production analysis entry point with no CLI exposure?

## Design Review v1 Revisions

Architect review v1 accepted the direction but required two clarifications before implementation:

- malformed operation handling is now scoped by evidence provenance, so sealed baseline/candidate
  malformed payloads are integrity errors rather than ordinary `Unknown`;
- two-sequence confluence is now restricted to flat candidate sequences whose operations are each
  analyzable from the common sealed baseline. Contextual adjacent-swap proof for internally dependent
  sequences is deferred.
