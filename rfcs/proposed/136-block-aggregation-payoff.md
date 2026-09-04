# RFC 136 — The block aggregation payoff: what sealing a block should make cheap

**Status.** **ACCEPTED by the project owner 2026-09-04**, the same day it was opened at their
instruction, in answer to their own question: the block concept came from the fact that patch-based
version control is forced into heavy calculation, and the intent was *"to make it by far more efficient
by aggregating multiple patches into a single patch as block when a cycle of development on some theme
is finished."*

**What the acceptance covers, stated because a bare acceptance is scope-ambiguous.** It accepts the
problem record, the evidence, and the shape of the question — the same reading RFC 133's acceptance
carried. **It does not answer §7**, and §7 remains the one thing outstanding. Following RFC 101/102's
precedent, **acceptance clears §9's measurements only**: no object-model change, no snapshot policy,
no composition definition, no handoff.

**§9.1 was measured under that clearance on 2026-09-04, and it eliminates one option.** Option B's
collapse ratio over this project's own 600-commit history is **1.16-1.25x at realistic block sizes**,
and the measured figure is an *upper bound* on what composition could achieve. **The architect
recommends against Option B**; Options A and C are untouched by it, because neither depends on
collapse. §7's question is unchanged and still the owner's.

**Nothing else is ruled.

**Author-review independence.** The architect wrote this RFC and is also its only reviewer, the
standing gap recorded on every architect-authored design in this project. Compensated at
implementation review, not here.

**Tracks.** Cost, and the object model that carries it. **No behaviour change is proposed in this
document.**

---

## 1. Where the concept came from, and why it needs a record

The block was not a storage convenience. It was the answer to the classical patch-VCS cost problem:
if history is a bag of patches, then reasoning about a merge means reasoning about relations among all
of them, and the work grows with history rather than with the change being made. Sealing a run of
patches into one unit was meant to cut that.

That intent has never been written down in this repository. `docs/src/reference/data-model.md`
describes what a Block *is*; nothing describes what sealing one is *for*. The result is that half the
mechanism was built, the other half was left as a field nobody writes, and no document records that
the second half is missing. This RFC is that record.

## 2. What a Block is, as built

`crates/prikk-object/src/payload/block.rs:49`:

| Field | Meaning |
|---|---|
| `parent_block_ids` | sorted; 0 for `Root`, 1 for `Normal`, 2 for `Merge` |
| `kind` | `Root` / `Normal` / `Merge`; `Repair` / `Import` are minted but unauthorized |
| `patch_ids` | the patches this block seals, in canonical block order |
| `state_merkle_root` | commits to the complete replay-derived live-node set in canonical path order |
| `snapshot_blob_ref` | **optional full snapshot of that state** |
| `mainline_parent_id`, `merge_baseline_block_id` | `Merge` only (DC-75) |

`prikk commit` appends patches to the active WAL; `prikk seal` turns everything accumulated since the
last seal into one block. The seal boundary is already exactly the owner's *"a cycle of development on
some theme is finished."*

## 3. What the block already buys — the half that works

**3.1 Merge work is bounded by the block window, not by history.**
`crates/prikk-store/src/merge_evidence.rs:293-315` derives a side's candidate sequence as
`ancestors(target) \ ancestors(baseline)` — the blocks strictly between the sealed baseline and the
target. Confluence then checks cross-pairs between the two candidate sequences
(`docs/src/reference/patch-algebra.md`, "Flat Confluence"). The work is **O(L x R) in patches since
the sealed baseline**, not in history length.

This is the escape from the classical cost, and it is real. The sealed block is what makes "since the
baseline" a bounded, cheap-to-name set instead of the whole history.

**3.2 Whole-tree state comparison is constant time.** `state_merkle_root` commits to the entire
live-node set, so two blocks' states compare in O(1) with no replay at all.

**3.3 A merge records the scope of its own proof.** A `Merge` block carries
`merge_baseline_block_id`, the block confluence was proven against — which `verify` re-derives rather
than trusts.

## 4. What it does not buy — and the exact evidence

**4.1 A block names its patches; it does not compose them.** `patch_ids` is a list. Every patch stays
individually addressed and individually replayed. There is no "block as a single patch" anywhere in
the object model.

**4.2 `snapshot_blob_ref` is never written.** All three paths that create a block hard-code `None`:

- `crates/prikk-cli/src/seal.rs:174` — `prikk seal`
- `crates/prikk-store/src/seal_from_accepted.rs:224` — sync receive
- `crates/prikk-store/src/merge_execute.rs:175` — `prikk merge`

`SnapshotManifest::encode` (`crates/prikk-store/src/snapshot.rs:74`) is called from **test code only**.
No production path has ever produced a snapshot.

**4.3 The consequence.** State at a block is derived by replaying its lineage — O(patches from
genesis). `merge_evidence.rs:50-51` shows the asymmetry in a single place: *classification* is bounded
by the window, but establishing the **baseline state** that window is measured against is a full
`replay_derived_state` from the lineage horizon.

**4.4 Two caches soften this, and neither of them is the block.**

