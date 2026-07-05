# DC-17 FDD-01 Update - Patch Algebra Evidence Contract

Status: Implemented with DC-17 / v0.10.0
Related RFC: `../../done/DC-17-PATCH-ALGEBRA-EVIDENCE-CONTRACT.md`
Target FDD: FDD-01 Patch Algebra

## Purpose

DC-17 extends the DC-16 patch-algebra vocabulary with a production evidence boundary. DC-16 defined how
operation pairs are classified. DC-17 defines which facts the classifier may trust, how those facts are
obtained from replay and object storage, and how evidence failures differ from unsupported algebra.

This update does not add merge execution, multi-parent publication, public CLI diagnostics, or persisted
conflict witnesses.

## Required FDD-01 Body Updates

### Evidence Authority

Patch algebra classification uses these authoritative evidence sources:

- baseline lifecycle state from replay-derived `NodeLifecycleState`;
- path occupancy from the same lifecycle state;
- operation preimage/postimage facts from decoded operation payloads;
- blob kind from validated object-store blob lookup;
- blob bytes from validated object-store blob lookup;
- baseline text from a live text-file node's lifecycle record plus validated blob bytes.

The classifier must not infer authoritative facts from worktree state, caller assertions, diagnostic
summaries, or non-authoritative caches.

### Evidence Result Vocabulary

FDD-01 should define an internal evidence result vocabulary:

```text
Evidence<T> =
  Known(T)
  Missing { fact }
  WrongObjectType { object_id, expected, actual }
  WrongBlobKind { blob_id, expected, actual }
  Malformed { fact, reason }
  Unreadable { fact, reason }
```

Names may differ in Rust, but these semantic states must remain visible to the classifier. They must
not collapse into `Option<T>` or default values.

Each evidence request must carry explicit provenance:

```text
EvidenceScope =
  SealedBaselineRequired
  SealedCandidateRequired
  UnsealedCandidateOptional
```

Minimum mapping:

- missing sealed-baseline evidence is an integrity error;
- missing sealed-candidate evidence is an integrity error;
- missing unsealed candidate evidence may be `Unknown { missing_candidate_evidence }`.

The classifier must not infer this mapping from loose call-site context or generic `None` behavior.

### Resolver Contract

The production resolver is read-only and internal. Design shape:

```text
PatchAlgebraEvidence {
  baseline_state() -> NodeLifecycleState
  blob_kind(blob_id) -> Evidence<BlobKind>
  blob_bytes(blob_id) -> Evidence<(BlobKind, bytes)>
  baseline_text(node_id) -> Evidence<bytes>
  create_blob_text(blob_id) -> Evidence<bytes>
}
```

The resolver may use cache-backed replay and store readers, but the contract is stated in terms of
authoritative replay and object evidence. It must not mutate refs, blocks, patches, worktrees, or object
storage.

Resolver construction is valid only after binding and validating:

- selected baseline block id;
- replay horizon;
- replay-derived lifecycle state;
- object store used for blob validation.

If baseline lookup, replay, lifecycle construction, or required object-store access fails,
classification must not proceed. There is no default-empty lifecycle fallback for this sealed-baseline
evidence contract.

Blob validation order is:

1. object id resolves to an object;
2. object type is `Blob`;
3. blob payload decodes canonically;
4. blob kind matches the requested fact;
5. bytes are returned only after those checks pass.

Baseline text and candidate create-blob reads are separate authority paths:

- `baseline_text(node_id)` proves live `TextFile` lifecycle state and validates that node's lifecycle
  blob before returning bytes;
- `create_blob_text(blob_id)` validates candidate create evidence and may return
  `Unknown { missing_candidate_evidence }` only under `UnsealedCandidateOptional`.

### Evidence-to-Classification Mapping

FDD-01 should record these mappings:

- unsupported operation kinds remain `Unknown`;
- future preconditions outside the current algebra remain `Unknown`;
- same-node text commutation without a transform rule remains `Unknown`, ordered, or conflict, never
  independent;
- missing optional candidate evidence for a not-yet-published operation is `Unknown`;
- missing, wrong-type, malformed, or unreadable evidence required by a sealed baseline is an integrity
  failure;
