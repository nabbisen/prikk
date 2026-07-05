# RFC (done) - DC-17 Patch Algebra Evidence Contract

**Status.** Implemented and released in v0.10.0.
**Released.** v0.10.0.
**Tracks.** Production evidence boundary for the M2+ patch-algebra classifier introduced by DC-16.
**Touches.** Patch algebra resolver API, lifecycle-cache authority, object-store evidence reads,
conflict witness diagnostics, and the FDD-01 patch-algebra vocabulary.
**Companion FDD updates.** `../handoffs/DC-17-patch-algebra-evidence-contract/fdd-01-update.md`.

## Context

DC-16 introduced Prikk's first patch-algebra vocabulary and internal pair classifier. It intentionally
stopped short of production confluence: the classifier could reason about supported operation pairs,
but its baseline text and lifecycle facts were supplied through an abstract resolver boundary.

That was the right first slice. The next slice must make the evidence boundary explicit before Prikk
claims production merge, confluence, or branch behavior. A classifier is only as strong as the facts it
is allowed to trust. If missing blobs, stale baseline text, malformed operation payloads, or unresolved
lifecycle facts collapse into ordinary `Unknown`, later code could accidentally treat repository
integrity failures as harmless unsupported cases.

DC-17 defines the production evidence contract for patch algebra. It says which facts are authoritative,
how the resolver obtains them, how evidence failures map to classifier results, and which witness fields
remain diagnostic rather than public schema.

## Design Goals

1. Define the authoritative evidence sources for patch-algebra classification against a sealed baseline.
2. Replace test-only baseline resolvers with a production-ready internal evidence contract.
3. Distinguish unsupported algebra from missing, unreadable, or inconsistent repository evidence.
4. Keep pair classification read-only and internal; do not add merge execution or CLI behavior.
5. Close DC-16 follow-up gaps around `CreateFile` to `ChangePerm`, resolver authority, and conflict
   witness field policy.
6. Preserve fail-closed behavior: no evidence gap may be reported as `Independent`.

## Non-goals

DC-17 does not add:

- multi-parent Block publication;
- branch switching, branch copy/fork, merge-base discovery, branch deletion/rename, tags, or remote refs;
- rollback refs or rollback authorization;
- automatic merge execution or worktree conflict materialization;
- semantic/language-aware text merge;
- same-node text commutation;
- symlink authoring/application or rename classification;
- public CLI diagnostics;
- persisted conflict-witness objects;
- schema changes to Patch, Block, RefState, or RefUpdate.

## Proposed Design

### Evidence Authority

Patch algebra classification may use only these authoritative fact sources:

| Fact | Authority |
|---|---|
| Baseline lifecycle state | Replay-derived `NodeLifecycleState` for the selected sealed baseline and horizon. |
| Live path occupancy | The same replay-derived lifecycle state, including required-free and occupied paths. |
| Operation preimage/postimage | The already decoded operation payload plus DC-16 preimage/postimage extraction. |
| Blob kind | Object store lookup of the referenced blob object. |
| Blob bytes | Object store lookup of the referenced blob object, after object type and blob kind validation. |
| Baseline text | Text bytes for a live text-file node proven by lifecycle state and blob evidence. |

No classifier path may infer text, kind, path, mode, or blob content from non-authoritative caches,
diagnostic summaries, file-system state, or caller assertions. Caches may speed up lookup, but the
resolver contract must be phrased in terms of authoritative replay and object evidence.

### Evidence Result Shape

The resolver should expose an internal evidence result shape. Exact Rust names are implementation
details, but the semantic states are required:

```text
Evidence<T> =
  Known(T)
  Missing { fact }
  WrongObjectType { object_id, expected, actual }
  WrongBlobKind { blob_id, expected, actual }
  Malformed { fact, reason }
  Unreadable { fact, reason }
```

`Missing` means the referenced object or lifecycle fact is absent. `WrongObjectType`, `WrongBlobKind`,
`Malformed`, and `Unreadable` mean the repository evidence exists in a form that cannot satisfy the
requested fact.

