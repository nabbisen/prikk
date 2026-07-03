# DC-14 FDD-03 Update - Inverse EditText Identity Bytes

Status: Accepted for v0.7.0 implementation after design re-review v1
Related RFC: `../../proposed/DC-14-ARBITRARY-SPAN-TEXT-INVERSE-ROLLBACK.md`
Target FDD: FDD-03 Object Schema and Canonical Identity

## Purpose

DC-14 does not add object fields or encodings. It clarifies which existing `EditText` bytes are emitted
when an inverse Patch is generated for rollback planning or rollback draft authoring.

## Required FDD-03 Body Updates

### No Schema Change

The inverse of an arbitrary-span text edit is encoded as the existing `EditText` operation record:

- `node_id`;
- `span_id`;
- `old_span_hash`;
- `left_anchor_hash`;
- `right_anchor_hash`;
- `replacement_text`;
- optional `presentation_hint_line`;
- optional `presentation_hint_column`;
- `old_span_text`.

No wire fields, tags, object types, or canonical encodings are added.

### Inverse Record Bytes

For a forward `EditText`, the inverse record uses:

- the same `node_id`;
- `old_span_text = forward.replacement_text`;
- `replacement_text = forward.old_span_text`;
- `old_span_hash = text_span_hash(forward.replacement_text)`;
- `left_anchor_hash` and `right_anchor_hash` recomputed over the post-forward text at the inverse old
  span range;
- `span_id` recomputed from the inverse `old_span_hash`, inverse anchors, inverse duplicate index, and
  `node_id`;
- absent presentation hints.

The post-forward text is part of deterministic inverse identity derivation. Reusing forward anchors,
forward `span_id`, or non-derived presentation hints produces a different Patch payload and must fail
rollback-draft verification.

`PatchPurpose::RollbackDraft` is the rollback-draft discriminator. A placeholder AUTHOR key id or
marker signature is not rollback-draft authority. Generated rollback draft Patch envelopes must carry
the real role-bound Ed25519 AUTHOR signature required by the DC-10 signing contract; trust-store and
authorization policy remain outside FDD-03 schema semantics.

### Validation

Existing object validation remains authoritative:

- all-zero `node_id` is rejected;
- `old_span_hash != text_span_hash(old_span_text)` is rejected;
- non-UTF-8 `old_span_text` or `replacement_text` is rejected;
- malformed canonical field layout is rejected.

Runtime replay/inverse derivation owns node-lifecycle and localization checks. The object schema still
does not store authoritative byte offsets.

## Required Tests

- canonical byte vectors proving inverse `EditText` uses recomputed anchors and `span_id`;
- stale-forward-anchor inverse payload rejects during rollback-draft verification;
- presentation hints remain absent in generated inverse payloads and any non-absent generated inverse
  hint rejects during rollback-draft verification;
- a normal-purpose Patch with otherwise byte-identical inverse operations is not accepted as a
  rollback draft;
- existing DC-12 `EditText` layout vectors remain unchanged.
