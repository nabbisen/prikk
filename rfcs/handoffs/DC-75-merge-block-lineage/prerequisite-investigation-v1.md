# DC-75 §4 — Prerequisite Investigation, With Measurements

**Handoff followed:** `implementation-handoff-v1.md` + `implementation-handoff-v1-addendum.md`
**Governing RFC:** `rfcs/done/DC-75-MERGE-BLOCK-LINEAGE.md`

Per §4 ("blocking prerequisites... answered with measurements, not preference") and criterion 1, this
reports before any design or production code. No source files are changed by this document.

## 0. The headline finding, first, because it changes how §3 should be read

**`prikk verify` already costs roughly O(N³) in the number of sealed format-2 blocks, independent of
DC-75 entirely.** Measured on this exact tree (`facdc72`, before any DC-75 work), single-branch
histories, `prikk verify` wall-clock time:

| N (sealed blocks) | verify time | ratio vs. previous |
|---:|---:|---:|
| 5 | 82 ms | — |
| 10 | 171 ms | 2.08× |
| 20 | 388 ms | 2.26× |
| 40 | 1,179 ms | 3.04× |
| 80 | 5,341 ms | 4.53× |
| 160 | 34,155 ms | 6.39× |

Linear growth would hold the ratio at 2.0× per doubling; quadratic would approach 4.0×; the ratio here
is climbing past 4× toward 8× (cubic) as N grows — consistent with the code, not just the numbers.

**Why:** `verify_objects` (`verify/objects.rs`) scans every persisted `Block` object independently and
calls `verify_block_v2_state` (`block_state.rs`) on each. That function's `derive_next_state_root` calls
`verify_v2_lineage_roots`, which — for **every ancestor** of the block being checked — calls
`replay_with_appended_patches`, which itself calls `walk_lineage_to_genesis` and replays from genesis
**every time**, with no memoization across either the outer per-block loop or the inner per-ancestor
loop. Verifying block at position *i* costs O(*i*²); summed over N blocks, O(N³).

**This is not DC-75's defect and I have not touched it.** It exists today, on `main`, for ordinary
single-parent history. I report it here rather than silently, because it is the dominant term in §3's
cost question: whatever DC-75 adds to this baseline, it adds on top of an already-severe curve, and a
design that makes it worse would be adding a multiplier to a problem already large enough that a
few-hundred-commit repository takes tens of seconds to verify. **Recommend a `MILESTONES.md` row for
this on its own, unowned, separate from DC-75** — it needs its own increment (memoize `walk_lineage_to_genesis`'s
result and reuse the accumulated state across the per-block loop, dropping the whole thing to O(N)), and
attaching that fix to DC-75 would be exactly the scope-widening the standing rule warns against.

## 1. §4.1 — answering §3 with the measurement, not preference

> When a block has two parents, what does "the state derived from this block" mean, and against which
> parent(s) is it cryptographically verified?

**Recommendation: mainline-authoritative.** Reasoning, in order of how much weight I give each:

**a. Cost, decisively, given §0.** Both-parents-verified means every block downstream of a merge, on
every full `verify`, would need `verify_v2_lineage_roots`-equivalent work run against **both** parent
chains, recursively through any earlier merges in either ancestry. Given the *single*-chain case is
already empirically cubic, doubling (or, for nested merges, multiplying) that per merge is not a
"more expensive but sound" option — it is very plausibly the difference between "seconds" and
"do not run this." I did not attempt to measure the multi-parent case directly, since no code
producing a `BlockKind::Merge` block exists yet to measure; the single-chain number is the necessary
component of that cost and is already large enough to decide this without needing the multi-parent
number too.