The classifier must not silently coerce these states into `None`, empty text, empty path sets, or
default modes.

Every evidence request must also carry an explicit provenance class:

```text
EvidenceScope =
  SealedBaselineRequired
  SealedCandidateRequired
  UnsealedCandidateOptional
```

Names may differ in Rust. The rule may not be inferred loosely from call-site convention:

- missing evidence required by the selected sealed baseline is an integrity error;
- missing evidence required by a sealed candidate Patch under analysis is an integrity error;
- missing evidence for an unsealed, not-yet-published candidate operation may classify as
  `Unknown { missing_candidate_evidence }`.

The same blob id can therefore map to a different outcome depending on whether it is sealed repository
authority or optional unsealed candidate evidence.

### Resolver Boundary

Introduce a read-only internal resolver boundary for production classification. The design shape is:

```text
PatchAlgebraEvidence {
  baseline_state() -> NodeLifecycleState
  blob_kind(blob_id) -> Evidence<BlobKind>
  blob_bytes(blob_id) -> Evidence<(BlobKind, bytes)>
  baseline_text(node_id) -> Evidence<bytes>
  create_blob_text(blob_id) -> Evidence<bytes>
}
```

This shape is not a public API commitment. It documents the required authority split:

- lifecycle state is built once from a selected sealed baseline;
- object evidence is read from the store and validated by object type and blob kind;
- baseline text is derived from the live node's lifecycle record plus validated blob bytes;
- create-operation text evidence is derived from the create blob referenced by the operation;
- all methods are read-only and must not mutate refs, blocks, patches, worktrees, or object storage.

The resolver may be constructed from existing store/replay components. It must not duplicate operation
decoding or create an alternate replay engine.

Resolver construction must be baseline-bound and fail closed. A production resolver exists only after
the implementation has validated:

- selected baseline block id;
- replay horizon;
- replay-derived lifecycle state;
- object store used for blob validation.

If baseline block lookup, replay, lifecycle construction, or required object-store access fails,
classification must not proceed. There is no default-empty lifecycle fallback except true genesis
authoring, which is outside the DC-17 sealed-baseline evidence case.

Object validation order is fixed:

1. the object id resolves to an object;
2. the object type is `Blob`;
3. the blob payload decodes canonically;
4. the blob kind matches the requested fact;
5. bytes are returned only after those checks pass.

Wrong object type and malformed blob payload are evidence/integrity states, not conflict witnesses.

`baseline_text(node_id)` and `create_blob_text(blob_id)` must remain separate authority paths:

- `baseline_text(node_id)` proves the node is live, its kind is `TextFile`, and its lifecycle blob id
  resolves to a valid Text blob;
- `create_blob_text(blob_id)` validates the candidate create blob and may return
  `Unknown { missing_candidate_evidence }` only when the caller explicitly marks the request as
  `UnsealedCandidateOptional`.

### Classification Mapping

Evidence failures must map deterministically:

| Condition | Classification behavior |
|---|---|
| Operation kind outside the DC-16 subset | `Unknown { unsupported_operation }`. |
| Future precondition record outside current algebra | `Unknown { future_precondition_deferred }`. |
| Same-node text pair without a DC-approved transform rule | `Unknown { same_node_text_commutation_deferred }` or ordered/conflict when proven. |
| Missing optional candidate blob for a not-yet-published operation | `Unknown { missing_candidate_evidence }`. |
| Missing baseline object required by a sealed baseline | repository integrity error at resolver construction or classification entry. |
| Wrong object type for required baseline blob | repository integrity error. |
| Wrong blob kind for a required baseline text fact | repository integrity error or `Conflict` only when comparing valid operation evidence to valid baseline evidence. |
| Valid preimage value disagrees with valid baseline fact | `Conflict { witness }`. |
| Valid evidence proves order dependence | `OrderedDependency { required_order, reason }`. |

