# RFC (done) - DC-16 Patch Algebra Foundation

**Status.** Implemented and released as 0.9.0.
**Target release.** v0.9.0.
**Tracks.** First M2+ patch-algebra design slice: operation dependency classification, conservative
commutation boundaries, and internal conflict-witness taxonomy.
**Touches.** Patch replay/inverse domain model, lifecycle-cache terminology, optional non-mutating
analysis APIs, docs/status, and the FDD-01 patch-algebra vocabulary.
**Companion FDD updates.** `../handoffs/DC-16-patch-algebra-foundation/fdd-01-update.md`.

## Context

DC-12 made worktree text edits deterministic arbitrary spans. DC-14 made direct inverse and rollback
draft verification work for those spans. DC-15 then hardened active-session freshness, lower-level ref
publication, and signature input validation before broader algebra work.

The next roadmap item is M2+ patch algebra: commutation, confluence, conflict witnesses, and merge
evidence. The repository should not jump directly to merge execution or branch switching. The first
increment needs a precise vocabulary and a fail-closed witness shape so later implementations do not
silently guess when two patch histories interact.

DC-16 is that foundation. It defines how Prikk should talk about operation dependencies and conflicts,
which pairs may be treated as independently commutable in the first slice, and what evidence must be
reported when a pair is not safely commutable.

Architect review v1 accepted the direction with three required errata folded into this revision:

- path occupancy is an explicit preimage/postimage fact, so cross-node path interactions cannot be
  misclassified as independent;
- the vocabulary and witness taxonomy are captured in the companion FDD-01 update;
- every `Independent` classification must be backed by a test-level both-order replay oracle.

## Design Goals

1. Define a small patch-algebra vocabulary that is independent of CLI workflow.
2. Classify operation pairs as definitely independent, definitely conflicting, ordered-dependent, or
   unsupported/unknown.
3. Define conservative commutation boundaries for the currently supported node-addressed operation
   surface.
4. Define internal conflict witness fields that can be reported in diagnostics without becoming merge
   authority or a persisted wire object in this RFC.
5. Keep replay, inverse planning, rollback draft verification, and current command behavior fail-closed.
6. Avoid claims about semantic merge, branch switching, rollback refs, sync, or automatic resolution.

## Non-goals

DC-16 does not add:

- multi-parent Block publication;
- branch switching, branch copy/fork, merge-base discovery, branch deletion/rename, tags, or remote refs;
- rollback refs or rollback authorization;
- automatic conflict resolution or worktree merge writes;
- semantic/language-aware text merge;
- symlink authoring/application;
- key lifecycle, audit/plugin policy, or sync.

## Proposed Design

### Vocabulary

DC-16 should standardize these terms:

| Term | Meaning |
|---|---|
| Operation identity | The canonical operation bytes plus `op_seq` position inside a Patch. |
| Target node | The non-zero `node_id` addressed by an operation. |
| Preimage | The structured state facts the operation requires before application: live/dead status, occupied paths before application, required-free paths, kind, path, mode, blob id, old text span, or old symlink target. |
| Postimage | The structured state facts the operation establishes after application: occupied paths after application, paths freed, paths newly occupied, kind/path/mode/blob changes, or tombstone facts. |
| Independent pair | Two operations whose application order can be swapped without changing the final lifecycle state and without changing either operation's identity bytes. |
| Ordered dependency | Two operations where both may be valid, but one must apply before the other. |
| Conflict | Two operations cannot both be applied to the same baseline under the current rules. |
| Unknown | The pair is outside the DC-16 algebra subset and must fail closed instead of being treated as independent. |
| Conflict witness | A structured explanation of the exact state fact that prevents safe commutation or joint application. |

The critical rule is that "no known conflict" is not enough. A pair is independent only if DC-16 rules
prove it independent. Otherwise it is `Unknown` or a concrete conflict.

### First classification result

Add an internal classification result shape:

```text
PairClass =
  Independent
  OrderedDependency { required_order, reason }
  Conflict { witness }
  Unknown { reason }
```

This is a design shape, not necessarily the exact Rust enum name. The important property is that every
non-independent result carries a reason suitable for diagnostics and future review.

`required_order` is one of:

- `left_before_right`;
- `right_before_left`.

