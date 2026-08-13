# DC-75 — Addendum-3 §3 Answered: Reachability and State Derivation Do Not Cleanly Split in Two

**Handoff followed:** `implementation-handoff-v1-addendum-3.md` §3.
**Governing RFC:** `rfcs/done/DC-75-MERGE-BLOCK-LINEAGE.md`

Per §4's discipline, restated in addendum-3 ("Report... before any design... trace it explicitly...
rather than reasoning about it"): this reports before any design or production code. All code below was
run as a temporary, reverted probe (`git diff` is empty; nothing is committed).

**Headline: item 1's split is correct but not sufficient by itself. A repeated merge between the same
two branches still fails under the sketched fix, for a reason distinct from the reachability question —
demonstrated by construction, not argued.**

## 1. Catalog — which functions are reachability, which are state derivation, as they exist today

| Function | Category | Current behavior |
|---|---|---|
| `block_state.rs`: `verify_block_v2_state` → `derive_next_state_root` → `verify_v2_lineage_roots` → `walk_lineage_to_genesis` | State derivation | Single-parent only today; §1's ruling already says a `Merge` block uses `parent_block_ids[0]` here — correct category, not yet implemented |
| `patch_replay`/`patch_inverse`'s single-parent-chain walks (materialization, checkout, rollback) | State derivation | Same — investigation §3 item 5 already places these on `parent_block_ids[0]` only |
| `merge_evidence.rs`: `lineage_horizon` (finds the baseline's own root, for replaying the baseline's `NodeLifecycleState`) | State derivation | Belongs here, not in reachability — it answers "what is the state *at* this block," not "is this block reachable from that one." Today it errors on any multi-parent block in the walk, same restriction as the row below; needs the same `parent_block_ids[0]`-on-`Merge` treatment as `block_state.rs`, not a DAG walk |
| `merge_evidence.rs`: `candidate_blocks` (shared by `candidate_sequence` — evidence/display — and `candidate_patch_ids` — execution's adoption set, `merge_execute.rs:95`) | **Conflates three questions today** | Verified directly: walking from a 2-parent block errors `"merge evidence requires single-parent candidate chains... has 2 parents"` **immediately**, the instant it reads that block — not "not an ancestor." It does not distinguish "is baseline reachable," "what state does target represent," and "what operations happened since baseline" — it refuses all three at once on any multi-parent block anywhere in the path |
| A merge-base/ancestor re-derivation for `verify` to cross-check a recorded baseline (this increment's own `baseline-recording-answer-v1.md`) | Reachability | Does not exist in code yet. Measured cheap by construction (linear, §1 of that document) — that finding is unaffected by anything below |

**One correction to addendum-3's own trace, confirmed by running it, not by re-reading:** the trace
describes `candidate_blocks(T1, M2)` as walking `M2 → M1 → G` and erroring `"is not an ancestor"` once it
reaches genesis. That is the **post-fix** behavior (mainline-only walk that tolerates `Merge` blocks by
following `parent_block_ids[0]`) — it does not exist yet. Today's code errors the instant it *reads* `M2`
itself, since `M2` already has two parents, with a different message (`"single-parent candidate chains"`).
This does not change §2 below in outcome, only in which error text a user sees before §5 lands.

## 2. Is `candidate_sequence`'s left-side operation set well-defined once ancestry is a DAG? No — demonstrated, not argued

**Construction** (`merge_evidence/tests.rs`, temporary, reverted):

1. `G → M1` on `main`. `topic` branches at `M1`, patch `t1_patch` creates `topic.txt`, sealed as `T1`.
2. First merge constructed directly at the shape a `Merge` block would have: `M2`'s parents `[M1, T1]`
   (sorted, matching the format's invariant), `patch_ids = [t1_patch]` — `t1_patch` adopted **verbatim**,
   same `ObjectId`, exactly DC-74's B′ semantics.
3. `topic` advances again: `T2`, a new patch on top of `T1`.
4. Right side (from `topic`, still a plain single-parent chain `T1 → T2`): unmodified
   `candidate_sequence(T1, T2)` works today, unchanged — 1 operation, `topic2.txt`'s create.
5. Left side (`main`, baseline `T1`, target `M2`): applied the **sketched fix directly** — for this
   graph, "ancestors of `M2` minus ancestors of `T1`" is exactly `{M2}` (`M1` and `G` are already
   ancestors of `T1` too, since `topic` branched from `M1`). So the fixed left-side set is `M2`'s own
   `patch_ids`, decoded: `t1_patch` — **the same patch object `T1` itself already contains**, decoded a
   second time.
6. Fed both sides into the real pipeline — `analyze_merge_evidence`, the exact function
   `prepare_merge_evidence` calls, with baseline `T1`'s real replayed state.

**Result:**

```
outcome=NotConfluent
items=[MergeEvidenceItem { side: Cross, operation_index: Some(0), peer_operation_index: None,
  outcome: NotConfluent, proof_phase: ReplayBothOrders, reason_code: PairReplayFailed, ... }]
```

**This is worse in kind than DC-74's over-old-baseline finding.** That one produced `Conflict` /
`pair_conflict` — a designed, understood outcome the classifier reaches deliberately. This produces
`NotConfluent` / `PairReplayFailed` — the proof engine tried to **replay** `topic.txt`'s create against a
state that already has `topic.txt` (because baseline `T1` already reflects that patch) and the replay
itself failed. It is not a conflict *classification*; it is the confluence proof breaking on an input it
was never built to receive.

**Root cause, confirmed by reading, not inferred:** `DecodedPatchOperation` (`patch_replay/decode.rs:29`)
carries only `op_seq` and the decoded operation body — no patch `ObjectId`, no source-patch identity.
`analyze_merge_evidence` (`patch_algebra/report/analysis.rs:25`) receives `&[DecodedPatchOperation]` on
each side and never sees which sealed `Patch` object an operation came from. **There is no mechanism
anywhere in this pipeline, at any layer, that could recognize "this operation is the same sealed patch
already reflected in the baseline" and exclude it.** Every existing call site happens to avoid this today
only because no walk has ever fed the pipeline a patch that's simultaneously the baseline's own content
and a "new" operation on one side — which is exactly what a repeated merge does, and exactly what §5
would be the first thing to construct.

**Answer: not well-defined. It needs a rule this increment must state** — some form of patch-identity
membership test (is this operation's source patch already reachable from the baseline via *any* parent
path, not just the side currently being walked?) applied before operations are handed to
`analyze_merge_evidence`, not after. I am not proposing the mechanism's placement or exact shape — that
is design, and item 2 of addendum-3 asked only whether the set is well-defined, not what fixes it.

## 3. Do repeated merges between the same pair work under the sketched fix? No, not by itself

Reachability alone **does** succeed trivially here: `T1` is `M2`'s direct secondary parent, one hop, no
walk needed. The blocking failure is entirely §2's — the sketched "left side follows all parents too"
fix is necessary (without it, execution can't even locate the operations) but **not sufficient**: once it
can locate them, the pipeline it hands them to breaks on the very case repeated merges guarantee it will
see — a side whose "new" content is content the baseline already carries by adoption.

**So the correct statement of the blocker is not "mainline-only vs. all-parents."** It is: **the walk
that finds candidate operations, and the classifier that judges them, need a shared notion of patch
identity that neither has today.** Fixing only the walk (§1's sketch) trades one failure mode
(`"single-parent candidate chains"`, today) for a different, worse one (`PairReplayFailed`, demonstrated
above) — not for success.

**No soundness regression either way**: both the current behavior and the sketched-fix behavior refuse
rather than produce a wrong merge. The finding is a capability gap plus a proof-engine fragility, not a
correctness risk.

## 4. Disposition on addendum-3's closing question

**Incomplete, not wrong.** The reachability/state-derivation split (§1 of addendum-3, and the ruling it
rests on) is still the right direction and does not need to be re-ruled — §1's category assignments in
the table above hold. What's missing is a third piece neither addendum-3 nor the original investigation
named: a patch-identity rule for candidate-sequence construction, without which §5 cannot support the
single most ordinary multi-merge workflow.

## 5. What I did not do

No production code changed. No test changed — the probe is reverted (`git diff` empty). No rule
proposed as binding; the shape described in §2 is reported as the missing piece, not designed.

## Request

Report only, per §4's discipline. A ruling is needed on whether to scope the patch-identity rule into
this increment's own §5 design work, or split it into a fourth handoff item — that choice belongs to the
architect, not to this document.
