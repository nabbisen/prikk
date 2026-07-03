# RFC (proposed) - DC-12 Arbitrary-Span Text Edits

**Status.** Accepted; implementation landed for v0.5.0 candidate.
**Target release.** v0.5.0.
**Tracks.** Replacing whole-file `EditText` generation/replay with deterministic arbitrary-span
text-span generation and application, as the next M2 bridge before full patch algebra.
**Touches.** `prikk-store` text-span generation, patch replay, patch materialization, inverse planning,
rollback preview/draft verification, worktree patch authoring, docs, status, release notes, and golden
vectors.
**Companion FDD updates.** `../handoffs/DC-12-arbitrary-span-text-edits/fdd-01-update.md`,
`../handoffs/DC-12-arbitrary-span-text-edits/fdd-03-update.md`.

## Context

v0.4.0 closed the local publication-trust placeholder. The highest-value next patch-engine step is
arbitrary-span text edits: the repository already has the FDD-01 text-span identity primitives
(`left_anchor`, `right_anchor`, `compute_span_id`, `locate_text_span`, `splice_text`, and text blob
identity), but pre-DC-12 worktree authoring planned a whole-file `EditText` for modified UTF-8 text
files.

DC-12 is a design-first bridge to M2+ patch algebra. It should make real text edits smaller and more
local without claiming commutation, confluence, conflict witnesses, or semantic merge. Those remain
separate FDD-01 increments after the concrete apply/generate surface is stable and covered by vectors.

## Design goals

1. Generate deterministic arbitrary-span `EditText` records for modified existing text-file nodes.
2. Apply arbitrary-span `EditText` records through the existing shared text-span localization and
   splice primitives.
3. Preserve the current FDD-03 `EditText` canonical record layout; DC-12 must not add identity fields.
4. Keep replay and materialization fail-closed when localization, UTF-8 validity, node kind, or
   preconditions do not match exactly.
5. Keep authoring and replay on one implementation path for identity-bearing span logic.
6. Add golden vectors that pin span selection, anchors, `span_id`, replay splice output, and negative
   localization cases.

## Proposed design

### Scope

DC-12 supports a single deterministic arbitrary span per modified text file.

The first implementation should emit one `EditText` per modified tracked `TextFile` node when both
baseline and current bytes are well-formed UTF-8. If a modified `TextFile` cannot be represented under
the DC-12 rules, authoring fails closed. It must not silently turn a text-node mutation into
`ReplaceBinary`, because `ReplaceBinary` is binary-node-only in the node model.

Multiple edit hunks in one file may be represented as one enclosing span. Multi-operation text diff
minimization is deliberately deferred until the ordering and commutation rules are designed.

### Span selection

Given old bytes `old` and new bytes `new`, choose the smallest byte range that covers the change, then
widen it to UTF-8 character boundaries:

1. compute the longest common prefix length `prefix`;
2. compute the longest common suffix length over the remaining tails without crossing `prefix`;
3. set tentative old range `prefix..old.len() - suffix` and tentative new range
   `prefix..new.len() - suffix`;
4. if the tentative old start is not a UTF-8 character boundary, decrease both starts by the same
   number of shared-prefix bytes to the previous boundary;
5. if the tentative old end is not a UTF-8 character boundary, increase both ends by the same number
   of shared-suffix bytes to the next old-text boundary;
6. set `old_span_text` and `replacement_text` from those widened ranges.

Rules:

- ranges are byte ranges over validated UTF-8 files;
- byte-level prefix/suffix may land inside a multibyte character, but the emitted ranges must be
  widened to enclosing UTF-8 character boundaries rather than rejected;
- an insertion is represented by an empty `old_span_text` at `prefix`;
- a deletion is represented by an empty `replacement_text`;
- no Unicode normalization is performed;
- CRLF, LF, tabs, and all other bytes are preserved exactly;
- if `old == new`, no operation is emitted.

This selection is deterministic, local, and easy to test. It is not a full diff algorithm. Widening may
leave shared boundary characters in both `old_span_text` and `replacement_text`; that is acceptable
because DC-12 prioritizes valid UTF-8 span records over maximal minimality.

### Span identity

Authoring computes the existing `EditText` fields using only shared `text_span` primitives:

- `left_anchor_hash = left_anchor(old, start)`;
- `right_anchor_hash = right_anchor(old, end)`;
- `old_span_hash = text_span_hash(old_span_text)`;
- `span_id = compute_span_id(node_id, old_span_hash, left_anchor_hash, right_anchor_hash, dup_index)`;
- `old_span_text` and `replacement_text` are stored verbatim.

`dup_index` is the zero-based index of the chosen occurrence inside the anchor-filtered occurrence list
defined by `locate_text_span`. Authoring must not compute a path-local or diff-local alternative index.

For empty `old_span_text`, occurrences are the insertion positions `0..=old.len()`, matching the
existing shared primitive. This makes insertions first-class arbitrary-span edits rather than a special
wire shape.

### Replay and materialization

Replay support changes from "full-file exact span only" to the general shared localization path:

1. resolve `node_id` against the live node lifecycle state;
2. require the live node kind to be `TextFile`;
3. read the current text blob bytes and require well-formed UTF-8;
4. decode and validate `old_span_text` and `replacement_text` as well-formed UTF-8;
5. require `old_span_hash == text_span_hash(old_span_text)`;
6. locate the span with `locate_text_span`;
7. splice with `splice_text`;
8. compute the resulting text blob id with `text_blob_id`;
9. update the live node's blob id without changing node id, path, mode, or kind.

