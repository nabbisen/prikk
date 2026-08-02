# DC-69 Prerequisite Questions — §3.1 Judgment, §3.2 Re-Examination, §3.4 Measurement

Per the handoff: §3.1 and §3.2's *factual* halves were discharged by the architect at acceptance;
this document verifies them (as instructed), answers §3.1's judgment half, and reports a finding
that **contradicts §3.2's conclusion as stated**. §3.3 is deliberately not designed here — the
handoff is explicit that it must not be designed before §3.1/§3.2 are settled, and §3.2 is now, by
this document's own finding, unsettled.

## 1. §3.1 — verified, and the judgment half answered

**Factual half, confirmed by reading, matches the architect's claim exactly.** The only commit-path
production consumer of `contains_seen_node_id` is `node_id_gen.rs:124`
(`NodeIdGenerator::draw_candidate`, called from `mint_fresh`). The other two call sites
(`patch_algebra/preimage.rs:79,232`) are merge-path — confirmed below in §2, with a caveat.

**Judgment half.** `mint_fresh`'s shape (`node_id_gen.rs:130-146`): draw once; on an all-zero or
colliding candidate, redraw exactly once; a second failure returns a structured error rather than
retrying further. Three named candidate threats:

- **A deterministic test generator escaping into production.** Structurally impossible today, and
  not because of the collision check: `NodeIdGenerator::with_source` (the only way to inject a
  non-`OsEntropySource`) is `#[cfg(test)]`-gated (`node_id_gen.rs:106-109`); `production()` — the
  only constructor available outside `#[cfg(test)]` — hardcodes `OsEntropySource`
  (`node_id_gen.rs:96-103`). A release build cannot compile a call to `with_source` at all. The
  collision check contributes nothing to this threat; the type/cfg boundary already closes it.
- **Degraded or stubbed entropy.** `OsEntropySource::fill_node_id_bytes` propagates
  `getrandom::getrandom`'s own result as `NodeIdMintError::EntropyUnavailable` on any error
  (`node_id_gen.rs:78-82`). `getrandom`'s documented contract is to return cryptographically secure
  bytes or fail explicitly — never silently weak ones. So genuine entropy degradation is already
  caught upstream of the collision check, at the entropy-source boundary, not by it.
- **A future non-random id scheme.** Speculative — nothing in the current codebase does this. If
  introduced, the collision check's *detective power against it is weakest exactly when it would
  matter most*: it only fires when a draw happens to match something already in `seen_ids`, so its
  chance of firing scales with how much of `seen_ids` the flawed scheme's output space overlaps —
  near zero early in a young repository or early in the flawed scheme's own rollout, which is
  precisely when you would want the strongest signal, not the weakest.

**Conclusion (§3.1 judgment): the guard is not load-bearing for any threat currently reachable in
this codebase.** Threat 1 is closed by Rust's own compilation model, not by this check. Threat 2 is
closed by `getrandom`'s own contract, not by this check. Threat 3 is not a present threat, and this
check would be a poor detector for it even if it were. The architect's own reframing —
*"ask whether checking the entropy source is a better control than remembering every id ever
minted"* — is correct and, on this analysis, already substantially true: the entropy source **is**
already checked (`EntropyUnavailable`), and that check is a strictly better control than the
membership test, since it is constant-cost and does not degrade with repository age.

**This conclusion is scoped to the commit-path mint guard specifically.** It says nothing yet about
whether `seen_ids` is needed for *other* reasons — see §2 below, which finds one.

## 2. §3.2 — re-examined, and found incomplete as stated

**The architect's claim:** *"The restoration-equivalence and `NodeIdReuse` decisions that need
tombstones live in `patch_algebra`, reached from `merge_evidence.rs`, i.e. the merge path."* Cited
consumers: `node_lifecycle/validation.rs:33`, `query.rs:33-41`, and DC-64's
`lifecycle_cache/incremental.rs:189` (persisting, not deciding).

**What I found by tracing every caller of `NodeLifecycleState::create_node`, not just the two named
tombstone-touching files:**

```
grep -rn "\.create_node(" crates/ (production only):
  crates/prikk-store/src/worktree_patch/node_authoring.rs:468   — fresh authoring (mint_fresh output; never a seen id)
  crates/prikk-store/src/lifecycle_cache/replay/effect.rs:34,49  — apply_state_effect's CreateFile/CreateSymlink handler
  crates/prikk-store/src/patch_algebra/replay_oracle.rs:142      — merge-evidence oracle
```

`node_lifecycle::mutation::create_node` (`crates/prikk-replay/src/node_lifecycle/mutation.rs:17-55`)
is the **one shared entry point** all three routes call. Its body, for a `node_id` already in
`seen_ids` but not live:

```rust
if self.seen_ids.contains(&node_id) {
    let tombstone = self.latest_tombstone_by_id.get(&node_id).ok_or_else(|| {
        PrikkError::Integrity("seen node_id has no recorded tombstone for restoration-equivalence")
    })?;
    ensure_restoration_equivalent(tombstone, &node)?;   // checks kind, content, AND path
}
```

This is a genuine correctness decision over tombstone **content** (not merely key-set membership),
and it is reached by `lifecycle_cache/replay/effect.rs` — the shared `apply_state_effect` fold that
**both** full replay (`replay_derived_state`) **and** DC-64's incremental step
(`try_incremental_step` → `apply_one_block`) **and** DC-66's queue fold
(`apply_queued_patch_envelopes`) call. All three are commit-path baseline reconstruction, run on
every `prikk commit`. This is not the merge path; `patch_algebra/replay_oracle.rs`'s call to the
same `create_node` is the merge path, and is a **separate, third** call site the two-file citation
did not name.