| Cache | Scope | Persistence |
|---|---|---|
| `LineageStateMemo` (DC-92, `block_state.rs:83-131`) | one process invocation; takes a whole `verify` from O(N^2) to O(N) | never persisted |
| incremental baseline cache (DC-64, `lifecycle_cache/incremental.rs`) | **commit path only**; re-anchors to a full replay every 64 steps | persisted, rebuildable, never authoritative |

This matches the RFC 133 measurement directly: incremental commit memory flat at 16.3 MiB, genesis
commit 1:1 with total worktree bytes.

## 5. What is already built on the read side — and the one place it is not fit for purpose

The read half of the snapshot mechanism is complete and waiting:

| Site | What it already does |
|---|---|
| `checkout.rs:219-227` | classifies a block with a snapshot as `RequiresSnapshotMaterialization` instead of `RequiresPatchEngine` |
| `checkout.rs:88-118` | `prepare_snapshot_checkout_plan` decodes the manifest and plans the materialization |
| `verify.rs:1565` | checks the referenced Blob exists |
| `bundle.rs:801` | transports it as a reachable blob |
| `patch_replay/read.rs:116` | reads it as a replay baseline |

**But the v1 snapshot format inlines file contents.** `SnapshotEntry` is `{ path: RepoPath, bytes:
Vec<u8> }` (`snapshot.rs:14-19`) — full content, not a Blob reference. A snapshot written at every
seal would therefore store a **second complete copy of the worktree per block**, so N sealed blocks
cost N x worktree bytes on top of the blobs already stored.

That is disqualifying as written, and the fix is already implied by the model: the state root's own
leaves bind *"the exact repository path, nonzero NodeId, node kind, normalized mode, and either the
file Blob ObjectId or opaque UTF-8 symlink target"* (`data-model.md`). A path-to-Blob-id manifest is
the state entry set the Merkle root already commits to, and it is small. **The existing snapshot
format is a worktree materialization aid, not a history-scale artifact, and this RFC should not
pretend otherwise.**

## 6. The constraint that shapes every option

`docs/src/reference/data-model.md` already states the governing rule, in shipped documentation:

> Snapshots and caches may be used only as checked auxiliary data; they cannot override replay.

This has a consequence worth stating plainly before any option is weighed: **verification cannot be
accelerated by any of this.** `verify` must still replay a block's patches and compare the result
against the recorded `state_merkle_root`; a snapshot that hashes to the same root proves the snapshot
matches the root, never that replaying the patches produces it. The accelerable surfaces are
**checkout** and **baseline reconstruction** (commit, merge), not verification.

Any option that appears to speed up `verify` has violated this rule and is wrong.

## 7. The option space — and the question for the owner

Three shapes are available. They are not variations on one design; they differ in what is stored, what
travels, and what identity depends on.

### Option A — full-tree snapshot in the block

Write `snapshot_blob_ref` at seal, as a path-to-Blob-id manifest (§5, not the v1 inline format).

- **Buys:** O(tree) checkout at any sealed block with no replay; O(tree) baseline reconstruction.
- **Costs:** a new snapshot format; a policy for *when* to snapshot (every seal is wasteful, never is
  useless); storage proportional to snapshot frequency.
- **Travels in bundles:** yes — a fresh receiver gets the acceleration immediately.
- **Identity:** the field is inside the canonical encoding (`block.rs:282`), so the block id depends
  on it. See §8.

### Option B — a composed net patch per block

Store the block's **net effect** as one operation sequence, alongside (not instead of) `patch_ids`.

- **Closest to the owner's original words.** "Aggregating multiple patches into a single patch as
  block" is literally this.
- **Buys:** applying one block becomes O(net change) rather than O(all operations in the block) —
  the win grows with how often a block's patches touch the same files, which is exactly what a
  finished theme looks like.
- **Costs:** composition must be defined and proven equal to sequential replay, over the full
  operation set including `EditText` — and RFC 134 is this project's own evidence that composition
  over text spans is where the hard cases live. No field exists.
- **MEASURED 2026-09-04 (§9.1): the payoff is 1.16-1.25x at realistic block sizes, and that figure is
  a ceiling.** **Architect recommends against.** Not refused — the owner rules — but the proof cost is
  not proportionate to a 20% reduction in operations replayed.
- **Travels in bundles:** yes.
- **Identity:** same question as A.

### Option C — a repository-local snapshot cache, outside the object model

Keyed by block id, in the DC-64 mould: persisted, rebuildable, never authoritative, no object-model
change at all.

- **Buys:** the same checkout and baseline-reconstruction wins, locally.
- **Costs:** does not travel — a fresh clone replays once before it benefits. Cache-invalidation
  surface, which DC-64 already has an answer for (`REANCHOR_BOUND`).
- **Identity:** **untouched.** Nothing enters a signed object.

**The question:** *is the acceleration something a repository should be able to hand to another
repository (A or B), or is it purely local (C)?* Everything else follows from that answer, and it is
the owner's to give, because it is a question about what prikk distributes, not about how it computes.

## 8. The identity question — and why it is smaller than it first appears

