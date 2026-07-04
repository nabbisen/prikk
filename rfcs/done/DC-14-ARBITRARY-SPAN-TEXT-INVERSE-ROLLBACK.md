# RFC (done) - DC-14 Arbitrary-Span Text Direct Inverse and Rollback Exposure

**Status.** Implemented (v0.7.0)
**Target release.** v0.7.0.
**Tracks.** Extending inverse planning, rollback preview, rollback draft append, and rollback draft
verification to deterministic arbitrary-span `EditText` records.
**Touches.** `patch_inverse`, rollback preview/draft flows, text-span inverse helpers, CLI output/docs,
and round-trip vectors.
**Companion FDD updates.** `../handoffs/DC-14-arbitrary-span-text-inverse-rollback/fdd-01-update.md`,
`../handoffs/DC-14-arbitrary-span-text-inverse-rollback/fdd-03-update.md`,
`../handoffs/DC-14-arbitrary-span-text-inverse-rollback/fdd-04-update.md`.

## Context

DC-12 made worktree text edits deterministic arbitrary spans and shared their replay identity through
the `text_span` primitives. It deliberately split arbitrary-span inverse/rollback because the direct
inverse vector set had not landed. v0.6.0 then broadened genesis to explicit non-default branch refs
without changing patch algebra.

The current inverse planner still fails closed on `EditText`:

```text
inverse planning for arbitrary-span EditText is deferred until direct-inverse vectors land
```

DC-14 closes that specific gap. It does not start the wider M2+ algebra program.

## Design goals

1. Derive deterministic direct inverse operations for supported arbitrary-span `EditText`.
2. Recompute inverse anchors, duplicate index, and `span_id` against the post-forward text; never reuse
   forward anchors as authority.
3. Preserve existing fail-closed behavior for unsupported node operations, unsupported node states, and
   malformed text records.
4. Expose the new inverse capability through existing read-only rollback surfaces without adding new
   CLI flags.
5. Preserve the DC-10 rollback-draft marker replacement: rollback drafts are distinguished by
   `PatchPurpose::RollbackDraft` and carry real role-bound Ed25519 AUTHOR signatures.
6. Pin the behavior with byte-level round-trip vectors before implementation is accepted.
7. Avoid claims about commutation, confluence, conflict witnesses, rollback refs, rollback
   authorization, worktree rollback mutation, or semantic merge.

## Proposed design

### Scope

DC-14 supports direct inverse for an `EditText` operation when all existing replay preconditions hold:

- the operation targets a live `TextFile` node by `node_id`;
- the current pre-forward bytes, `old_span_text`, and `replacement_text` are well-formed UTF-8;
- `old_span_hash == text_span_hash(old_span_text)`;
- the forward span localizes exactly once through the existing anchor-filtered `span_id` rule;
- the forward splice succeeds through the shared `splice_text` primitive.

The inverse planner already replays the sealed single-parent chain from root to target and accumulates
inverse operations, then reverses them into rollback application order. DC-14 keeps that shape. The
new `EditText` inverse is derived while applying each forward edit to the planner's in-memory state,
so both pre-forward and post-forward bytes are available without a second replay implementation.

Out of scope:

- rollback-specific refs;
- rollback authorization policy;
- mutating the worktree as rollback;
- commutation, confluence, conflict witnesses, or semantic merge;
- multi-operation text diff minimization;
- branch copy/fork, branch switching, or merge-base semantics;
- text-to-binary or binary-to-text transitions beyond existing fallback behavior.

### Direct inverse rule

For one forward `EditText` over a text node:

1. Validate the forward operation using the same object and runtime checks used by supported replay.
2. Localize the forward span in the pre-forward bytes, yielding `[start, end)`.
3. Apply the forward splice:

   ```text
   post = pre[..start] || forward.replacement_text || pre[end..]
   ```