The boundary is deliberate: unsupported algebra is `Unknown`; corrupt or unreadable repository evidence
is not just unsupported algebra. Callers must be able to tell the difference.

The implementation must make integrity errors unambiguous in the result surface. Prefer an outer result
such as:

```text
Result<PairClass, EvidenceError>
```

If an enum variant is used instead, it must not be confusable with `Unknown`. `Unknown` remains an
ordinary fail-closed algebra classification, not a repository-integrity error.

### `CreateFile` to `ChangePerm`

DC-17 must pin the same-node `CreateFile` to `ChangePerm` relation because it is a simple case that
exercises resolver authority.

For a `CreateFile` followed by `ChangePerm` on the same node:

- the create operation proves the node kind is file-like for the newly created node;
- the create operation establishes the initial mode;
- `ChangePerm.old_mode` is checked against that initial mode;
- matching mode evidence yields `OrderedDependency { left_before_right }`;
- mismatching mode evidence yields `Conflict { mode_mismatch }`;
- an operation claiming to mutate the node before it exists is not independent.

This rule does not require baseline text. It does require the lifecycle facts that the node id is not
already live in the selected baseline and that the create path is free.

### Conflict Witness Policy

DC-16 allowed optional `expected` and `actual` witness fields but did not make them a stable public
diagnostic contract. DC-17 keeps witnesses internal and narrows their authority:

- witness fields are test-stable internal diagnostics only;
- no witness is persisted, signed, or exposed as a public wire object;
- `expected` and `actual` may be populated only for deterministic scalar facts such as kind, mode,
  blob id, live/dead state, or path occupancy;
- text bytes must not be copied into witnesses;
- missing, malformed, or unreadable repository evidence must be reported through evidence/error state,
  not disguised as an `expected`/`actual` conflict.

If implementation finds the optional fields encourage speculative strings or unused-code suppressions,
it should remove them in DC-17 rather than keep a misleading diagnostic shape for future UX.

If `expected` and `actual` remain, they must be typed diagnostic values rather than arbitrary strings.
The allowed shape should be limited to deterministic scalar facts:

```text
WitnessValue =
  Mode(u32)
  BlobId(ObjectId)
  NodeKind(NodeKind)
  Path(RepoPath)
  LiveState(Live | Dead | Missing)
  Occupancy(Free | OccupiedBy(NodeId))
```

If that shape is too much for this release, remove `expected` and `actual` for now and keep witness
kind plus minimal ids. Do not introduce prose fields that tests might accidentally freeze as public
diagnostics.

### Confluence Boundary

DC-17 prepares production confluence by defining evidence authority, but it still does not execute or
publish merges.

It may add an internal read-only confluence candidate check only if all of the following are true:

1. both candidate sequences share the same explicit sealed baseline;
2. every pairwise interaction is `Independent` or an ordered dependency compatible with both sequences;
3. every required evidence read is `Known`;
4. unsupported operations or evidence failures fail closed;
5. equality compares authoritative lifecycle state, not diagnostic summaries.

If these conditions cannot be pinned in a small implementation, DC-17 should stop at the resolver
contract and classifier integration. A later DC can own multi-sequence confluence.

Any test that compares resulting lifecycle states must define structural equality precisely. It should
include at least:

- live node ids;
- node kind;
- path;
- normalized mode;
- blob id or symlink target;
- tombstone/latest-deletion facts when in scope;
- path occupancy.

It must not compare diagnostic summaries or map iteration order.

### Compatibility

No object schema migration is required. Existing repositories remain valid. Histories that contain
operations outside the current algebra subset continue to replay under existing rules and classify as
`Unknown` for algebra purposes.

## Implementation Outline

1. Review this RFC before implementation.
2. Add the internal evidence result type and resolver trait/boundary near the patch-algebra module.
3. Add explicit evidence provenance/scope to resolver requests.
4. Add a store-backed resolver constructor over existing replay/lifecycle-cache and object-store
   readers.