The obvious objection to A and B is that they put **derived** data inside an **identity-bearing**
object: two repositories with identical history would seal different block ids depending on whether
they chose to snapshot.

That objection is real but already priced in, and `docs/src/reference/data-model.md` says so:

> `target_block_id` itself does not survive a move: blocks diverge between repositories by design even
> when the underlying history is identical.

Block identity is **already local**. Cross-repository portability runs through `patch_set_digest` —
the digest of the block's own patch closure, `DOMAIN ‖ count ‖ sorted patch ids` — which does not
depend on `snapshot_blob_ref` and would not begin to.

So the residual question is narrower than "may identity depend on a cache". It is: **should two seals
of the same patches, differing only in a local performance choice, be distinguishable objects?** For
Option C the question does not arise. For A and B it must be answered explicitly rather than absorbed.

## 9. What must be measured before an option is chosen

RFC 133 established the measurement discipline this project now expects. None of these numbers exist
yet, and this RFC should not be implemented on any of them assumed:

1. **Checkout cost today** at a realistic history depth — the number Option A and C both claim to cut.
   Never measured.
2. **Baseline reconstruction cost** on the merge path specifically. DC-64 measured the commit path;
   merge's own `replay_derived_state` call (§4.3) was not in that scope.
3. **Compression ratio for Option B** — how many operations a real sealed block's net effect collapses
   to. If a typical block's patches touch mostly disjoint files, B buys close to nothing and its
   composition-proof cost is unjustified.
4. **Snapshot storage cost under a candidate policy**, once the format is path-to-Blob-id rather than
   inline (§5).

Item 3 is the one that can kill an option outright, and it is cheap to obtain. **It was measured on
2026-09-04 — see §9.1, which is why Option B is no longer recommended.** Items 1, 2 and 4 remain
unmeasured and no option may be implemented on any of them assumed.

## 9.1 Measured 2026-09-04 — Option B's collapse ratio, and why it is a ceiling

**Source.** This project's own last 600 non-merge commits, file lists from
`git log --pretty=format:'@@%H' --name-only --no-merges -n 600`. Mean 3.37 files changed per commit.
A block is modelled as a run of K consecutive commits — non-overlapping windows, the shape a real
"seal every finished theme" produces.

**Metric.** `sum(files touched by each patch) / count(distinct files in the window)`. This is exactly
the factor by which composition collapses *file-level* operations.

| Block size K | aggregate ratio | median | p90 | blocks collapsing **nothing** |
|---:|---:|---:|---:|---:|
| 2 | 1.05 | 1.00 | 1.67 | 77% |
| 3 | 1.11 | 1.01 | 1.50 | 50% |
| 5 | 1.16 | 1.16 | 1.67 | 17% |
| 8 | 1.22 | 1.23 | 1.64 | 4% |
| 10 | 1.25 | 1.22 | 1.62 | 2% |
| 20 | 1.35 | 1.40 | 1.75 | 0% |
| 50 | 1.55 | 1.65 | 1.92 | 0% |

**This is a ceiling, not an estimate.** For a file touched by *n* patches in a block, the composed net
effect is *at most* `n` operations (all spans disjoint) and *at least* 1 (fully overlapping). So
composed operation count is bounded below by the distinct-file count, and the true operation-granular
collapse can only be **less** than the table. Composition cannot do better than these numbers; it can
do worse.

**What it means.** At realistic block sizes a block's patches touch mostly disjoint files. Option B
would buy roughly a **20% reduction in operations replayed** while requiring composition to be defined
and proven equal to sequential replay across the full operation set — including `EditText`, where
RFC 134 is this project's own evidence of how hard that is. At K=3, half of all blocks would collapse
nothing at all. **The proof cost is not proportionate to the payoff, and this is the option closest to
the original wording — which is why the number had to be taken before ruling rather than after.**

**Three limits on this measurement, stated so it is not over-read.**

1. **It is prikk's git history, not prikk sealed history**, because no prikk repository with realistic
   development history exists to measure. Git commits stand in for patches. Prikk's own granularity is
   the same or finer, since `commit` queues several patches per seal.
2. **It is one project with a disciplined one-theme-per-commit rhythm.** A project that accumulates
   many small fixups against the same file would show a higher ratio. This bounds *prikk's* case, not
   every case.
3. **It says nothing about Options A or C.** A full-tree snapshot's payoff depends on tree size against
   history depth, not on how much a block's patches overlap. Both remain fully live, and §7's question
   is unaffected.

## 10. Scope

**Proposed here:** the problem record, the evidence, the option space, and the §7 question.

**Not proposed here:** any change to the object model, any snapshot policy, any composition
definition, any behaviour change, and any handoff. `Repair` and `Import` block kinds stay unauthorized
and are out of scope; `verify` acceleration is excluded by §6 and is not a goal of any option.

**Related:** RFC 133 (cost and its evidence — the measurement discipline this RFC defers to),
RFC 134 (text span identity under composition — the prior art for Option B's hard case), DC-64
(incremental baseline cache — Option C's existing mould), DC-92 (lineage replay memoization),
DC-75 (merge block shape).
