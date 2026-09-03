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

## 7. Ruled 2026-09-04 — containment now, stable identity as the answer

**The owner's challenge was the right axis**: is shape 1 strongly safe *in future*, not only now. It is
not, and the architect's earlier recommendation of it as the answer was wrong. Recorded with the
reasoning rather than replaced.

### 7.1 Why containment alone is not safe forward

**The invariant is an accident of current authoring, not a law.** It holds because `plan_edit_text`
emits one operation per file and `current_text_for_node` resolves through the queued-patch cache.
**Two directions already on this project's board would break it:**

- **RFC 113 — history import from Git, Subversion, CVS.** Converting foreign history constructs
  operations programmatically, and the natural implementation authors several edits to one file
  against a single baseline state. **That is precisely the failing shape.**
- **Any per-hunk or interactive commit mode**, whose natural implementation does the same.

**And the cost of waiting rises monotonically.** `span_id` is inside signed, sealed history. Every
commit adds more history under the positional scheme, so the only variable under our control is how
much exists when it changes. **Containment alone defers the work to the moment it blocks a feature** —
under schedule pressure, at maximum accrued cost.

### 7.2 Correcting this RFC's own framing: shape 3 is not a format break

**§5 called shape 3 "a format-breaking change requiring migration". That was wrong**, and it made the
option look more expensive than it is.

RFC 114's contract is explicit:

> **Any prikk release can read every object any prior release wrote, and verifies it to the same
> conclusion. Storage may require a migration step, which is documented and tested. Object identity
> and signatures never require one.**

and

> **Freezing is not "never add a field".** `schema_version` is *inside* the id preimage, so a new field
> means a new schema version, new ids for new objects, and **no change whatsoever to objects already
> written.**

**So a v2 span identity is an additive schema version, not a break** — the pattern `DC-75` already used
for `Block` 1 → 2, and which `RefState` carries two of at once. **The design anticipated it**: the
preimages are already domain-separated and versioned (`PRIKK-TEXT-SPAN-v1`,
`PRIKK-TEXT-LEFT-ANCHOR-v1`, FDD-01 §5.1).

**Migration sketch: none is required for identity.** Existing `EditText` operations keep resolving
through the v1 path, unchanged, forever — they were authored under the invariant and are sound.
New operations are authored and resolved under v2. Nothing is rewritten, nothing is re-signed, and no
repository becomes unreadable.

**The failure to design against is recorded in RFC 114 itself**: DC-53 Stage 2's `PBNDL001` →
`PBNDL002` bump severed the bundle migration path and made every repository below format 6
unmigratable. **A v2 identity must not touch the transport or the repository-format gate**, and the
increment that lands it must demonstrate an old-format repository still round-trips.

### 7.3 The two candidate identities

**(a) Guarantee uniqueness at authoring.** Extend the anchor window until the span is unique in the
buffer and record the length used. **`dup_index` disappears.** If no extension can make it unique — a
wholly uniform file — **refuse at authoring time**, where the user can still act, rather than at replay
where they cannot. Content-based, stable under composition, small model change.

**(b) Spans as first-class identified entities**, minted like node ids and carried in node state.
Cleanest and most sophisticated; the largest change, since a node's state must then carry span identity
rather than only bytes.

**The architect's recommendation is (a)**, for the same reason RFC 126 §6 chose criterion: it removes
the fragile mechanism rather than managing it, and its failure mode is a refusal a user can act on
rather than a mismatch at replay. **§7.5 re-reviews (b) against the threat model and refuses it** —
the first pass called it "the better model and the worse trade", which was wrong.

**Both are the owner's to authorize**, because both add a schema version to signed history.
**(a) was authorized 2026-09-04; the design is §8.**

### 7.5 Re-reviewed 2026-09-04 against the data model, the lifecycle, and the threat model

**The owner asked for this judgement to be re-examined. It changed: (b) is not "the better model and
the worse trade" — it is a security regression, and (a) is stronger than the first pass argued.**