The classifier should never reorder an existing Patch. It only reports whether two operations from a
common baseline could be safely reordered or jointly considered by a later merge/confluence design.

### Initial operation subset

DC-16 should cover only operation kinds whose current replay/inverse behavior is well pinned:

- `CreateFile`;
- `DeleteNode` for file nodes;
- `EditText`;
- `ReplaceBinary`;
- `ChangePerm`.

The following remain `Unknown` in DC-16 classification:

- `RenamePath`;
- `CreateSymlink`;
- symlink `DeleteNode` cases beyond existing replay validation;
- future precondition records;
- operation kinds not recognized by the current decoder;
- any malformed operation that current replay would reject.

This keeps the first algebra layer aligned with shipped worktree authoring and rollback support. Later
DCs can widen the subset after symlink validation, rename semantics, and precondition records are
designed.

### Node-level independence

Operations targeting different `node_id`s are candidates for independence, but node id difference alone
is insufficient. The classifier must also check path-level effects. Path occupancy is a first-class
structured preimage/postimage effect set:

```text
PathEffects {
  occupied_before,
  required_free,
  occupied_after,
  freed,
  newly_occupied,
}
```

- two creates to the same path conflict even with different node ids;
- a create to a path occupied by another live node conflicts;
- a `CreateFile` requires its target path to be free and postimages that path as occupied;
- `required_free` participates in path-effect intersection checks; implementations must not infer
  independence by comparing postimages only;
- a file `DeleteNode` requires the target node to be live at its recorded path and postimages that path
  as free;
- a cross-node pair whose path-occupancy effects intersect is `OrderedDependency` or `Conflict`, never
  `Independent`;
- a delete-frees-path plus create-occupies-same-path pair is ordered-dependent when delete-before-create
  is valid, and conflict when neither order satisfies both preimages;
- a rename interaction that would alter path occupancy is outside DC-16 unless already covered by a
  later explicit subset;
- mode, content, and text-span operations on distinct live file nodes are independent when they do not
  create path occupancy changes.

For the DC-16 subset, cross-node independence is allowed only when both operations:

1. target different non-zero node ids;
2. have no intersecting path-occupancy preimage/postimage effects;
3. do not require one operation's postimage as the other's preimage;
4. are both inside the supported operation subset.

### Same-node classification

Same-node operation pairs are conservative by default.

Allowed same-node independent pair:

- `ChangePerm` and content-only mutation (`EditText` or `ReplaceBinary`) may commute when:
  - both target the same live file node;
  - `ChangePerm.old_mode` matches the baseline mode;
  - the content operation's old blob/text preimage matches the baseline content;
  - neither operation changes kind, node id, or path;
  - applying either order yields the same final mode and content blob.

Ordered dependencies:

- `CreateFile` before any mutation of that same node;
- content mutation before `DeleteNode` when the delete preimage includes the post-mutation content;
- `ChangePerm` before `DeleteNode` when the delete preimage includes the post-mode value;
- multiple `EditText` operations on the same node when the later edit localizes only after the earlier
  edit has been applied.

Conflicts:

- two `CreateFile` operations for the same `node_id` with different postimages;
- `CreateFile` for a `node_id` that already exists in the common baseline;
- `DeleteNode` versus any mutation that requires the deleted node to remain live, unless the delete
  explicitly depends on that mutation's postimage and is ordered after it;
- `ReplaceBinary` versus `EditText` on the same node;
- two `ChangePerm` operations from the same baseline that set different final modes;
- any preimage mismatch that current replay would classify as inconsistent.

Unknown:

- same-node pairs involving rename, symlink creation/application, or unsupported operation kinds;
- same-node text pairs whose relationship cannot be proven through replay-localized spans.

### Text edit classification

`EditText` needs stricter rules than file-level replacement because span identity is content-anchored.

Two `EditText` operations on different text nodes may be independent under the node-level rules above.

Same-node `EditText` independence is deferred from DC-16. Same-node text pairs must classify as
`OrderedDependency`, `Conflict`, or `Unknown`, never `Independent`, unless a later text-commutation DC
adds a full perturbation vector suite and explicit transform rule.

The deferred proof burden for a future DC is:

1. both localize against the same baseline text;
2. their old spans are disjoint byte ranges in that baseline;
3. neither replacement changes the byte positions or anchor context required by the other after order
   swap, or the swapped operation is re-derived under a later DC's explicit transform rule;
4. applying left then right and right then left yields byte-identical final text.

This design must not claim general operational transform, diff3, or CRDT behavior.

Overlapping text spans are conflicts unless a later DC defines a richer text merge witness. Adjacent
spans are `Unknown` or ordered-dependent in DC-16 because anchor context at the boundary is
perturbation-prone.

### Conflict witness shape

A conflict witness should name the smallest authoritative fact that blocks safe classification.

Design shape:

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

Fields are optional where not applicable. `kind` is one of:

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

This witness is diagnostic evidence, not an authorization or merge decision. It should be stable enough
for tests and review, but DC-16 does not freeze a public wire object or prepare a persisted witness
schema.

Conflict witnesses in DC-16 are internal diagnostic values for tests and classifier debugging. They are
not persisted objects, not canonical identity bytes, not part of Patch, Block, RefState, or RefUpdate
schema, and not a user-facing conflict-resolution format.

When multiple witness kinds apply, the classifier must report deterministically. Implementation should
define a fixed precedence order for witness kinds before adding tests so output does not depend on
decoder or map iteration order.

### Non-mutating analysis boundary

The first implementation should be non-mutating:

- classify operation pairs inside one Patch or between two candidate Patches against a common baseline;
- return diagnostics and witnesses;
- add no new object type and no new publication command;
- add no CLI surface in DC-16;
- preserve current replay and inverse behavior;
- fail closed on unsupported operations.

DC-16 is library/test-only. A diagnostic CLI can be designed later when merge/confluence has a caller
and an output contract worth preserving.

### Confluence boundary

DC-16 may define the precondition for future confluence checks:

Two patch sequences are confluence candidates only when:

1. they share an explicit baseline state;
2. every pairwise interaction is `Independent` or an ordered dependency whose required order is
   compatible with both sequences;
3. replaying both candidate orders yields byte-identical lifecycle-cache/state summaries.

DC-16 should not implement full confluence over multi-patch histories unless the first implementation
can pin the state summary and all unsupported-operation failure modes. It is acceptable to leave
confluence as a design contract for DC-17.

Production confluence equality checks are deferred to DC-17. DC-16 still requires a test-level
soundness oracle for independent pairs: for every pair the classifier labels `Independent`, tests must
replay left-then-right and right-then-left against the same baseline and assert structural equality of
the resulting lifecycle states. This oracle is test methodology, not a production API or state-root
claim.

The oracle must replay the same decoded operation identity bytes in alternate application order. It
must not rewrite operation payloads, change canonical bytes, or renumber `op_seq` to make a swapped
order pass.

Structural lifecycle-state equality includes live node map, path occupancy, node kind, path, mode, blob
id or symlink target where applicable, and tombstone/seen-id facts needed by lifecycle rules. It
excludes non-authoritative caches and diagnostic ordering.

### Compatibility

No object schema migration is required. Existing Patch, Block, RefState, RefUpdate, lifecycle cache, and
signature identity bytes remain unchanged.

Existing repositories remain valid. Histories containing operations outside the DC-16 classification
subset are still replayed according to existing rules; they are simply reported as `Unknown` by the
algebra classifier.

## Implementation Outline

1. Review this RFC before implementation.
2. Add an internal classifier module near patch replay/inverse code.
3. Define pair classification and witness types as internal diagnostic structures.
4. Reuse existing operation decoders and lifecycle-cache/replay preimage checks; do not create a second
   parser.
5. Add same-node and cross-node pair tests for the supported subset.
6. Add the both-order replay oracle to every test that expects `Independent`.
7. Add text-span pair vectors for overlapping, adjacent, stale-anchor, and ordered-dependent cases,
   while proving same-node text pairs are not classified as independent in DC-16.
8. Add negative tests proving rename, symlink, malformed, and unknown operation kinds fail closed as
   `Unknown` or `Conflict`, never `Independent`.
9. Define witness precedence before fixture expansion so outputs do not depend on map iteration or
   decoder order.