- valid baseline evidence that disagrees with a valid operation preimage is a `Conflict`;
- no evidence failure path may classify as `Independent`.

Integrity errors must have a distinct result surface, preferably `Result<PairClass, EvidenceError>`.
If modeled as a classification variant, it must not be confusable with `Unknown`.

### `CreateFile` to `ChangePerm`

For a same-node pair:

- `CreateFile` establishes a file node and initial mode;
- `ChangePerm.old_mode` must match the create operation's initial mode;
- matching mode classifies as ordered create-before-change;
- mismatching mode classifies as `Conflict { mode_mismatch }`;
- change-before-create is not independent;
- baseline node-id reuse and create-path occupancy still participate in conflict checks.

### Conflict Witness Policy

Conflict witnesses remain internal diagnostics:

- not persisted;
- not signed;
- not part of Patch, Block, RefState, or RefUpdate schema;
- not a public CLI or JSON contract.

Optional `expected` and `actual` fields may be populated only for deterministic scalar facts such as
kind, mode, blob id, live/dead state, and path occupancy. Text bytes must not be copied into witnesses.
Evidence errors must stay evidence errors rather than being represented as ordinary conflict witnesses.

Implementation may remove optional witness fields if they are not populated deterministically.

If retained, `expected` and `actual` must be typed diagnostic values, not arbitrary strings:

```text
WitnessValue =
  Mode(u32)
  BlobId(ObjectId)
  NodeKind(NodeKind)
  Path(RepoPath)
  LiveState(Live | Dead | Missing)
  Occupancy(Free | OccupiedBy(NodeId))
```

### Lifecycle-State Structural Equality

Any confluence-adjacent test that compares lifecycle states must compare authoritative lifecycle facts:

- live node ids;
- node kind;
- path;
- normalized mode;
- blob id or symlink target;
- tombstone/latest-deletion facts when in scope;
- path occupancy.

It must not compare diagnostic summaries or map iteration order.

## Required Tests

- store-backed baseline text evidence succeeds for a live text file;
- resolver construction is bound to baseline block id, replay horizon, lifecycle state, and object
  store;
- replay/lifecycle construction failure prevents classification;
- missing sealed-baseline blob is an integrity failure;
- missing sealed-candidate blob is an integrity failure;
- missing unsealed candidate blob is `Unknown { missing_candidate_evidence }`;
- wrong object type for blob evidence is an integrity failure;
- wrong blob kind for text evidence fails closed;
- blob validation follows object lookup, object type, canonical blob decode, blob kind, then bytes;
- baseline text and create-blob text use separate authority paths;
- `CreateFile` to `ChangePerm` matching mode is ordered create-before-change;
- `CreateFile` to `ChangePerm` mismatching mode is `mode_mismatch`;
- `ChangePerm` before same-node `CreateFile` is not independent;
- create path occupied in baseline conflicts through path occupancy evidence;
- unsupported operations remain `Unknown`;
- no evidence failure classifies as `Independent`;
- witness diagnostics are deterministic and do not include text bytes;
- `expected` / `actual` witness values are typed or absent;
- lifecycle structural equality tests ignore diagnostic summaries and map iteration order.

## Implementation Errata Checklist

These checklist items are required by design re-review before implementation can be accepted:

1. Make `EvidenceScope` visible in the concrete API by explicit arguments, separate scope-specific
   methods, or typed evidence request objects. Do not hide scope in comments or generic `Option<T>`.
2. Prefer `Result<PairClass, EvidenceError>` for classification. Use an enum alternative only if the
   evidence-error case cannot be mistaken for algebra `Unknown`.
3. Pin missing-evidence tests by scope as separate cases:
   missing sealed-baseline blob is an evidence error; missing sealed-candidate blob is an evidence
   error; missing unsealed candidate blob is `Unknown { missing_candidate_evidence }`.
4. Keep `CreateFile` to `ChangePerm` independent of blob/text evidence. The relation uses node-id
   non-liveness, create-path free, file node kind, create initial mode, and `ChangePerm.old_mode`.
5. Defer production confluence unless resolver-backed state equality and evidence-failure behavior are
   already pinned. The resolver contract and classifier integration are the priority for DC-17.
