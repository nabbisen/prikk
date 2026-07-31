# DC-64 Incremental Baseline Cache — Design v1

Required by `rfcs/accepted/DC-64-BASELINE-RECONSTRUCTION-COST.md` acceptance criterion 2 and the
architect's ruling at `.git-exclude/reviewed/prikk-dc64-trust-ladder-ruling-v1.md`: this document
states how a cached predecessor lifecycle state is prevented from becoming a root of trust, under
the ruling's four binding conditions. The implementation is
`crates/prikk-store/src/lifecycle_cache/incremental.rs`.

## 1. Scope, per the ruling

**This cache accelerates the commit path only.** It resolves the baseline `NodeLifecycleState` that
`worktree_patch/node_authoring.rs` compares the worktree against. It is not consumed by
`patch_algebra` or `merge_evidence.rs`, which reconstruct their own baseline through the unmodified
`replay_derived_state` and are untouched by this increment. If a future increment wants to consume
this cache from the merge path, the ruling is explicit that rung-4 (`ComparedLifecycleCache`)
full-replay certification would then apply — that is a new design question, not an extension of this
one, and this document does not attempt to answer it.

**Why the commit path is a narrower risk than the merge path.** The trust-ladder's rung 4 exists to
gate `node_id` **reuse** and **restoration-equivalence** decisions — both consumed only by
`patch_algebra::preimage` from `merge_evidence.rs`. The commit path never makes either decision: it
builds `baseline_files` from `live_nodes()` only, so a path with no live baseline entry is always
treated as a fresh create (`node_authoring.rs`'s `create_candidates` path always mints a new
`node_id`; it never consults `latest_tombstone_by_id` for restoration purposes). The commit path's
only dependency on lifecycle history beyond the live set is the mint-collision guard,
`NodeIdGenerator::mint_fresh`'s check against `contains_seen_node_id` (`node_id_gen.rs:124`) — which
is why `seen_ids` fidelity (§4) is a binding condition rather than an incidental detail.

## 2. What is cached, and where

One file, `cache_dir()/lifecycle-state.v1` (`.prikk/cache/lifecycle-state.v1`):

| Field | Meaning |
|---|---|
| `schema_version` | Wire format version |
| `baseline_block_id` | The block this cached state represents "as of" |
| `horizon_id` | The lineage genesis this chain is anchored to |
| `steps_since_reanchor` | Consecutive incremental steps since the last full replay produced this lineage |
| live entries | `(NodeId, LiveNode)` — path, kind, content (blob_id+mode or symlink target) |
| tombstone entries | `(NodeId, Tombstone)` — same shape, deletion preimage |
| `checksum` | SHA-256 over every preceding field's encoded bytes |

`seen_ids` is **not stored as a separate list.** `NodeLifecycleState::seed_live_node` and
`seed_tombstone` (`prikk-replay/src/node_lifecycle/mutation.rs:259-293`) both insert into `seen_ids`
as part of seeding a live or tombstoned entry, and — by construction of `create_node`/`delete_node`,
the only two mutators that ever touch `seen_ids` — it always equals exactly `live_by_id.keys() ∪
latest_tombstone_by_id.keys()` for any state reachable through this lifecycle model (no
restoration-equivalence path exists on the commit side to make a node "seen" without being either
live or tombstoned). Reconstructing via `seed_live_node`/`seed_tombstone` for every persisted entry
therefore rebuilds `seen_ids` completely and correctly as a structural consequence, not as a separate
concern that could be forgotten. **Binding condition 1 is satisfied by persisting every live and
every tombstone entry in full, never a truncated subset** — nothing here drops history to save space.

**Wire format.** Own magic (`PRIKK-LIFECYCLE-INCREMENTAL-CACHE-v1\0`), distinct from
`lifecycle_cache/cache_ladder.rs`'s `PRIKK-NODE-LIFECYCLE-CACHE-v1` — different cache, different
producer, must never be mistaken for the other's format. Encoded with the same general-purpose
`CanonicalWriter`/`WireType` primitives every other structured record in this codebase uses, for
correct primitive encoding; this is **not** identity-bearing (it participates in no `ObjectId` or
signature preimage), so it does not need — and does not attempt — the stricter
exactly-one-representation canonicalization identity-bearing formats require.

## 3. When an entry is trusted (the incremental path)

On each commit against a `Published` baseline, given target `baseline_block_id` and `horizon_id`:

1. Load the persisted cache. Any problem — file absent, checksum mismatch, decode failure — is
   treated as **absent**, never a hard error (NFR-PERF-04: rebuildable, never a root of trust).
2. The incremental path is attempted only if **all** of:
   - the cache's `horizon_id` matches the target horizon;
   - `steps_since_reanchor < REANCHOR_BOUND` (§5);
   - the *new* target block, read directly (one small object, not a lineage walk), has **exactly one
     parent**, and that parent **equals** the cache's `baseline_block_id`.
