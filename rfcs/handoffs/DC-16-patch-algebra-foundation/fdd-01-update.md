# DC-16 FDD-01 Update - Patch Algebra Vocabulary and Conflict Witness Taxonomy

Status: Implemented and released with DC-16 / 0.9.0
Related RFC: `../../done/DC-16-PATCH-ALGEBRA-FOUNDATION.md`
Target FDD: FDD-01 Patch Algebra

## Purpose

DC-16 introduces the first durable vocabulary for M2+ patch algebra. It does not execute merges,
publish multi-parent blocks, add rollback refs, or freeze a persisted conflict-witness object. The
purpose of this FDD-01 update is to give later confluence and merge DCs a stable language for pair
classification and fail-closed conflict evidence.

## Required FDD-01 Body Updates

### Pair Classification Vocabulary

Patch algebra uses these terms:

- **Operation identity**: canonical operation bytes plus the operation's `op_seq` position inside a
  Patch.
- **Target node**: the non-zero `node_id` addressed by an operation.
- **Preimage**: the structured state facts an operation requires before application, including
  live/dead status, occupied paths before application, required-free paths, kind, path, mode, blob id,
  old text span, or old symlink target.
- **Postimage**: the structured state facts an operation establishes after application, including
  occupied paths after application, paths freed, paths newly occupied, kind/path/mode/blob changes, or
  tombstone facts.
- **Independent pair**: two operations whose application order can be swapped without changing the
  final lifecycle state and without changing either operation's identity bytes.
- **Ordered dependency**: two operations where both may be valid, but one must apply before the other.
- **Conflict**: two operations cannot both be applied to the same baseline under the current rules.
- **Unknown**: the pair is outside the proven algebra subset and must fail closed instead of being
  treated as independent.
- **Conflict witness**: structured diagnostic evidence naming the exact state fact that prevents safe
  commutation or joint application.

The governing rule is fail-closed: a pair is `Independent` only if the rules prove it independent.
Otherwise it is `OrderedDependency`, `Conflict`, or `Unknown`.

### Pair Classification Result

FDD-01 should describe the design shape:

```text
PairClass =
  Independent
  OrderedDependency { required_order, reason }
  Conflict { witness }
  Unknown { reason }
```

`required_order` is `left_before_right` or `right_before_left`.

This classification is diagnostic foundation. It does not authorize command behavior, reorder an
existing Patch, or publish merge results.

### Initial Algebra Subset

The first supported pair-classification subset is:

- `CreateFile`;
- file `DeleteNode`;
- `EditText`;
- `ReplaceBinary`;
- `ChangePerm`.

The following classify as `Unknown` unless a later FDD update widens the subset:

- `RenamePath`;
- `CreateSymlink`;
- symlink `DeleteNode` cases beyond existing replay validation;
- future precondition records;
- unknown operation kinds;
- malformed records that current replay would reject.

### Path Occupancy Authority

Path occupancy is a first-class structured preimage/postimage effect set:

```text
PathEffects {
  occupied_before,
  required_free,
  occupied_after,
  freed,
  newly_occupied,
}
```

- `CreateFile` requires its target path to be free and postimages that path as occupied.
- `required_free` participates in path-effect intersection checks; implementations must not infer
  independence by comparing postimages only.
- File `DeleteNode` requires the target node to be live at its recorded path and postimages that path
  as free.
- Cross-node pairs whose path-occupancy effects intersect are `OrderedDependency` or `Conflict`, never
  `Independent`.
- A delete-frees-path plus create-occupies-same-path pair is ordered-dependent when delete-before-create
  is valid, and conflict when neither order satisfies both preimages.

Different `node_id`s are therefore not sufficient proof of independence.

### Same-Node Text Boundary

Same-node `EditText` independence is not part of the first algebra subset. Same-node text pairs classify
as `OrderedDependency`, `Conflict`, or `Unknown`, never `Independent`, until a dedicated text-commutation
design provides perturbation vectors and transform rules.

Different-node `EditText` pairs may be independent only when the node-level and path-occupancy rules
prove independence.

### Conflict Witness Taxonomy

Conflict witnesses name the smallest authoritative fact that blocks safe classification. The initial
diagnostic witness fields are:

```text
ConflictWitness {
  kind,
  left_op_seq,
  right_op_seq,
  node_id,
  path,
  expected,
  actual,
  text_span,
}
```

Fields are optional where not applicable. Initial `kind` values:

- `same_path_create`;
- `node_id_reuse`;
- `live_state_mismatch`;
- `kind_mismatch`;
- `mode_mismatch`;
- `blob_mismatch`;
- `text_span_overlap`;
- `text_anchor_stale`;
- `delete_mutation_conflict`;
- `unsupported_operation`;
- `malformed_operation`;
- `unknown_relation`.

The implementation should define deterministic precedence when multiple witness kinds apply. Witnesses
are internal diagnostics in DC-16, stable for tests but not frozen as a public wire object.

Conflict witnesses in DC-16 are internal diagnostic values for tests and classifier debugging. They are
not persisted objects, not canonical identity bytes, not part of Patch, Block, RefState, or RefUpdate
schema, and not a user-facing conflict-resolution format.

### Independent-Pair Soundness Oracle

Every `Independent` classification must be covered by a test-level both-order replay oracle:

1. start from the same baseline lifecycle state;
2. replay left then right;
3. replay right then left;
4. assert structural equality of the resulting lifecycle states.

This oracle is test methodology, not a production confluence API. Production confluence checks remain a
later FDD/DC topic.

The oracle must replay the same decoded operation identity bytes in alternate application order. It
must not rewrite operation payloads, change canonical bytes, or renumber `op_seq` to make a swapped
order pass.

Structural lifecycle-state equality includes live node map, path occupancy, node kind, path, mode, blob
id or symlink target where applicable, and tombstone/seen-id facts needed by lifecycle rules. It
excludes non-authoritative caches and diagnostic ordering.

## Required Tests

- independent different-node content and mode/content pairs pass the both-order replay oracle;
- same-path create/create conflicts;
- delete-frees-path plus create-occupies-same-path is ordered-dependent or conflict, never independent;
- same-node `ChangePerm` plus `EditText` / `ReplaceBinary` commutes only when both preimages match the
  same baseline;
- create-then-mutate and mutate-then-delete ordered dependencies are distinguished from conflicts;
- same-node text pairs are not classified as independent in DC-16;
- unsupported rename, symlink, precondition, malformed, and unknown operation cases fail closed as
  `Unknown` or `Conflict`, never `Independent`;
- malformed operations that cannot decode into normal classifier inputs still produce `Unknown` /
  `Conflict` diagnostics through the surrounding classifier API instead of being skipped;
- witness-kind precedence is deterministic.
