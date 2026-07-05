# DC-18 FDD-01 Update - Commutation and Confluence Contract

Status: Released with DC-18 / v0.11.0
Related RFC: `../../done/DC-18-PATCH-ALGEBRA-COMMUTATION-CONFLUENCE.md`
Target FDD: FDD-01 Patch Algebra

## Purpose

DC-18 defines the internal production contract for pair commutation and small sequence confluence after
the DC-16 classifier and DC-17 evidence boundary. It does not execute merges, publish multi-parent
Blocks, add CLI behavior, or freeze a persisted proof/witness object.

The purpose of this FDD-01 update is to make future merge/conflict designs depend on a precise algebra
contract instead of a loose "no known conflict" interpretation.

## Required FDD-01 Body Updates

### Commutation Vocabulary

FDD-01 should define:

- **Common sealed baseline**: the validated Block/lifecycle state against which candidate operations are
  analyzed.
- **Candidate sequence**: an ordered list of candidate operations whose internal order is preserved.
- **Flat candidate sequence**: a sequence whose operations are each individually analyzable and
  replay-valid from the common sealed baseline under the DC-18 pair oracle. A sequence with internal
  ordered dependencies is deferred.
- **Pair commutation**: proof that `left; right` and `right; left` both replay from the same baseline
  and yield the same authoritative state using the same operation identity bytes.
- **Sequence confluence**: proof that two candidate sequences from one sealed baseline can be composed
  as `A+B` and `B+A` and yield the same authoritative state.
- **Authoritative state equality**: equality of replay-derived lifecycle facts, not equality of debug
  strings, cache state, or filesystem summaries.

### Pair Commutation Result

FDD-01 should describe the design shape:

```text
Result<CommutationResult, EvidenceError>

where CommutationResult =
  Commutes { proof }
  DoesNotCommute { pair_class }
  Unknown { reason }
```

`Commutes` requires both:

1. pair classification returns `Independent`;
2. a replay-both-orders oracle proves equal final authoritative state.

`OrderedDependency` and `Conflict` do not commute. `Unknown` remains fail-closed for unsupported or
deferred algebra.

Required sealed evidence failures remain outer evidence/integrity errors, following DC-17.

### Pair Replay Oracle

Every `Commutes` result must be backed by an oracle that:

1. starts from the same common sealed baseline lifecycle state;
2. replays left then right;
3. replays right then left;
4. uses original operation identity bytes without rewriting or renumbering;
5. rejects either failed replay order;
6. compares final authoritative lifecycle state.

The oracle is an internal analysis primitive. It must not mutate refs, active WALs, objects, or the
worktree.

### Sequence Confluence Result

FDD-01 should describe the design shape:

```text
Result<ConfluenceResult, EvidenceError>

where ConfluenceResult =
  Confluent { proof }
  NotConfluent { witness }
  Unknown { reason }
```

For DC-18, `Confluent` is limited to two flat candidate sequences from one common sealed baseline.

`Confluent` requires:

- both candidate sequences replay individually from the baseline;
- every operation is individually analyzable and replay-valid from the common baseline, not only after
  an earlier operation in the same sequence;
- every operation is inside the supported algebra subset;
- every required evidence read is `Known`;
- every cross-sequence pair that swaps between `A+B` and `B+A` has a proven `Commutes` result;
- both composed orders replay successfully;
- final authoritative states are equal.

Cross-sequence `OrderedDependency` is not confluence evidence. Concrete cross-sequence
`OrderedDependency` or `Conflict` maps to `NotConfluent`. Unsupported/deferred relations remain
`Unknown`, and evidence/integrity failures remain outer `EvidenceError`.

A candidate sequence with internal ordered dependencies, such as `CreateFile -> ChangePerm`, is
`Unknown { sequence_internal_dependency_deferred }` for DC-18 confluence analysis. Contextual
adjacent-swap proof is deferred to a later DC.

### Authoritative State Equality

The equality relation includes:

- live node ids and tombstoned/seen ids;
- live path occupancy and path-to-node mapping;
- node kind, path, mode, blob id, and symlink target where applicable;
- tombstone facts required by lifecycle reintroduction rules;
- lifecycle provenance required to bind the state to the selected baseline/horizon.

It excludes:

- cache insertion order;
- resolver memoization state;
- diagnostic strings;
- filesystem metadata outside canonical operation/lifecycle state.

Blob id equality is enough for content equality only after normal object identity validation.

### Supported Subset

DC-18 inherits the DC-16/DC-17 subset:

- `CreateFile`;
- file `DeleteNode`;
- `EditText`;
- `ReplaceBinary`;
- `ChangePerm`.

The following remain `Unknown`:

- `RenamePath`;
- `CreateSymlink`;
- symlink `DeleteNode` beyond existing replay validation;
- future precondition records;
- unknown operation kinds;

Malformed operation handling is scoped by evidence provenance:

- well-formed but unsupported operation kinds are `Unknown { unsupported_operation }`;
- malformed operation payloads in sealed baseline or sealed candidate evidence are outer
  `EvidenceError` / repository-integrity failures;
- malformed or incomplete unsealed candidate input is `Unknown` or invalid candidate input, but never
  `Commutes` or `Confluent`.

Same-node `EditText` commutation remains deferred. Same-node text pairs must not produce `Commutes`
until a later text-transform DC defines perturbation rules and vectors.

### Diagnostic Proof Policy

Diagnostic proof values remain internal. They may include baseline id, sequence labels, operation
positions, pair class, oracle failure phase, witness kind, and evidence-error identity. They must not
include raw text bytes, arbitrary prose expected/actual fields, persisted canonical proof bytes, or
user-facing conflict-resolution instructions.

When multiple proof failures apply, diagnostics should use this deterministic precedence:

1. evidence/integrity error;
2. malformed or unsupported operation;
3. individual sequence replay failure;
4. cross-sequence non-commuting pair;
5. composed-order replay failure;
6. final authoritative-state inequality.

## Required Tests

- every `Commutes` result has a replay-both-orders oracle assertion;
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
- malformed sealed candidate operations use the outer evidence-error channel;
- malformed or incomplete unsealed candidate input remains `Unknown` or invalid candidate input, never
  `Commutes` or `Confluent`;
- required sealed evidence failures use the outer evidence-error channel;
- optional unsealed candidate evidence remains `Unknown`;
- final authoritative-state inequality is detected.
