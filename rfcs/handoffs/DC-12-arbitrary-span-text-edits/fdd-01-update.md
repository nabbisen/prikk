# DC-12 FDD-01 Update - Arbitrary-Span Text Edit Apply/Inverse

Status: Implemented for v0.5.0, with inverse/rollback split
Related RFC: `../../proposed/DC-12-ARBITRARY-SPAN-TEXT-EDITS.md`
Target FDD: FDD-01 Patch Algebra

## Purpose

DC-12 turns the existing content-anchored `EditText` identity primitives into a real arbitrary-span
apply/generate surface. This is an M2 bridge, not the full commutation/confluence phase.

## Required FDD-01 Body Updates

### Text Span Selection

For a modified existing text node, the first supported authoring rule is a single smallest enclosing
byte span widened to UTF-8 character boundaries:

1. longest common prefix;
2. longest common suffix over the remaining tails;
3. tentative old and new spans from the unmatched ranges;
4. widen both starts by the same shared-prefix byte count when the old start must move to the previous
   UTF-8 character boundary;
5. widen both ends by the same shared-suffix byte count when the old end must move to the next UTF-8
   character boundary;
6. derive old span and replacement text from the widened ranges.

The selected byte boundaries must be UTF-8 character boundaries. Byte-level prefix/suffix may land
inside a multibyte character; this is widened, not rejected. Empty old spans represent insertions; empty
replacements represent deletions. No normalization or line-ending conversion is allowed.

### Text Span Localization

Application localizes the span by content identity, not byte offset:

- enumerate occurrences of `old_span_text` in canonical byte order;
- filter by left and right anchor hashes;
- recompute `span_id` using the zero-based index inside that anchor-filtered list;
- require exactly one match.

Failure to localize is a patch-application failure. The implementation must not fall back to offsets,
line numbers, fuzzy matching, or best-effort patching.

### Text Edit Application

For an `EditText` operation:

- the target `node_id` must name a live `TextFile` node;
- the current text blob, `old_span_text`, and `replacement_text` must be well-formed UTF-8;
- `old_span_hash` must equal `text_span_hash(old_span_text)`;
- successful localization yields `[start, end)`;
- application splices `replacement_text` over `[start, end)`;
- the node id, path, kind, and mode are preserved;
- the node's blob id becomes the canonical text blob id of the spliced bytes.

### Direct Inverse

The direct inverse of a supported arbitrary-span `EditText` swaps old and replacement text, but its
anchors and `span_id` are recomputed against the post-forward text. Reusing the forward anchors for the
inverse is invalid unless they happen to be recomputed to the same bytes by the rule above.

### Deferred Algebra

DC-12 does not define commutation laws for overlapping or adjacent text spans. It defines deterministic
generation and fail-closed application. Direct inverse and rollback exposure split to a follow-up
because the required round-trip vectors did not land in the v0.5.0 cut.

## Required Tests

- byte-level vectors for span selection and `span_id` computation;
- sub-character byte-boundary widening vectors, including `é` -> `è` and a CJK example;
- replacement, insertion, deletion, repeated occurrence, and CRLF-preserving application;
- stale anchor, no matching span id, ambiguous localization, hash mismatch, wrong node kind, and
  invalid UTF-8 negatives;
- inverse planning fails closed for arbitrary-span `EditText` until direct-inverse vectors land;
- no offset or presentation-hint fallback is used.