3. If eligible: apply **only that one block's** patch operations to a clone of the cached state,
   through `replay::apply_one_block` — which calls the *exact same* `apply_patch_ids`/
   `apply_state_effect` functions full replay uses, with a fresh, empty `TextCache`. `TextCache` is
   documented as a performance memoization only (`replay.rs:31-33`): on a miss it always falls back
   to reading the node's actual current blob content, so an empty cache changes nothing about
   correctness, only how many redundant blob reads a single step might do if it edits the same node
   twice (which authoring never produces — one op per node per patch).
4. The result is wrapped through `ReplayDerivedLifecycleState::from_replay` — **unchanged, not
   bypassed** (binding condition 3) — which runs `validate_internal_consistency` before the state may
   be used. This is exactly the same validation a full replay's result receives; incremental
   application is not exempt from it.
5. The refreshed cache (`steps_since_reanchor + 1`) is persisted.

**If eligibility fails, or the new block cannot be read:** fall through to the **unmodified**
`replay_derived_state` full-replay path (§5's fallback list, closed). A read failure for the new
block is not treated as a distinct case — the full replay path reads that same block as the last
entry of its own walked chain, so falling through produces the identical, properly-classified
`LifecycleReplayError` (missing/unreadable block) that a full replay would have raised anyway; it is
not a masked or narrowed condition, just the one path that already handles it correctly.

**If eligibility succeeds but application itself fails** (`apply_one_block` or `from_replay` returns
an error): this propagates as a genuine error, it is **not** silently retried via full replay. Binding
condition 4's fallback list is exhaustive — cache absent, corrupt, wrong horizon, parent mismatch,
multi-parent, reanchor bound reached — and "the attempted step itself failed" is deliberately not on
it. A full replay of the same lineage would encounter the identical malformed data and fail the same
way; masking that by silently falling back would hide a real problem behind a slower success instead
of surfacing it.

## 4. Why incremental application does not weaken the trust bar (the ruling's §3 correction)

The initial concern (this document's predecessor, the design-question escalation) was that
incremental application "compounds errors silently across cycles" in a way full replay cannot,
because full replay is self-correcting. **The architect's ruling corrected this:** full replay is
self-correcting against *state-persistence* faults (a stored, stale, or corrupted state) — not
against *computation* faults. It re-executes the identical `apply_state_effect` fold every time; a
latent bug in that fold corrupts a full replay exactly as it would corrupt an incremental step,
because both call the same function. Re-deriving a wrong computation from the horizon does not make
it right.

**What incremental application genuinely adds is exposure to persistence and serialization faults** —
a cached state that was correct when written but corrupted by storage bit-rot, or a decode bug that
silently produces plausible-but-wrong bytes. This is a narrower, addressable class:

- The **checksum** catches bit-level corruption the format parser's structural checks might not
  (a flipped bit inside a valid-looking `NodeId` or `ObjectId` still decodes; it does not still
  checksum).
- **`from_replay`'s structural check** catches internally inconsistent results (a node both live and
  tombstoned) regardless of source.
- The **reanchor bound** (§5) caps how long any fault that survives both could persist before an
  independent full replay overwrites the cache with ground truth.
- **`verify`'s divergence check** (§6) catches a fault retroactively, off the hot path, by comparing
  the cache's *current* claim against an independent full replay of the same block.

This holds specifically **because the design reuses the existing application functions** rather than
reimplementing the fold. A parallel incremental implementation of `apply_state_effect` would
reintroduce exactly the computation-fault risk the ruling distinguishes away, and would not be
covered by any of the four defenses above.

## 5. The reanchor bound

**`REANCHOR_BOUND = 64`.** After 64 consecutive incremental steps on one lineage, the next commit is
forced through the unmodified full-replay path regardless of cache eligibility, and
`steps_since_reanchor` resets to 0.

**Reason, stated per the ruling's requirement.** The bound is the only control on how long a
persistence fault that survives the checksum and `from_replay`'s structural check could live before
an independent reconstruction (a reanchor, or an explicit `verify`) recomputes ground truth and either
confirms or overwrites the cache. 64 was chosen to keep that exposure window small in absolute commit
count — under two full working days even at a sustained rate of dozens of commits — while keeping the
amortized cost of the mandatory full replay low relative to commit volume: one full replay per 64
incremental commits keeps its amortized share of total commit cost under ~2% for a repository whose
per-operation replay cost dominates, which is the exact regime DC-64's own measurement (~40 µs/op)
describes. This is a stated, revisitable choice, not a structural limit of the design — changing it
does not require touching the trust argument in §4, only re-justifying the exposure/overhead
trade-off above.

## 6. Divergence detection

`prikk verify` performs the check the hot path deliberately does not: it loads the persisted cache
(if present), performs an **unmodified full replay** of the block and horizon the cache currently
claims to represent, and compares the two `NodeLifecycleState`s for equality. A mismatch is reported
as a `LifecycleCacheDivergence` — the cache disagreeing with ground truth for the exact commit it
claims to already represent, which is the "reported, not silent" requirement (RFC §3, criterion 4).

This is not the same question as "is the cache eligible for the *next* commit's incremental step" —
`verify` does not know what the next commit will be. It answers a narrower, checkable question: is
the state the cache is currently offering to accelerate the next commit with actually correct for the
block it claims to represent. This mirrors DC-56's `verify_divergence` shape and, like that check,
this one necessarily costs a full replay every time it runs — acceptable because `verify` carries no
latency bound analogous to NFR-PERF-01.

## 7. Deletion and rebuild (NFR-PERF-04's evidence obligation)

Deleting `cache_dir()/lifecycle-state.v1` and then committing must produce a result identical to
committing with the cache intact — same operations, same patch, same `ObjectId`s. This holds by
construction: a missing cache is one of the four listed fallback triggers (§3), which routes to the
unmodified `replay_derived_state` full-replay path — the exact path every commit took before this
increment existed. There is no "cache present" code path that computes a *different* answer from the
"cache absent" path, only a cheaper one when a valid predecessor is available.

## 8. What did not change

No change to what a commit *means*: node identity, parentage, canonical ordering, and every existing
operation's applied effect are untouched — this cache only changes how many times
`apply_state_effect` executes to arrive at the same result, never what it computes. `from_replay`
remains the sole sanctioned constructor of a `ReplayDerivedLifecycleState`, reached by both the
incremental and full-replay paths. `merge_evidence.rs` and `patch_algebra` are untouched and continue
to call `replay_derived_state` directly.

## 9. Measured result — a substantial, real improvement, not a full flattening

The design eliminates its intended target exactly: at N=10,000 files, `try_incremental_step`'s
application of the one new block's operations costs ~2.6 ms, against the 370.6 ms the RFC measured
for full replay at the same size (`rfcs/accepted/DC-64-BASELINE-RECONSTRUCTION-COST.md` §1.1). The
O(operations replayed) cost — the dominant violator DC-64 exists to remove — is gone on the warm
path, confirmed both by the "removed the genesis patch, incremental step still succeeds" unit test
(§ correctness) and by profiling a real 10,000-file repository.

**It does not fully flatten Axis A**, and the reason is structural, not an oversight. Profiling the
same 10,000-file commit found three costs that remain proportional to **live node count** — the size
of the persisted state itself, independent of how many operations produced it:

| Phase | Cost at N=10,000 |
|---|---:|
| `load` (decode the persisted cache) | ~58 ms |
| `try_incremental_step` (apply the new block) | ~2.6 ms |
| `from_replay` (`validate_internal_consistency`) | ~5.4 ms |
| `persist` (encode, checksum, atomic write) | ~29 ms |

`load`, `from_replay`, and `persist` are each **binding conditions of the architect's ruling**, not
implementation choices open to trimming: `from_replay` "stays in the path, unmodified" (condition 3),
and `seen_ids` — reconstructed only by seeding every live and tombstoned entry — "must be persisted
complete and never truncated" (condition 1), which means the full state, not a delta, is what gets
written and read back every commit. Combined (~95 ms), plus DC-56's now-dominant metadata-walk-and-
cache-consult phase (~72 ms, itself O(repository size) — see the DC-56 finding), the warm-cycle total
tracks repository size at a **lower, but non-flat, slope**: DC-59's Axis C (consecutive commit+seal
cycles, `crates/prikk-cli/tests/dc59_commit_benchmark.rs`) measured warm cycles at 1,000 files
(~30 ms) to 10,000 files (~236 ms) — roughly 7.9× for 10× repository size, against cold/full-replay's
~9× at the same sizes. A real, substantial reduction (roughly 2–2.5× overall at 10,000 files, and the
dominant historical-replay cost specifically eliminated), not the flat curve criterion 6 as originally
worded asked for.

**This directly bears on criterion 7 too** (amended already, §2b of the ruling, to target worktree
content and the live node set specifically): the persisted cache **is** a full copy of the live node
set, so `load`/`persist` holding it in memory does not reduce memory below what holding the
`NodeLifecycleState` itself already costs — DC-64 does not, and structurally cannot, make that
component smaller while satisfying condition 1.

Reported as measured, per the RFC's own instruction not to conclude compliance here — that
determination, and any decision to further reduce `load`/`persist`'s cost (e.g. an incremental, not
full, persisted representation) within the ruling's constraints, belongs to the architect.