**b. The secondary parent's soundness is not actually left unverified — it is verified elsewhere, for
free.** `verify_objects` does not walk one ref's lineage; it scans **every persisted `Block` object in
the store**, whatever ref does or does not currently point at it. A merge's secondary parent's own
blocks are each independently reached and independently checked by that scan, exactly as if the merge
had never happened. Mainline-authoritative does not skip verifying the secondary chain's *structural*
soundness (state roots, signatures, object existence) — it skips **re-deriving, from the merge block
itself, that adopting those specific patches was confluence-sound**.

**c. That gap is not new, and not special to merges.** `verify` never re-derives that an ordinary
`EditText`/`ChangePerm`/etc. was semantically the "right" edit — it confirms structural and
cryptographic validity (signatures, state-root consistency) and **trusts the maintainer's signature**
for everything else. A merge block's confluence proof, under mainline-authoritative, is trusted the
same way: DC-74's `execute_merge` only ever seals when `patch_algebra` proved `Confluent`, the
maintainer's signature is on the resulting block, and a later verifier confirms that signature and the
referenced objects are real and internally consistent — the same trust boundary every other sealed
decision already sits behind. Both-parents-verified would be *stronger* than the project's existing
trust model for everything else it seals, at a cost §0 already shows is severe for the weaker version.

**d. It is the smaller change.** Under mainline-authoritative, `derive_next_state_root` for a merge
block is called exactly as it is today (`Some(mainline_parent)`, adopted `patch_ids`) — DC-74's
`execute_merge` state-derivation logic does not change at all. `patch_replay`/`patch_inverse`'s
`single_parent_chain` functions need only learn to continue through a `Merge`-kind block via its
mainline parent, ignoring the secondary parent for traversal — not walk two divergent chains and
reconcile them.

## 2. A format-level blocker this surfaced, not anticipated in either handoff

**Mainline-authoritative requires knowing *which* parent is mainline. `parent_block_ids`'s current
canonical encoding cannot represent that.**

`prikk-object/src/payload/block.rs:95,224-226`: both `decode_canonical` and `CanonicalEncode` reject
a `BlockPayload` whose `parent_block_ids` is not **strictly sorted** by `ObjectId` byte value. Sorting
by content-hash order is exactly the operation that destroys "position 0 = the ref being advanced,
position 1 = the ref merged in" — the two `ObjectId`s land in whichever order their hashes happen to
compare, unrelated to which one is mainline.

This must be resolved before §5 can start, and I see two options, neither of which I have picked:

1. **Relax the sort requirement for `BlockKind::Merge` specifically** — `is_strictly_sorted` stays
   enforced for `Root`/`Normal` (where it is trivially true or vacuous anyway) but not for `Merge`,
   where insertion order becomes semantically meaningful (`[mainline, secondary]`) instead of a
   canonicalization property.
2. **Add an explicit field** naming the mainline parent, leaving `parent_block_ids` sorted as today
   for uniform decode/lookup, with the field redundant-but-disambiguating within the sorted set.

Option 1 is smaller (touches `CanonicalEncode`/`decode_canonical`'s one validation branch, no new
field, no schema-version bump question) but changes what "sorted" means for one `BlockKind` — a
canonicalization property, which is exactly the kind of thing `EXECUTION-ORDER.md` §6 rule 5 asks to
be treated as a reviewed policy change, not a refactor. Option 2 is more invasive (new field, new
canonical-encoding tag, touches every existing `BlockPayload` construction site to decide what to put
there for `Root`/`Normal`) but leaves the sort invariant uniform. **Reporting this fork rather than
choosing it** — it's closer to a re-open of "does this need a schema version bump" than a mechanical
choice, and I'd rather it be ruled than guessed.

## 3. §4.2 — what `verify` must do with a `Merge` block, concretely

Given §1's recommendation and §2's fork (assume option 1 for concreteness; option 2 changes only where
the mainline pointer is read from, not this list):

1. `validate_block_v2_shape` gains `(BlockKind::Merge, [_, _]) => Ok(())` — exactly two parents,
   `Repair`/`Import` untouched (§4.3, below).
