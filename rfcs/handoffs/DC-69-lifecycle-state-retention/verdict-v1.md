# DC-69 Verdict — §3.3 and "Does Prikk Forget?"

Companion to `prerequisite-questions-v1.md` (§3.1 judgment, §3.4 measurement, the §3.2 finding) and
the architect's ruling (`.git-exclude/reviewed/prikk-dc69-tombstone-ruling-v1.md`), which is
authoritative on §3.2: withdrawn as originally discharged, narrowed to a checkable invariant. This
document answers §3.3 under that invariant and states the increment's final verdict (criterion 2).

## 1. The ruling's invariant, restated as the design constraint

> A horizon may not sever a `DeleteNode` from a later restoring `CreateFile` of the same node id.

Three facts the ruling establishes, load-bearing for everything below:

1. **`seen_ids` and `latest_tombstone_by_id` have different retention characters.** `seen_ids` only
   ever grows. Tombstones self-prune on successful restoration
   (`mutation.rs:50`, `self.latest_tombstone_by_id.remove(&node_id)`) — a tombstone survives only
   for a deletion never (yet) restored. Treating them as one pair, as the RFC originally did, is too
   coarse.
2. **The requirement is on replay, not on final storage.** A tombstone must exist *at the moment the
   restoring create is replayed* — not in some permanent retained state.
3. **A cached predecessor taken after a restoration carries no dependency on the tombstone that
   restoration consumed.** DC-64's incremental cache holds post-fold state; once a delete/restore
   pair has been folded into a persisted predecessor, that specific tombstone is gone from the state
   and nothing forward-looking needs it again — until the *next* deletion of that same id creates a
   new one.

## 2. §3.3 — can a horizon become a boundary of obligation?

**In principle, yes. Not today, and not as a decision this increment can make alone**, because two
conditions the mechanism would depend on are not present, and both belong to other surfaces:

**Condition A — `rollback-draft`'s reach would need to be bounded to match the horizon.**
`prepare_patch_inverse_plan` (`patch_inverse.rs:94-100`) walks `single_parent_chain` — the *entire*
lineage back to the last snapshot or genesis, with no depth limit. A restoration can legitimately
target a deletion from anywhere in a repository's history. A horizon that drops tombstones older
than itself is only safe if nothing before it can still be restored — and today, nothing prevents a
user from requesting exactly that at any time. Making a horizon safe therefore requires *also*
teaching `rollback-draft` to refuse (fail closed, not silently) any restoration whose target predates
the horizon — which is the ruling's §4 second option ("refuse to place a boundary that would split a
pair") applied at the point the split would otherwise occur. That is a change to `rollback-draft`'s
own contract — what it promises a user it can undo — not a retention detail; it deserves its own
review, the same way DC-64's own trust-ladder question was not decided inside DC-64.

**Condition B — full replay would need to trust the horizon as a checkpoint, not walk past it.**
Fact 3 above means a *warm* incremental cache does not need old tombstones. But full replay is still
reachable: DC-64's `REANCHOR_BOUND = 64` forces a full replay every 64 incremental steps by design,
and cache absence, corruption, or a horizon change do the same as a fallback. Full replay walks the
*entire* lineage from genesis — every delete/restore pair in the whole history, not just recent ones.
A horizon cannot be safe against this path unless full replay itself is redefined to start from the
horizon (trusting it as a checkpoint) rather than genesis. That redefinition is exactly the
"proof vs. material" trust question the RFC poses in §3.3's own wording — and it is the same class of
question DC-64's trust-ladder ruling already drew a boundary around once (`ComparedLifecycleCache`
requires a full authoritative replay for any identity-bearing decision). Redefining what full replay
trusts is not a retention mechanism; it is a change to this project's one closest-to-a-root-of-trust
argument, and it needs the review that argument has always gotten, not a decision folded into a
retention increment.

**§3.3's answer:** the mechanism has a coherent shape — a horizon paired with a corresponding bound
on `rollback-draft`'s reach, plus a redefinition of what full replay may trust — but building it
requires decisions on two other surfaces this increment does not own. Proposing it without those
decisions would violate criterion 3 (a mechanism must say explicitly what a verifier can still check
after material is dropped) and criterion 5 (DC-64's condition 1 must be preserved or *explicitly*
renegotiated with reasoning) — there is no reasoning available yet for either, because the two
conditions above have not been decided. **Not designing it further here is the correct application
of the ruling's own invariant, not a stall.**