5. Convert classifier paths that need baseline text, blob kind, or blob bytes to use evidence results
   instead of test-only optional values.
6. Make sealed-baseline evidence failures distinguishable from unsupported algebra through a distinct
   error channel.
7. Add `CreateFile` to `ChangePerm` vectors for matching mode, mismatching mode, missing baseline facts,
   and create-path occupancy conflicts.
8. Apply the conflict witness policy and remove any unused witness fields that are not populated
   deterministically.
9. Keep all APIs crate-internal unless a later RFC designs a public diagnostic surface.
10. Update FDD-01 and RFC status/index files without claiming merge execution.

## Required Test Vectors

Required vectors:

- store-backed resolver can read live baseline text for a text-file node;
- resolver construction is bound to selected baseline block id, replay horizon, lifecycle state, and
  object store;
- baseline lookup/replay/lifecycle construction failure prevents classification;
- missing required sealed-baseline blob is an integrity failure, not `Independent` or ordinary
  unsupported algebra;
- missing sealed-candidate blob evidence is an integrity failure;
- missing unsealed candidate blob evidence is `Unknown { missing_candidate_evidence }`;
- wrong object type for a referenced blob is an integrity failure;
- wrong blob kind for required text evidence fails closed;
- blob validation follows object lookup, object type, canonical blob decode, blob kind, then bytes;
- `baseline_text(node_id)` proves live text-file lifecycle state before reading bytes;
- `create_blob_text(blob_id)` uses explicit candidate evidence scope;
- `CreateFile` then `ChangePerm` with matching mode is ordered create-before-change;
- `CreateFile` then `ChangePerm` with mismatching mode is `Conflict { mode_mismatch }`;
- `ChangePerm` before same-node `CreateFile` is not independent;
- create path already occupied in baseline conflicts through path occupancy evidence;
- valid baseline preimage mismatch produces a conflict witness, not an evidence error;
- unsupported rename/symlink/precondition cases remain `Unknown`;
- no evidence failure path is classified as `Independent`;
- witness output is deterministic and does not include text bytes;
- `expected` / `actual` witness values are typed deterministic scalar facts or absent;
- lifecycle-state structural equality tests compare authoritative lifecycle facts, not diagnostic
  summaries or map iteration order.

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
- automatic conflict-file generation;
- persisted conflict witnesses;
- public CLI or JSON diagnostics;
- same-node text transform rules.

## Review Questions

Architect review should confirm:

1. sealed-baseline missing/wrong/malformed evidence is an integrity failure rather than `Unknown`;
2. missing candidate evidence may be `Unknown` only when the candidate is not yet repository authority;
3. the `CreateFile` to `ChangePerm` relation is the right required coverage for DC-17;
4. conflict witness `expected`/`actual` fields should either be deterministically populated or removed;
5. production multi-sequence confluence may be deferred if the resolver contract is implemented first.

Architect review v1 accepted DC-17 with implementation errata. This revision folds in the required
errata:

1. evidence requests carry explicit provenance/scope;
2. resolver construction is baseline-bound and fail-closed;
3. blob validation order is explicit;
4. baseline text and candidate create-blob reads remain separate authority paths;
5. lifecycle-state structural equality is defined for confluence-adjacent tests;
6. witness `expected` / `actual` values must be typed or removed;
7. integrity errors use a distinct result channel from `Unknown`.

## Rejected Alternatives

### Treat all missing evidence as `Unknown`

Rejected. Missing evidence from a sealed baseline can indicate repository corruption or an invalid
resolver horizon. Treating that as an unsupported algebra case would make later merge logic unsafe.

### Add merge UX now

Rejected. Merge UX depends on a trustworthy evidence boundary and diagnostic contract. DC-17 should
finish that boundary first.

### Freeze conflict witnesses as public schema

Rejected. Witnesses are still diagnostic implementation evidence. A future public conflict-reporting
format should be designed after resolver-backed classifications have test coverage.