4. Identify the inverse old span in `post` as the exact replacement range:

   ```text
   inverse_start = start
   inverse_end = start + byte_len(forward.replacement_text)
   inverse_old_span_text = forward.replacement_text
   inverse_replacement_text = forward.old_span_text
   ```

   `inverse_start` and `inverse_end` must be valid UTF-8 byte boundaries in `post`. Empty spans are
   represented only at valid insertion-position byte boundaries.

5. Compute inverse identity from `post`, not from `pre`:

   - `old_span_hash = text_span_hash(inverse_old_span_text)`;
   - `left_anchor_hash = left_anchor(post, inverse_start)`;
   - `right_anchor_hash = right_anchor(post, inverse_end)`;
   - `dup_index` is the zero-based index of `[inverse_start, inverse_end)` among canonical-order
     occurrences of `inverse_old_span_text` whose left and right anchors match those inverse anchors;
   - `span_id = compute_span_id(node_id, old_span_hash, left_anchor_hash, right_anchor_hash,
     dup_index)`.

6. Build an inverse `EditText` with the same `node_id`, absent presentation hints, and the computed
   inverse fields above.
7. Re-localize the derived inverse with `locate_text_span(post, ...)` and require the located range to
   equal exactly `[inverse_start, inverse_end)`.
8. Apply the derived inverse splice and require the result to be byte-identical to `pre`.

This handles replacement, insertion, and deletion uniformly:

- a forward insertion has empty `old_span_text`, so the inverse deletes the inserted replacement text;
- a forward deletion has empty `replacement_text`, so the inverse inserts the deleted old span at the
  zero-length post-forward position;
- a replacement swaps the two byte spans.

The inverse must itself be replay-valid against `post`. As an implementation gate, tests must apply
forward then inverse and recover the exact original bytes, and apply inverse then forward from the
post-forward bytes to recover the exact post-forward bytes.

### Helper ownership

DC-14 should keep all identity-bearing text-span inverse derivation in the shared text-span module or a
nearby single-purpose helper, not in CLI or rollback code. The helper should return either a complete
inverse `EditText` or a precise fail-closed error. It must use the existing `left_anchor`,
`right_anchor`, `text_span_hash`, `compute_span_id`, `occurrences`, `locate_text_span`, and
`splice_text` primitives.

If helper visibility needs to change, expose the smallest crate-local API required by
`patch_inverse`. Do not duplicate anchor, duplicate-index, or `span_id` computation.

### Planner state and operation ordering

The inverse planner must continue to walk history in forward order and update the in-memory file map as
each supported operation is applied. For `EditText`, it must update the file bytes to `post` after
deriving the inverse operation.

The accumulated inverse operations remain reversed at the end of planning and then renumbered from
1..N. This is important for patches containing multiple operations against the same text node: rollback
application order is the reverse of forward application order.

The implementation gate must include an ordering-sensitive vector with two `EditText` operations
against the same `node_id`, where the second forward edit depends on the first edit's post-text.
Inverse planning must emit the inverse edits in reverse order; applying them in forward order must fail
or fail to restore the original bytes.

### CLI behavior

No new commands or flags are introduced.

Existing commands extend their supported subset:

- `prikk inverse-plan [path] [--ref REF]` includes `edit-text` inverse operation summaries for
  supported arbitrary-span text edits.
- `prikk rollback-preview [path] [--ref REF]` validates the derived inverse plan and reports the
  affected file through the existing file-level preview vocabulary.
- `prikk rollback-draft --append-inverse [path] [--ref REF] -m <message>` can append an AUTHOR-signed
  rollback draft whose payload contains inverse `EditText` operations.
- `prikk rollback-draft-verify [path] [--ref REF]` recomputes the inverse plan and byte-compares it
  with the active rollback draft payload, including all recomputed `EditText` identity fields.

The commands still fail closed when the selected history includes any unsupported operation or
unsupported lifecycle state.

### Verification and rollback draft identity

DC-14 does not change Patch, Block, RefState, RefUpdate, or signature identity. It does explicitly
preserve the DC-10 rollback-draft marker replacement:

- rollback-draft identity is determined by `PatchPurpose::RollbackDraft` plus rollback-draft
  verification rules, not by a placeholder AUTHOR key id or any other marker key;
- rollback-draft Patch envelopes carry real role-bound Ed25519 AUTHOR signatures through the AUTHOR
  signing path introduced in DC-10;
- the signature bytes are real, but DC-14 still does not add AUTHOR trust-store enforcement,
  rollback authorization policy, rollback refs, or a publication policy for rollback;
- `rollback-draft-verify` must reject a normal-purpose Patch even if its operations are byte-identical
  to the derived inverse;
- `rollback-draft-verify` must reject a rollback-draft envelope that lacks a real AUTHOR signature or
  fails the existing rollback-draft signature checks.

DC-14's AUTHOR-signature verification boundary is intentionally narrower than trust or authorization.
Without an AUTHOR trust store or a supplied AUTHOR public-key source, rollback-draft verification must
reject a missing signature, wrong role, wrong algorithm, malformed signature record, placeholder/marker
signature, and payload purpose mismatch. Release notes must not imply policy verification or full
cryptographic trust validation of arbitrary historical AUTHOR signatures unless a public-key authority
is introduced by a separate design.

The exact inverse `PatchPayload` canonical bytes are part of rollback-draft verification. Verification
must recompute the complete inverse payload, reverse and renumber operations exactly as the planner
emits them, require generated inverse presentation hints to be absent, and compare canonical payload
identity bytes rather than summaries or semantic replacement equivalence. A draft that swaps the texts
but carries stale anchors, a stale `span_id`, mismatched presentation hints, or any other non-derived
byte must fail verification.

### Diagnostics

Diagnostics should distinguish these cases where practical:

- forward span could not be localized in planner state;
- inverse replacement range could not be represented as UTF-8 boundaries;
- inverse duplicate index could not be found;
- the derived inverse failed replay validation against post-forward bytes;
- a non-text or missing live node was targeted;
- the history still contains an unsupported operation outside the DC-14 subset.

The diagnostic should point at inverse/rollback support, not at repository corruption, when the issue
is an intentionally unsupported operation.

## Implementation outline

1. Add byte-level direct-inverse helper vectors for replacement, insertion, deletion, repeated text,
   CRLF, sub-character widened edits, and multi-hunk enclosing spans.
2. Add a crate-local helper that derives an inverse `EditText` from `(node_id, pre_text,
   forward_edit)` by replay-localizing forward and recomputing inverse identity over `post_text`.
3. Wire `patch_inverse::derive_inverse_operation` to handle `DecodedOperationKind::EditText` using the
   helper and update the planner file map.
4. Extend inverse summaries so `EditText` is reachable rather than treated as unreachable.
5. Extend rollback preview/draft append/verify tests with text-edit histories.
6. Keep unsupported operation tests intact so DC-14 cannot accidentally broaden the patch subset.
7. Add rollback-draft verification negatives for normal-purpose patches, missing, wrong-role,
   wrong-algorithm, malformed, marker/placeholder, and purpose-mismatched AUTHOR signature records,
   stale inverse anchors, stale inverse `span_id`, and non-absent inverse presentation
   hints.
8. Update README, roadmap, implementation status, changelog, and command help if output wording
   changes.

## Required test vectors

DC-14 acceptance requires deterministic vectors for:

- replacement: `alpha beta gamma` -> `alpha BETA gamma`;
- insertion: insert text at a zero-length span;
- deletion: delete a non-empty span;
- repeated occurrence disambiguation through anchors and duplicate index;
- CRLF-preserving edit;
- UTF-8 sub-character widening inherited from DC-12;
- multi-hunk enclosing span inherited from DC-12;
- a hard-gated ordering vector with two edits against the same `node_id`, where the second forward
  edit depends on the first edit's post-text and reverse rollback order is required;
