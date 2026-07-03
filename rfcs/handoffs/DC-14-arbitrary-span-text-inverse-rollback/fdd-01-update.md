# DC-14 FDD-01 Update - Arbitrary-Span Text Direct Inverse

Status: Accepted for v0.7.0 implementation after design re-review v1
Related RFC: `../../proposed/DC-14-ARBITRARY-SPAN-TEXT-INVERSE-ROLLBACK.md`
Target FDD: FDD-01 Patch Algebra

## Purpose

DC-14 completes the direct inverse rule for the arbitrary-span `EditText` apply surface introduced by
DC-12. It is still an M2 bridge: rollback can derive and verify inverse text edits, but commutation,
confluence, conflict witnesses, and semantic merge remain deferred.

## Required FDD-01 Body Updates

### Direct Inverse for EditText

For a supported forward `EditText`, the inverse is derived while applying the forward operation to the
planner's authoritative in-memory text state:

1. localize the forward span in the pre-forward text using the existing anchor-filtered `span_id` rule;
2. splice `replacement_text` over that range to produce post-forward text;
3. select the inverse old span as the exact replacement range in the post-forward text;
4. set inverse `old_span_text = forward.replacement_text`;
5. set inverse `replacement_text = forward.old_span_text`;
6. recompute inverse `old_span_hash`, left anchor, right anchor, duplicate index, and `span_id` against
   the post-forward text;
7. re-localize the derived inverse in the post-forward text and require the located range to equal the
   exact selected inverse range;
8. apply the derived inverse and require byte-exact recovery of the pre-forward text.

All inverse range lengths are byte lengths. `inverse_start` and `inverse_end` must be valid UTF-8 byte
boundaries in the post-forward text, and empty spans are valid only at UTF-8 insertion-position
boundaries.

Forward anchors and forward `span_id` must not be reused for the inverse unless the recomputation rule
independently produces the same bytes.

### Empty Spans

The same rule covers insertions and deletions:

- forward insertion: empty forward `old_span_text`, inverse deletes the inserted bytes;
- forward deletion: empty forward `replacement_text`, inverse inserts the deleted bytes at the
  zero-length post-forward position;
- replacement: inverse swaps the two spans and recomputes identity.

Empty old spans are localized through the existing canonical insertion-position enumeration. No offset
fallback is allowed.

### Operation Ordering

Inverse planning continues to replay history in forward order, accumulate inverse operations, reverse
the accumulated inverse list, and renumber from 1..N. Multiple text operations against the same node
therefore roll back in reverse application order.

The implementation gate must include a hard ordering vector with two `EditText` operations against the
same `node_id`, where the second forward edit depends on the first edit's post-text. Emitting inverses
in forward order must fail the vector.

### Rollback Draft Verification

Rollback-draft verification compares derived canonical payload bytes, not summaries or semantic
equivalence. Verification must recompute the complete inverse `PatchPayload`, apply the same reverse
and renumber rules as inverse planning, require generated inverse presentation hints to be absent, and
then compare canonical payload identity bytes.

Rollback draft identity is `PatchPurpose::RollbackDraft`; a normal-purpose Patch with byte-identical
operations is not a valid rollback draft. The Patch envelope must carry the real role-bound Ed25519
AUTHOR signature required by the DC-10 rollback-draft signing contract. This does not add AUTHOR
trust-store enforcement or rollback authorization policy.

Without an AUTHOR trust store or supplied AUTHOR public-key source, rollback-draft verification must
not claim full cryptographic trust validation of arbitrary historical AUTHOR signatures. It must still
reject missing, wrong-role, wrong-algorithm, malformed, marker/placeholder, and purpose-mismatched
AUTHOR signature records. For Ed25519, malformed includes a signature payload whose length is not 64
bytes.

### Fail-Closed Conditions

Direct inverse derivation fails closed when:

- the forward edit cannot be localized exactly once;
- the target node is missing or is not a live `TextFile`;
- `old_span_hash` does not match `old_span_text`;
- either text span is not well-formed UTF-8;
- the inverse span cannot be found in the post-forward text through the same anchor-filtered rule;
- the derived inverse localizes to a range other than the exact selected inverse range;
- operation count or duplicate-index numbering overflows;
- the derived inverse does not replay cleanly against the post-forward text.

Unsupported node operations outside the DC-14 subset remain unsupported; DC-14 must not silently widen
rollback to unrelated patch kinds.

## Required Tests

- direct inverse vectors for replacement, insertion, deletion, repeated text, CRLF, UTF-8 widening, and
  multi-hunk enclosing spans;
- forward-then-inverse and inverse-then-forward byte-exact round trips;
- hard-gated multiple operation ordering against the same text node;
- negatives for stale anchors, stale `span_id`, hash mismatch, wrong node kind, invalid UTF-8,
  unresolvable localization, and unsupported operation kinds;
- rollback preview/draft/verify coverage for inverse `EditText` payloads;
- rollback-draft verify negatives for normal-purpose patches, missing, wrong-role, wrong-algorithm,
  malformed Ed25519 length, marker/placeholder, and purpose-mismatched AUTHOR signature records, stale
  inverse identity bytes, and non-absent generated inverse presentation hints.