**When is this branch actually reached, given ordinary authoring only mints ids `mint_fresh` proves
are unseen?** `rollback-draft`'s inverse-patch construction is the answer:
`patch_inverse.rs:249-251` inverts a historical `DeleteNode` into a `CreateFile` that **reuses the
original `node_id`** — by design, so an undone deletion restores the same node identity rather than
minting a new one under the old path. Once such a patch is authored, committed, and sealed, it
becomes a normal block in the sealed lineage. **Every subsequent commit's baseline replay — full or
incremental — must then replay that `CreateFile`, and `create_node`'s restoration-equivalence branch
fires**, requiring `latest_tombstone_by_id` to still hold the exact tombstone for that `node_id` at
that point in the replay.

**How far back can this reach?** `prepare_patch_inverse_plan` (`patch_inverse.rs:94-100`) walks
`single_parent_chain` — the **entire lineage** back to the last snapshot or genesis, accumulating
inverse operations across every block — not a bounded recent window. So a rollback-draft is not
restricted to undoing only the most recent commit; a deletion from arbitrarily far back in a
repository's history is a legitimate restoration target today.

**Conclusion (§3.2): the commit path does consult tombstone content, in any repository that has ever
sealed a rollback-draft restoration, for as long as any future commit needs to replay through that
point in history — which, for a full replay, is always, and for DC-64's incremental cache, is
whatever the persisted predecessor state itself must have retained to have been correctly
constructed.** This does not mean route (c) is wrong — it may still be the right answer — but it
means the clean split §3.2 proposed ("commit doesn't need tombstones; that's a separate, later,
merge-path question") does not hold as stated, and any retention design that drops tombstone content
on the assumption that commit never consults it would silently break replay of any repository that
has used rollback-draft.

**This is reported per the handoff's explicit instruction** ("if something here contradicts what the
code actually does — including anything in §2 — stop and report it"), not resolved unilaterally: it
changes what a bounded-retention mechanism would need to preserve, and possibly whether one is
coherent at all, which is exactly the kind of premise this program's process exists to catch before
design proceeds on top of it.

## 3. §3.4 — measurement: Axis D, long history at a fixed small tree

**Method.** Added to `crates/prikk-cli/tests/dc59_commit_benchmark.rs` as a new, self-contained,
`#[ignore]`d pass (`axis_d_long_history_small_tree`), never interleaved with `commit_benchmark`'s
existing Axis A/B/C or the memory pass — same precedent as DC-62's memory pass. Tree size is held
fixed at 20 files throughout. Each generation deletes the oldest tracked file and creates a new one
at a fresh path (churn, not edit — an edit mints or tombstones nothing, so it cannot exercise
`seen_ids`/`latest_tombstone_by_id` growth at all), keeping live tree size constant while
`seen_ids`/`latest_tombstone_by_id` each grow by one per generation. History depths measured:
10/50/100/200 generations, 3 samples each; only the final (depth-th) commit is timed per sample, all
earlier generations are untimed setup — matching Axis A/B/C's "time only the measured commit"
convention.

**Results** (full table in `axis-d-benchmark-report-v1.md`, this directory, generated by the test
itself):

| History depth | Live tree size | Median |
|---:|---:|---:|
| 10 generations | 20 files | 2.66 ms |
| 50 generations | 20 files | 5.47 ms |
| 100 generations | 20 files | 9.01 ms |
| 200 generations | 20 files | 17.91 ms |

**Reading.** Tree size is identical (20 files) at every row; only cumulative history depth varies.
Cost still climbs roughly **linearly** with depth — a linear fit (`cost ≈ 1.86 ms + 0.080 ms ×
depth`) predicts all four points within ~10%. 20x more history (10 → 200 generations) costs ~6.7x
more per commit, at a tree size that never changed. **This directly confirms the RFC's framing**:
the cost DC-64 measured (~93 ms at 10,000 files) is not only a large-tree phenomenon — a
*permanently small* repository with enough historical churn pays a real, growing, unbounded-in-the-
limit cost too, from `seen_ids`/`latest_tombstone_by_id` alone. "A repository with a decade of
churn does not have a slow commit, it has a commit whose cost nobody has bounded" (RFC §1) is not
speculative; this axis measures it directly, isolated from repository size for the first time.

## 4. What this means for the increment's next step

Per the handoff's own definition of done and standing request, and per this project's established
precedent (DC-64's trust-ladder question, most directly): **this document stops short of proposing a
mechanism, declaring route (c), or designing §3.3**, because §3.2's premise — which §3.3 explicitly
depends on being settled first — is now in question. §3.1's judgment half is answered in full and is
not affected by this finding (it concerns a different consumer of `seen_ids` than the one this
section found). §3.4's measurement is complete pending the background benchmark run's numbers, which
are independent of how §2's question resolves and are worth having either way.

**Requested from the architect:** a ruling on whether §2's finding is correct as traced, and if so,
whether it changes the shape of §3.2's conclusion (commit does not need tombstones) to something
narrower (commit needs tombstones *only* for node ids a rollback-draft has restored, a set that may
itself be much smaller than "every tombstone ever") or invalidates the clean split entirely. That
ruling is what determines whether §3.3 (a horizon-as-boundary-of-obligation mechanism) is worth
designing at all, and if so, what it would need to preserve.