**Data model.** `NodeContent::File { blob_id, mode }` (`prikk-replay/src/node_lifecycle/types.rs:9`)
carries a blob reference and mode; **content has no sub-identity, and `compute_state_root`
(`state_root.rs:116`) hashes the resulting blob, not spans.** So span identity is a *resolution
mechanism*, not a verification-bearing value: **neither option changes what any state root means.**
That de-risks both, and it is why a v2 identity affects only new Patch ids.

**Threat model — the decisive input.** `text_span.rs:159` states a deliberate property:

> The stored `span_id` is recomputed, never trusted directly.

That is load-bearing, not incidental, because
`docs/src/reference/trust-threat-model.md` records that a received Patch's contents are
adversary-controlled in the relevant sense:

> an attacker who re-signs a Patch with their own key and ships that key in the bundle produces a
> bundle that verifies perfectly.

**So signature verification does not constrain an `EditText`'s fields.** The only thing stopping a
crafted operation from relocating an edit somewhere of the attacker's choosing is that resolution
**recomputes** the identity from the victim's actual buffer content and demands a match.

**(b) breaks exactly that.** A minted span id is by construction *not* derivable from content, so it
cannot be recomputed and compared; resolution would consult a mapping instead, and a crafted operation
could carry an id chosen to redirect the edit. **Elegant in the abstract, weaker against the adversary
this project actually models.** Refused on that ground, not on weight.

**(a) preserves the property.** Identity stays `H(node, old_span_hash, left_anchor, right_anchor)` —
fully content-derived, fully recomputable — with `dup_index` deleted and the anchor *lengths* recorded.
An attacker must still produce text and context matching the victim's buffer, exactly as today, and the
positional ambiguity disappears.

**And (a)'s feared failure case does not exist.** The first pass worried it would refuse a wholly
uniform file. **Uniqueness is always achievable for a finite file**: extending the left anchor
eventually reaches the file start, and distinct positions have distinct prefixes. **Anchors are
hashed**, so a longer anchor costs one recorded integer, not more bytes.

**(a)'s real cost, stated honestly:** disambiguating in highly repetitive content requires long anchors,
and a long anchor is sensitive to *any* nearby edit — so some legitimate composed sequences will be
refused that a positional scheme would have accepted. **That is the right trade**: the refusal is an
honest anchor mismatch meaning *"the context you authored against has changed"*, surfaced at authoring
or replay, and it can never silently resolve to the wrong span. **The current scheme's failure mode is
a mismatch nobody can explain; (a)'s is a sentence a user can act on.**

**Conclusion unchanged, reasoning replaced: (a).** And the recommendation is now stronger than
"preferred" — **(b) should not be adopted** while `span_id` remains adversary-supplied and
recompute-verified.

### 7.4 Disposition

1. **Shape 1 ships as containment** — report a violating sequence as operations that are mutually
   inconsistent, not as malformed evidence — **and the invariant is written down** in `text_span.rs`'s
   module doc, where `dup_index` lives. Cheap, immediate, no format involvement.
2. **Shape 4 is refused.** The oracle (`replay_oracle.rs:231`) and real materialization
   (`patch_replay/apply.rs:272`) call `locate_text_span` identically **by design** — that sameness is
   what makes the oracle's prediction sound. Shape 4 must land in both or neither; in both it changes
   materialization on existing history, trades a cryptographic context check for offset arithmetic, and
   makes the system *accept* a sequence whose refusal looks correct.
3. **Shape 3 proceeds as design**, options (a) and (b), recommendation (a).

## 8. Design — option (a), authorized by the project owner 2026-09-04

**Content-unique anchors: identity stays fully content-derived and recomputable, `dup_index` is
deleted, and uniqueness is guaranteed at authoring instead of disambiguated at replay.**

### 8.1 The schema

`admitted_schemas(ObjectType::Patch)` currently returns `&[1, PATCH_PARENT_IDS_RETIRED_SCHEMA]`, and
`PATCH_PARENT_IDS_RETIRED_SCHEMA = 2` (`prikk-object/src/payload/patch.rs:59`). **v2 identity mints
Patch schema 3**, admitted alongside both.

