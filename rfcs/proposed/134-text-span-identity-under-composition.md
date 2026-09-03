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

## 3. CORRECTED 2026-09-04 — it is NOT user-reachable, and the first ruling was wrong

**This section originally ruled the defect reachable through `merge`. That ruling was wrong, and it
reached a released changelog and the architecture reference before it was caught.** Both are corrected;
the correction is recorded here rather than overwritten, because how the error was made is the useful
part.

**The error.** Reachability was asserted from a structural reading — `candidate_sequence`
(`merge_evidence.rs:345-359`) concatenates operations across every block between baseline and target,
so a merge side spans commits — **without checking the authoring side that decides whether the failing
*shape* can arise at all.** That is the same defect this RFC documents in §2's control: reasoning about
one mechanism while the question lived in another.

**What is actually true.** `current_text_for_node` (`node_authoring.rs:877-899`) resolves a node's
text through the queued-patch text cache, then the stored blob, then replay — so **every `EditText` is
authored against the state its predecessors produced**, never against a shared baseline. A second
commit editing the surviving duplicate sees one candidate and records `dup_index = 0`; replaying the
chain reproduces exactly that state, and the index matches.

**Verified empirically, not by reading.** A repository was built with two identical `f` spans separated
by uniform 96-byte filler (so both anchors match), then committed, then each span edited in its own
commit:

```
init -> 0 | commit(genesis) -> 0 | commit(edit word1) -> 0 | commit(edit word2) -> 0
verify -> 0 ; final text exactly filler+"a"+filler+"b"+filler
```

**`merge` composes sequentially-authored operations, so it cannot construct the failing sequence.** The
property test's generator builds both of one side's operations against the *same* pristine baseline —
a shape this codebase's authoring never produces.

## 3a. What the finding actually is

**Not a live defect. A latent fragility resting on an unstated invariant**, plus a diagnosis gap:

- **The invariant**: each `EditText` is authored against the state its predecessors produced. Every
  authoring path upholds it. **Nothing states it and nothing checks it.**
- **The consequence if it is ever violated** — a crafted patch, an externally-produced sequence, or a
  future authoring path that batches edits against one baseline — is a refusal reported as
  `EvidenceError::Malformed`, i.e. correct behaviour with a misleading diagnosis.
- **`ensure_flat_sequence` validating against the pristine baseline remains a real gap**; it simply
  is not reachable by our own authoring today.

**Severity is therefore lower than §1 first implied, and the property test remains valuable**: it found
an undocumented invariant that the type system does not enforce and no test asserted.



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

## 5a. Ruled 2026-09-04 — shape 2 is unsound, and a fourth shape exists

**Shape 2 (tolerant lookup) is refused. It cannot work, and the reason is visible in one loop**
(`text_span.rs:183-195`):

```rust
for (dup_index, &(start, end)) in anchor_matching.iter().enumerate() {
    let sid = compute_span_id(node_id, old_span_hash, record_left, record_right, dup_index as u32);
    if sid == *record_span_id { matches.push((start, end)); }
}
```

**Every input to `compute_span_id` except `dup_index` is taken from the record and is constant across
the loop. The candidate's own `(start, end)` never enters the hash.** And every candidate is, by
construction, identical in the remaining inputs — `occurrences` matched the same `old_span_text`, and
the filter kept only entries whose anchors equal the record's.

**Therefore `sid` is a pure function of `dup_index`, not of position.** A span id does not identify a
span; it identifies *an index into a candidate list*, and the list is rebuilt from the current buffer.

**So widening the search over `dup_index` recovers which index the record used and learns nothing
about which surviving candidate that was** — all survivors produce the same id for the same index. In
the persisted case one candidate survives and a tolerant lookup could only guess; with three originals
and two survivors it would guess between them. **A wrong guess silently edits the wrong span, which is
strictly worse than today's refusal.** Refused.

**Noted while proving it: the `Ambiguous` arm at `:200` is unreachable.** Distinct indices give
distinct ids, so `matches` holds zero or one element. A defensive arm that cannot fire is the shape
this project refuses elsewhere; it should be either proved reachable or replaced by a statement of why
it is not.

### Shape 4 — resolve against the baseline, track offsets through the sequence

**Identity-preserving, no format break, and it addresses the cause rather than the report.**

The recorded `dup_index` is valid against **the buffer the operation was authored against**. It stops
being valid only because sequence replay resolves each operation against the *running* buffer. So
resolve against the baseline text — where the candidate set is the one the id was computed from — and
carry the resulting byte range forward through the edits already applied in the same sequence.

**Replay has what this needs**: `replay_operations` starts from a baseline `OracleState`, whose
`texts: BTreeMap<NodeId, Vec<u8>>` holds the original content.

**Unproven, and it is design work, not a patch.** Offset tracking has real edge cases — an earlier edit
landing inside a later span, overlapping ranges, an edit that changes the candidate set the baseline
resolution depended on. **This RFC records shape 4 as the first candidate to design against, not as a
decision.**

**Shape 1 remains available as containment** — converting a malformed-evidence error into an honest
verdict — but it is not a fix: the sequence still fails to replay.

**Shape 3 stays the fallback**, and stays the owner's, if shape 4 does not survive design.

## 6. Scope

**In:** the mechanism (§1), the invisibility (§2), the reachability path (§3), the evidence table (§4),
and the three shapes (§5).

**Out:** any change to `commutation.rs`'s allowlist entry, which is the correct interim and keeps the
finding named in the property sweep's own output.

**Updated 2026-09-04:** shape 2 is refused (§5a) and **shape 4 is the candidate to design against**.
Choosing between shape 4 and shape 1 is the architect's; shape 3 remains the owner's.

**One non-negotiable for whatever is chosen: the persisted seed must go on failing until it passes for
the right reason.** The allowlist entry names this RFC; removing it without the underlying replay
succeeding would convert a recorded finding into a hidden one.
