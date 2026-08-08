# RFC (proposed) - DC-75 Merge Block Lineage and the Structural Merge Record

**Status.** **Proposed 2026-08-08.** Awaits owner acceptance. **Not authorized for implementation.**
**Authored by** the architect, from a sizing finding produced by the developer.
**Independence.** The §2 sizing is the developer's work, verified by the architect — better than the
standing author-review ceiling, and recorded as such.
**Arises from.** DC-74's release condition (`MILESTONES.md`, "Attached release conditions"): *merge
execution does not ship until sealed history structurally records a merge.* **This RFC is the increment
that discharges that condition.** DC-74 is buildable and mergeable without it; neither is releasable
until it lands.

## 1. Why this is its own increment

DC-74's architect deferred multi-parent block lineage on the reasoning that the patch DAG already
recorded merges structurally. **`parent_patch_ids` is `Vec::new()` at every construction site, including
`worktree_patch/node_authoring.rs:534`, and is read nowhere.** There is no patch DAG. With single-parent
blocks, sealed history records neither the baseline a merge's confluence was checked against nor the two
sides — so **a later verifier cannot re-check that a merge was sound.** For a project whose claim is
history that cannot lie, that is the wrong artifact to ship, and history's immutability makes it
unrepairable after the fact.

## 2. What the work actually is — sized by the developer, verified by the architect

**The blocking gate is upstream of the machinery, not inside it.** `block_state.rs:13-26`
(`validate_block_v2_shape`) rejects `BlockKind::Merge | Repair | Import` as *"format-2 Block kind is not
authorized"* **before parent count is considered**, and requires `Normal` to have exactly one parent.

- `BlockKind::Merge = 3` **exists in the wire format** (`payload/block.rs:18`) and decodes.
- **Nothing in the tree constructs one.** This is greenfield write-side design, not a read-side widening.
- `cache_ladder.rs:31-36` already reserves `ParentPolicy::Dc13MergeAware` — *"Reserved for DC-13
  merge-aware baselines; rejected (fail closed) in v1."* **The project recognized this question once, in
  scaffolding, and did not resolve it.**
- Four tests assert the current behaviour and would **change**, not extend: `merge_lineage_fails_closed`,
  two `ParentPolicy::Dc13MergeAware` rejection tests, `multi_parent_candidate_fails_before_report`,
  `format2_parent_and_kind_matrix_is_closed`.

**An architect estimate that this was "confined to derived-state machinery" was wrong** and is corrected
here so it is not inherited. `verify.rs:327` and `checkout.rs:186` do already traverse all parents, but
that is irrelevant while the shape gate rejects the kind outright.

## 3. The design question at the centre — to be answered, not assumed

> **When a block has two parents, what does "the state derived from this block" mean, and against which
> parent(s) is it cryptographically verified?**

Two candidate answers, and the choice shapes `block_state.rs`, both `single_parent_chain` functions,
`merge_evidence.rs`'s candidate walks, and `ParentPolicy`:

- **Mainline-authoritative** (git `-m`-style): one parent is walked and verified; the other is retained
  as evidence only. Cheaper, and preserves a single linear verification path — but the second parent's
  history is then *named* without being *verified through*, which is weaker than it looks.
- **Both-parents-verified**: the merge block's `patch_ids` must be consistent against both independently
  replayed parent states. Strictly stronger and the honest reading of "history that cannot lie" — and it
  is the expensive one, since it doubles replay at every merge and interacts directly with DC-64's cache.

**This RFC does not choose.** §4 requires it be chosen with evidence.

## 4. Blocking prerequisites

1. **Answer §3's question with measurements, not preference** — including what both-parents-verified
   costs against DC-64's incremental cache, which is where the price lands.
2. **State what `verify` must do with a merge block** so that a merge's soundness is re-derivable by a
   party who was not present. This is the release condition's actual content; everything else is
   mechanism.
3. **Decide whether `RepairBlock`/`Import` stay unauthorized.** The shape gate rejects all three kinds
   together; opening one deliberately should not open the others by accident.

## 5. Acceptance criteria

1. §4 answered and reported before design.
2. A merge sealed by DC-74's execution path is recorded as `BlockKind::Merge` naming both parents, and
   `verify` re-derives its soundness **from sealed history alone**, tested through the compiled binary.
3. DC-64's cache, `rollback_preview`, `patch_inverse`, and `checkout` work against post-merge history.
4. The four tests in §2 are **changed deliberately, with the reason recorded** — not deleted.
5. **DC-74's release condition is discharged explicitly** in `MILESTONES.md` by the same commit that
   satisfies it.
6. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, verbatim, with test counts before and after.

## 6. Non-goals

- Merge *execution* — DC-74 owns it.
- Conflict arbitration, and patch aggregation. Separate themes.
- Populating `parent_patch_ids`. A patch DAG is a different structure answering a different question; if
  §3 shows it is the better vehicle, that is a **finding to report**, not scope to absorb here.
