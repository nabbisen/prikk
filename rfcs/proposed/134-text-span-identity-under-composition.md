# RFC 134 — A text span's identity is not stable under composition

**Status.** **ACCEPTED by the project owner 2026-09-04** — the analysis, not a fix. **§5's three
shapes remain unruled**, which is what this RFC deliberately left open.

**Who rules them.** Shapes 1 and 2 are identity-preserving implementation design and are the
architect's. **Shape 3 changes `span_id`'s definition and is therefore a format break under RFC 114's
stability contract — that one is the owner's**, and would arrive as an escalation rather than a
handoff.

Found 2026-09-03 by RFC 126 §2's patch-algebra property tests — the
increment whose own subject was that the property the RFC originally named could not fail. Reachability
and mechanism established by the architect at `e8f55ff`/`dd3fc1e`; **shipped in `0.30.0`'s known-costs
table rather than held**, because the defect predates every release and refuses rather than corrupts.

**Tracks.** The identity function for text spans, and the merge path that meets its limit. **No fix is
proposed here** — §5 explains why the obvious one is not available.

---

## 1. The defect

`compute_span_id` (`crates/prikk-store/src/text_span.rs:124-137`) folds a **duplicate-occurrence
index** into a span's identity:

```rust
preimage.extend_from_slice(b"PRIKK-TEXT-SPAN-v1");
preimage.extend_from_slice(node_id.as_bytes());
preimage.extend_from_slice(old_span_hash);
preimage.extend_from_slice(left);
preimage.extend_from_slice(right);
preimage.extend_from_slice(&dup_index.to_be_bytes());
```

and `:184` derives that index from `enumerate()` over `anchor_matching` — **a list rescanned from the
buffer as it exists at lookup time**:

```rust
for (dup_index, &(start, end)) in anchor_matching.iter().enumerate() {
```

**So a span's identity depends on how many textually-and-anchor-identical occurrences the buffer holds
at the moment of lookup.** Editing one of them removes it from `anchor_matching` and renumbers every
later one, invalidating their recorded ids.

**Concretely, from the persisted failing case** (`proptest-regressions/patch_algebra/tests/algebra_properties.txt`):
two words of identical text (`"f"`), each inside otherwise-uniform filler so their anchor hashes match
too. Both are authored against the pristine baseline, taking `dup_index` 0 and 1. On replay the first
edit lands, the second rescans, finds **one** candidate instead of two, recomputes with
`dup_index = 0`, and fails to match its recorded id — `Err(NoMatchingSpanId)`.

## 2. Why this was invisible

**`ensure_flat_sequence` validates each operation individually against the pristine baseline**, so it
cannot see that a sequence's own operations break each other. The failure surfaces later, inside
`replay_sequence_order`, which converts it to `EvidenceError::Malformed` with the reason
*"composed replay failed after confluence proof"* — **a string whose own wording shows the author
believed the state unreachable.**

**And the control that blessed this operation pair validated a different mechanism.**
`oracle_verdict_finds_a_genuinely_separated_same_node_edit_pair_to_be_deliberate_not_a_bug` used
**distinct** old-word values and checked two *independent* fresh-from-baseline replays — never the
chained `replay_sequence_order` that `check_confluence` actually uses for one side's own sequence, and
never the identical-value case. **A passing control over the wrong path.**

## 3. It is user-reachable, through merge

- **One commit cannot produce it.** `plan_edit_text` (`node_authoring.rs:625-665`) returns a single
  `PlannedOp` per file from one whole-text diff. No commit emits two `EditText` operations for one node.
- **A merge side can.** `candidate_sequence` (`merge_evidence.rs:345-359`) iterates **every block
  between baseline and target** and concatenates every patch's operations into one sequence. **A branch
  of two commits — one editing each of two anchor-identical spans in the same file — is exactly the
  failing shape.** `prepare_merge_evidence` and `execute_merge` share that path.

**Severity, stated precisely: it refuses rather than accepts.** A user with a legitimate branch is told
the candidate is malformed instead of receiving a verdict. **No corruption, no history at risk.**
Reaching it requires two spans identical in both replaced text and surrounding anchor context, which in
real content means substantial repetition — repeated stanzas, generated tables, long uniform runs.

## 4. What holds today, and what does not

| Property | Held by |
|---|---|
| A single commit's text edit replays | Real use, and the existing suite |
| Two independent same-node edits replay in either order from a shared baseline | The control named in §2 |
| **A sequence's own chained operations replay** | **Nothing.** `ensure_flat_sequence` checks against the pristine baseline only |
| **`merge` returns a verdict rather than a refusal for such a branch** | **Nothing** |

## 5. Why the obvious fix is not available

**Redefining `compute_span_id` to drop `dup_index` is a format break.** `span_id` is **field 2 of the
canonical `EditText` encoding** (`prikk-object/src/payload/patch/operations.rs:171,221`) — it is inside
signed, sealed Patch objects in every existing repository, and it is pinned in the frozen FDD identity
vectors (`prikk-object/src/vectors/hard.rs`). `locate_text_span` matches a **recorded** id against
recomputed candidates, so changing the function makes every existing recorded id fail to match.

**That puts the naive fix under RFC 114's format-stability contract**, and it is why this is an RFC
rather than a patch.

**Three shapes exist and none is ruled here:**

1. **Fix the classification, not the identity.** Make sequence validation chain-aware so this becomes
   a proper conflict or refusal verdict instead of malformed evidence. Identity-preserving; no format
   change; **leaves the underlying replay failure in place** and only reports it honestly.
2. **Make lookup tolerant.** Have `locate_text_span` try the recorded id against plausible
   duplicate indices rather than only the freshly-rescanned position. Identity-preserving and may make
   these sequences replay — but the search space and its ambiguity need design.
3. **Change span identity** to something stable under composition. Correct at the root, and a
   format-breaking change requiring migration or a schema bump.

## 6. Scope

**In:** the mechanism (§1), the invisibility (§2), the reachability path (§3), the evidence table (§4),
and the three shapes (§5).

**Out:** choosing among them. **Out:** any change to `commutation.rs`'s allowlist entry, which is the
correct interim and keeps the finding named in the property sweep's own output.

**One non-negotiable for whatever is chosen: the persisted seed must go on failing until it passes for
the right reason.** The allowlist entry names this RFC; removing it without the underlying replay
succeeding would convert a recorded finding into a hidden one.