**`EditText` gains two optional fields**, tags **10** (`left_anchor_len`) and **11**
(`right_anchor_len`), `u32` each. **Optional and absent below schema 3**, exactly as tags 7 and 8
already are — so **no object already written changes by a single byte.**

### 8.2 The identity function

**v2**, domain-separated from v1 as FDD-01 §5.1's `-v1` suffixes already anticipated:

```
PRIKK-TEXT-SPAN-v2 ‖ node_id ‖ old_span_hash ‖ left_anchor ‖ right_anchor ‖ left_len ‖ right_len
```

**`dup_index` does not appear.** Anchors are `PRIKK-TEXT-LEFT-ANCHOR-v2` / `-RIGHT-ANCHOR-v2` over
exactly `left_anchor_len` bytes preceding the span and `right_anchor_len` following it, rather than the
fixed `TEXT_ANCHOR_WINDOW = 64`.

### 8.3 Authoring

Choose the **smallest** lengths, each at least the current 64, that make the span **unique** among
occurrences of `old_span_text` in the authoring buffer.

**This always succeeds for a finite file.** Extending the left anchor eventually reaches the file
start, and distinct positions have distinct prefixes. **Anchors are hashed**, so a long anchor costs
one recorded integer, not more bytes.

### 8.4 Resolution

**v2** — find occurrences of `old_span_text`; filter by the two anchors computed at the *recorded*
lengths; **require exactly one**; recompute the id and compare. Uniqueness is a property of the record,
not of a rescanned list, so **nothing renumbers under composition.**

**v1 — unchanged and frozen forever.** Operations below schema 3 keep resolving through the existing
`dup_index` path. They were authored under the invariant of §3a and are sound.

### 8.5 What must not change, and why

- **Every v1 object's bytes, id, and resolution.** RFC 114: *"keep every version ever written
  decodable, forever, and keep its bytes hashing the way they did on the day they were written."*
- **The repository-format gate and bundle transport.** RFC 114 records that DC-53 Stage 2's
  `PBNDL001` → `PBNDL002` bump severed the migration path and made every repository below format 6
  unmigratable. **A schema addition must not touch transport**, and the increment must *demonstrate* an
  older repository still round-trips rather than assume it.
- **`compute_state_root`'s inputs.** It hashes the resulting blob, not spans (§7.5), so what history
  *means* is untouched.

### 8.6 The known trade

Highly repetitive content needs long anchors, and a long anchor is sensitive to any nearby edit — so
some composed sequences that a positional scheme accepted will now be refused. **That is the intended
trade** (§7.5): the refusal is an honest anchor mismatch a user can act on, and it can never silently
resolve to the wrong span.

### 8.7 Sequencing

**One increment, not two.** A half-landed version scheme — the identity function without authoring, or
authoring without resolution — is worse than either end state.

**The Property B allowlist entry stays** until the property generator itself is moved to v2 and the
underlying sequence passes *for the right reason*. Removing it because v2 landed would convert a
recorded finding into a hidden one.

## 6. Scope

**In:** the mechanism (§1), the invisibility (§2), the reachability path (§3), the evidence table (§4),
and the three shapes (§5).

**Out:** any change to `commutation.rs`'s allowlist entry, which is the correct interim and keeps the
finding named in the property sweep's own output.

**Updated 2026-09-04 (§7):** shapes 2 and 4 are both refused; **shape 1 ships as containment and shape
3 proceeds as design**, with options (a) and (b) and a recommendation of (a). §5's characterisation of
shape 3 as a format break is **corrected in §7.2** — it is an additive schema version requiring no
migration for identity.

**One non-negotiable for whatever is chosen: the persisted seed must go on failing until it passes for
the right reason.** The allowlist entry names this RFC; removing it without the underlying replay
succeeding would convert a recorded finding into a hidden one.