## 3. Verdict — does prikk forget?

**No, not today, for either structure, and for a specific, established reason in each case — not
because retention was never considered.**

**`seen_ids`.** Its one *protective* purpose — the mint-collision guard — is not load-bearing against
any currently-reachable threat (§3.1, accepted by the ruling): a test generator escaping to
production is closed by `#[cfg(test)]`-gating at compile time, degraded entropy is closed by
`getrandom`'s own fail-closed contract, and a future non-random id scheme would be poorly detected by
this check regardless (its firing probability scales with `seen_ids`'s size, weakest exactly when a
new threat would be newest). **But `seen_ids` has a second, load-bearing role the RFC did not
originally name**: it gates whether `create_node` even attempts a restoration-equivalence check at
all (`mutation.rs:37`, `if self.seen_ids.contains(&node_id)`). Because that gate must correctly
recognize *every* id a future rollback-draft might restore, and rollback-draft's reach is unbounded
(§2 above), `seen_ids` cannot be bounded independently of the tombstone question — even though the
guard it was originally justified by turns out not to need it.

**`latest_tombstone_by_id`.** Grows only with deletions never restored — a real, bounded-relative-to-
churn property the RFC's original framing missed by pairing it with `seen_ids`. But for the
deletions that remain unrestored, their tombstones are required at replay time for as long as a
future restoration might target them (unbounded, per §2 Condition A) and for as long as full replay
might walk back to them (not eliminated, per §2 Condition B). Neither bound exists today.

**This is route (c)** — DC-64's precedent for a permitted, respectable outcome — established rather
than assumed: traced through `create_node`'s restoration-equivalence gate
(`mutation.rs:37-46`), `patch_inverse.rs`'s deliberate reuse of the original node id when inverting a
`DeleteNode` (`patch_inverse.rs:245-253`), and `prepare_patch_inverse_plan`'s unbounded lineage walk;
confirmed load-bearing in production by the architect's independent verification of the same trace.
**The consequence, measured, not estimated:** Axis D (`axis-d-benchmark-report-v1.md`) shows
cumulative-history cost growing roughly linearly independent of tree size — 2.66 ms at 10 sealed
generations to 17.91 ms at 200, with live tree size fixed at 20 files throughout. A repository with a
decade of churn does not get a slow commit because its tree is large; it gets one because its history
is long, and that cost has no ceiling under the current design.

## 4. DC-64's binding condition 1

**Preserved, unchanged, not renegotiated.** `seen_ids` persisted complete on every commit remains
required — this document does not propose, and this increment does not make, any change to what
DC-64's incremental cache persists or validates. The dependency this document traces
(`create_node`'s restoration-equivalence gate) is, if anything, an *additional* reason condition 1
must hold as stated: dropping `seen_ids` completeness would not only reopen DC-64's original
persisted-state-validation concern, it would also let a future restoring `CreateFile` silently skip
the equivalence check entirely for an id the cache had forgotten, which is a correctness break, not
merely a performance one.

## 5. What a future increment would need to answer, if this cost becomes a real problem

Recorded so the next reader does not have to re-derive it: bounding `rollback-draft`'s reach (its own
increment — changes what users are told they can undo, a product decision, not a retention detail)
and redefining what full replay may trust below a horizon (an extension of DC-64's trust-ladder
argument — changes this project's closest-to-a-root-of-trust claim, needing the same review that
argument has always required). Both would need to land, coupled, before a horizon mechanism could be
proposed safely. Neither is decided by this document.

## 6. What did not change

No production code. `NodeLifecycleState`, `create_node`, `mint_fresh`, DC-64's incremental cache,
and `rollback-draft`'s inverse-planning are all unchanged from before this increment — this is a
design increment whose deliverable is the answer above, per the RFC's own §2. The one code addition
across the whole increment is `axis_d_long_history_small_tree`, a `#[ignore]`d benchmark instrument
(`crates/prikk-cli/tests/dc59_commit_benchmark.rs`), already committed at `c17268f` and independently
reproduced on a clean checkout.