2. `verify_block_v2_state`/`derive_next_state_root`: for a `Merge` block, use `parent_block_ids[0]`
   (mainline, per §2 option 1's ordering) as the state-derivation parent — structurally the same call
   shape as `Normal` today, no new replay logic.
3. Confirm `parent_block_ids[1]` (secondary) exists and decodes as a `Block` — cheap, already the
   shape of existing `ensure_object_exists`-style checks elsewhere in `verify.rs`. Its own chain's
   soundness is covered by the independent full-object-store scan (§1.b) — no new walk from here.
4. Do **not** re-run `patch_algebra`'s confluence analysis. Trust the maintainer signature on the
   merge block, consistent with every other sealed decision (§1.c).
5. `patch_replay`/`patch_inverse`'s `single_parent_chain`-equivalents: continue through a `Merge`
   block via `parent_block_ids[0]` only, treating its `patch_ids` as an ordinary appended batch —
   unchanged in shape from how `Normal` blocks are walked today.
6. `merge_evidence.rs`'s `candidate_blocks`: same tolerance, so a *later* merge whose ancestry passes
   through an *earlier* merge block can still walk past it.

## 4. §4.3 — Repair/Import

**Recommend: leave both closed.** Nothing in this investigation found a reason to open either.
`validate_block_v2_shape` keeps `(BlockKind::Repair | BlockKind::Import, _) => Err(...)` exactly as
today; only the `Merge` arm changes. Matches the handoff's own caution against opening either by
accident.

## 5. Re-running DC-74's over-old-baseline scenario under this design

Per the addendum's explicit request: constructed the same case the DC-74 review found (`G → M1`
shared, `main` advances to `M2`, `topic` branches at `M1` and advances to `T2`, merge attempted with
baseline `G` instead of the true merge-base `M1`).

**Outcome is unchanged: it still fails closed, `Conflict`/`pair_conflict`, for the same reason.**
Nothing in §1–§4's design touches *how confluence is computed* — `patch_algebra`/`merge_evidence`'s
analysis is exactly DC-74's, unmodified. The recommendation here is entirely about how a *sealed*
`Merge` block is later *recorded and re-verified*, not about the confluence check `execute_merge` runs
before sealing one. **No soundness regression, because nothing about the confluence decision itself
changed.**

**One thing worth reporting precisely rather than leaving implicit:** DC-74's refusal here is a false
positive at the `patch_algebra` operation-classification level, not a genuine conflict. `M1`'s patches
appear in both the left sequence (baseline `G` to `M2`, which passes through `M1`) and the right
sequence (baseline `G` to `T2`, also through `M1`) as the *literal same sealed patch objects* — but
`patch_algebra`'s pairwise classifier works at the operation level (same-path-create, same-node
mutation, etc.), not patch-identity level, so it sees two "independent" creates at the same path and
calls it `SamePathCreate`/`Conflict` rather than recognizing "these are the identical historical
patch, already common to both sides, not a conflict." A correct baseline (`M1`) never exercises this,
so it is invisible in ordinary use — it only surfaces when a baseline older than the true merge-base
is supplied, which discovery being manual (DC-74 Q3) makes an easy mistake to make. **Recording this
as a finding, not fixing it** — DC-74's own non-goals explicitly place widening `patch_algebra`'s
conservative subset out of scope and ask for it to be reported rather than absorbed, and this is
exactly that: the current behavior is *safe* (refuses rather than silently mis-merging), just
imprecise about *why*.

## 6. What I did not do

No production code changed. No test changed. §2's format fork is unresolved by design — flagged for a
ruling, not guessed at. The O(N³) `verify` finding (§0) is reported, not fixed.

## Request

Report only, per criterion 1. Two things specifically need a ruling before §5 can start: §2's
sort-relaxation-vs-new-field fork, and whether §0's finding gets its own tracked row now or waits.
Everything else in §3/§4.2/§4.3 above is a recommendation I'm prepared to implement once §2 is settled.