10. Ensure malformed operations that cannot decode into the classifier's normal input still produce
   `Unknown` / `Conflict` diagnostics through the surrounding classifier API instead of being skipped.
11. Update docs/status without claiming merge execution, branch workflows, rollback refs, or sync.

## Required Test Vectors

Required vectors:

- different-node content edits commute;
- different-node mode/content edits commute;
- same-path create/create conflicts;
- delete-frees-path plus create-occupies-same-path is ordered-dependent or conflict, never independent;
- same-node `ChangePerm` plus `EditText` commutes only when both preimages match the same baseline;
- same-node `ChangePerm` plus `ReplaceBinary` commutes only when both preimages match the same baseline;
- create-then-mutate is ordered-dependent;
- mutate-then-delete is ordered-dependent when delete preimage matches mutation postimage;
- delete-versus-mutate from the same baseline conflicts;
- two mode changes to different final modes conflict;
- `EditText` spans on different nodes commute;
- every vector expecting `Independent` replays both orders and asserts equal lifecycle states;
- the both-order oracle replays the same decoded operation identity bytes without renumbering `op_seq`;
- overlapping same-node `EditText` spans conflict;
- disjoint and adjacent same-node `EditText` spans are not accepted as independent in DC-16;
- stale text anchors produce a text-anchor witness;
- unsupported rename/symlink/precondition cases are `Unknown`;
- malformed operations are conflict/unknown diagnostics, not successful classifications.

Standard gates:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Out of Scope

- executing a merge;
- publishing multi-parent Blocks;
- branch switching, branch copy/fork, branch deletion/rename, tags, remotes, or sync;
- rollback refs or rollback authorization;
- automatic worktree conflict materialization;
- semantic text merge, CRDT, or operational transform;
- symlink application or rename classification;
- persistent conflict-witness objects.

## Open Questions Before Implementation

None after architect-review rulings:

- DC-16 is library/test-only; no CLI surface.
- Conflict witnesses remain internal diagnostics, not persisted wire objects.
- Same-node `EditText` independence is deferred to a dedicated text-commutation DC.
- Production confluence checks are deferred to DC-17, while DC-16 includes the test-level both-order
  replay oracle for independent-pair soundness.

## Review Rulings

Architect review v1 accepted the direction with the following implementation-gate rulings:

1. Path occupancy must be represented as preimage/postimage authority. Any cross-node pair whose
   occupancy effects intersect is ordered-dependent or conflicting, never independent.
2. The algebra vocabulary, pair taxonomy, and witness-kind taxonomy belong in FDD-01 as part of this
   design pass. FDD-03 needs no update because DC-16 changes no operation-record schema.
3. Every `Independent` classification in tests must be validated by replaying both orders and comparing
   resulting lifecycle states structurally.
4. DC-16 stays library/test-only; no diagnostic CLI.
5. Conflict witnesses stay internal diagnostics; no persisted witness object is prepared.
6. Same-node text independence is deferred.
7. Production confluence equality checks are deferred to DC-17.

Architect re-review v1 accepted DC-16 for implementation with five implementation checklist items:

1. `required_free` path occupancy must be explicit, or equivalently tested as an enforced create
   preimage. This RFC chooses explicit `required_free`.
2. Swapped-order oracle tests must not rewrite operation identity bytes or renumber `op_seq`.
3. Structural lifecycle-state equality must compare authoritative lifecycle state, not summaries or
   diagnostic ordering.
4. Witness precedence must be deterministic before broad fixture work.
5. Malformed or unknown cases must fail closed through `Unknown` / `Conflict` diagnostics and must not
   be silently skipped.

## Rejected Alternatives

### Start with branch merge UX

Rejected. Merge UX needs stable witnesses and confluence rules first. Starting at branch commands would
force policy decisions before the algebra can explain conflicts.

### Treat different node ids as automatically independent

Rejected. Path occupancy, create/delete effects, and preimage dependencies can cross node boundaries.
Node id difference is a useful filter, not proof.

### Make conflict witnesses public wire objects immediately

Rejected for the first slice. DC-16 should prove the witness vocabulary with tests without preparing or
freezing a durable object schema.

### Implement full text operational transform

Rejected. Prikk's current text identity is content-anchored. A general transform layer needs a separate
design and much stronger vector coverage.