Failures are replay failures, not best-effort patch application. A stale anchor, changed context,
ambiguous location, wrong node kind, missing blob, invalid UTF-8, or mismatched hash fails closed and
reports the `node_id`, `span_id`, and reason where available.

### Inverse and rollback surfaces

An arbitrary-span forward `EditText` has a direct inverse:

- inverse `old_span_text = forward.replacement_text`;
- inverse `replacement_text = forward.old_span_text`;
- inverse anchors and `span_id` are computed against the post-forward text at the replacement span.

DC-12 may extend inverse planning, rollback preview, rollback draft append, and rollback draft
verification to this direct inverse only if direct-inverse round-trip vectors land in the same cut.
Otherwise, inverse/rollback extension must split to a fast-follow. It must not claim rollback
authorization, rollback refs, worktree rollback writes, or commutation.

### CLI behavior

No new CLI flag is added. The current CLI still accepts the compatibility flag:

```text
prikk commit --from-worktree --text-edits -m "message"
```

In the current node-kind-driven authoring path, this flag is retained as a no-op: existing `TextFile`
nodes author `EditText`, and existing `BinaryFile` nodes author `ReplaceBinary`. DC-12 is a better
span-selection/application implementation for the existing `EditText` path, not a new user mode.

`presentation_hint_line` and `presentation_hint_column` remain absent (`None`) in v0.5.0. They have no
current UI consumer and are not replay authority.

### Golden vectors

DC-12 must add vectors for:

- replacement in the middle of a file;
- insertion with empty `old_span_text`;
- deletion with empty `replacement_text`;
- sub-character byte-boundary edits that widen and still author/replay, including `é` -> `è` and a CJK
  example;
- repeated text where anchors and `dup_index` select the intended occurrence;
- prefix and suffix overlap boundaries;
- CRLF-preserving edits;
- multi-hunk edit represented as one enclosing span;
- replay splice output and resulting `text_blob_id`;
- negatives for anchor mismatch, no matching span id, hash mismatch, invalid UTF-8, wrong node kind,
  and ambiguous defensive localization.

Vectors must pin the byte-level identity material, not just round-trip through Rust structures.

## Rejected alternatives

### Keep whole-file `EditText` for v0.5.0

Rejected. Whole-file text edits are useful scaffolding, but they do not exercise the span-localization
surface needed for M2 patch reasoning.

### Implement a full diff algorithm now

Rejected. Multi-hunk minimization interacts with operation ordering, commutation, and conflict
witnesses. A single smallest enclosing span provides deterministic local edits without expanding the
algebra claim.

### Store byte offsets as authoritative preconditions

Rejected. Byte offsets are presentation hints at most. The authoritative location is the
content-anchored span identity.

### Add new `EditText` fields

Rejected. FDD-03 already has the fields needed for arbitrary spans. Adding wire fields would create
identity churn without solving the replay problem.

## Compatibility and identity rules

- Existing full-file `EditText` records remain valid arbitrary-span records whose span covers the
  entire old text.
- The canonical bytes for `EditText`, `Operation`, and `PatchPayload` do not change.
- New worktree-authored text edits receive new Patch object IDs because the `EditText` payloads become
  smaller spans rather than whole-file spans.
- `presentation_hint_line` and `presentation_hint_column` remain optional non-authoritative hints.
  Replay must not trust them, and DC-12-authored records leave them absent.
- Repositories sealed before DC-12 remain readable under the existing compatibility rules; DC-12 does
  not migrate stored Patch objects.

## Implementation plan

1. Review this RFC and companion FDD updates before implementation.
2. Add a shared span-selection helper that returns `(old_start, old_end, new_start, new_end)` by first
   finding the smallest enclosing byte span and then widening to UTF-8 character boundaries.
3. Add authoring tests and vectors for the span-selection helper.
4. Change worktree text authoring to emit the selected arbitrary span through shared text-span identity
   primitives.
5. Change patch replay/materialization to apply arbitrary `EditText` through `locate_text_span` and
   `splice_text`.
6. Split arbitrary-span inverse planning and rollback preview/draft verification to a follow-up because
   the required direct-inverse round-trip vectors did not land in this cut.
7. Update docs, README, status, and changelog without claiming commutation or conflict handling.
8. Cut v0.5.0 only after local gates and release checks pass.

## Test gates

Required tests:

- span selection replacement, insertion, deletion, whole-file replacement, prefix/suffix overlap,
  sub-character byte-boundary widening, CRLF, repeated text, and multi-hunk enclosing span;
- authored `EditText` fields match pinned vectors for anchors, `span_id`, `old_span_hash`,
  `old_span_text`, and `replacement_text`;
- replay applies arbitrary-span replacement, insertion, and deletion to the expected final text blob id;
- replay rejects stale anchors, changed text, mismatched `old_span_hash`, wrong node kind, missing blobs,
  invalid UTF-8, and malformed records;
- authoring and replay share the same span identity helpers; no authoring-local span-id logic;
- inverse planning fails closed on arbitrary-span `EditText` until direct-inverse round-trip vectors
  land;
- rollback preview/draft verification inherit that fail-closed inverse boundary;
- old full-file `EditText` fixtures still replay.

Standard gates:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Out of scope

- multi-operation text diff minimization;
- rename inference;
- symlink authoring/application;
- commutation, confluence, and conflict witnesses;
- rollback refs, rollback authorization, and worktree rollback mutation;
- semantic or language-aware merge;
- key lifecycle, audit plugins, and sync.

## Open questions before implementation

None after architect-review rulings:

- inverse/rollback extension was split because direct-inverse round-trip vectors did not land in the
  same cut;
- presentation hints stay absent (`None`) in v0.5.0.