- forward then inverse returns exact original bytes;
- inverse then forward from post-forward bytes returns exact post-forward bytes;
- rollback-draft verify rejects stale inverse anchors or stale `span_id`;
- rollback-draft verify rejects a normal-purpose Patch, missing, wrong-role, wrong-algorithm,
  malformed, marker/placeholder, and purpose-mismatched AUTHOR signature records, and generated inverse
  presentation hints;
- a history with supported `EditText` plus any unsupported operation still fails closed for
  `inverse-plan`, `rollback-preview`, and rollback draft append rather than skipping the unsupported
  operation;
- negatives for stale post text, wrong node kind, hash mismatch, invalid UTF-8, and ambiguous or
  unresolvable localization.

## Implementation errata

Design re-review v1 accepted DC-14 with the following implementation errata as acceptance criteria:

1. Remove or demote production marker-key authority everywhere. `PatchPurpose::RollbackDraft` is the
   rollback-draft discriminator; reserved AUTHOR key ids, marker signatures, or hash markers are not
   valid DC-14 rollback-draft authority.
2. Define and test the real AUTHOR-signature boundary without overstating trust. DC-14 rejects missing,
   wrong-role, wrong-algorithm, malformed, marker/placeholder, and purpose-mismatched AUTHOR signature
   records. It does not add AUTHOR trust-store enforcement or rollback authorization.
3. Add the FDD-04 / release security-note delta before v0.7.0 finalization.
4. Keep inverse identity single-sourced in the shared text-span / patch-domain layer and reuse replay's
   primitives for anchors, occurrence enumeration, `span_id`, localization, and splice.
5. Enforce exact inverse-range re-localization against post-forward text.
6. Compare canonical `PatchPayload` bytes in rollback-draft verification, with inverse presentation
   hints absent.
7. Preserve fail-closed unsupported-operation boundaries across inverse-plan, rollback-preview,
   rollback-draft append, and rollback-draft verify.

## Compatibility

No object schema migration is required. Existing DC-12 `EditText` records stay valid. Histories that
currently fail closed during inverse planning because they contain arbitrary-span `EditText` become
supported only when the direct inverse can be derived deterministically under this RFC's rules.

## Open questions

None blocking. Presentation hints should remain absent in generated inverse operations until a UI
design gives them meaning; they are not algebraic preconditions and should not affect rollback draft
identity.

## Review resolution

Design review v1 required the rollback-draft signing/marker scope to be explicit before
implementation. DC-14 chooses Option A from that review: rollback-draft marker replacement is in scope
as an inherited DC-10 contract. `PatchPurpose::RollbackDraft` is the rollback-draft discriminator,
rollback-draft envelopes are real role-bound Ed25519 AUTHOR-signed Patch envelopes, and signed rollback
drafts still do not imply AUTHOR trust-store enforcement or rollback authorization.

Design re-review v1 accepted this design for implementation with the errata above.

## Rejected alternatives

### Reuse forward anchors for inverse

Rejected. The inverse operation applies to the post-forward text. Forward anchors describe the
pre-forward occurrence and are not authoritative after the splice.

### Store byte offsets in rollback drafts

Rejected. Offsets are not part of `EditText` authority and would introduce a second replay model.

### Materialize rollback into the worktree

Rejected for DC-14. The existing rollback surfaces are read-only preview and active rollback draft
authoring. Worktree mutation requires a separate authorization and UX design.

### Treat DC-14 as full text patch algebra

Rejected. Direct inverse is necessary for rollback, but it does not define commutation or conflict
witnesses for overlapping text spans.

### Keep rollback draft marker-key authority

Rejected for DC-14. The repository's accepted DC-10 direction is that rollback-draft identity is
`PatchPurpose::RollbackDraft`, not a reserved AUTHOR key id. Keeping marker-key authority while
exposing richer rollback draft payloads would preserve an obsolete internal scaffold and make
rollback-draft verification ambiguous.
