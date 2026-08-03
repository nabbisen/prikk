# DC-69 Lifecycle-State Retention - Handoff

**Cleared to start.** Accepted by the project owner on 2026-08-03, at
`rfcs/accepted/DC-69-LIFECYCLE-STATE-RETENTION.md`.
**Authored by** the architect, who discharged §3.1 and §3.2 at acceptance — §2 below.
**Size:** unknown, deliberately. The deliverable is **an answer**; a mechanism may or may not follow.
**Touches:** `prikk-replay`'s `NodeLifecycleState`, DC-64's persisted cache, and possibly nothing else.

## 1. What this increment is

**A design increment.** The question is: *does a prikk repository forget anything, ever?*

`seen_ids` and `latest_tombstone_by_id` grow with cumulative history without bound, and DC-64's binding
condition 1 requires `seen_ids` persisted complete on **every commit** — ~93 ms at 10,000 files, a cost
that does not shrink with the cache because it *is* the cache.

**"Unbounded growth is inherent to the model" is a permitted and respectable outcome**, on DC-64's route-(c)
precedent — provided it is *established*, and provided the consequence is then written where a user and the
roadmap can see it, not left implicit in a struct definition. Do not treat a mechanism as the success case.

## 2. §3.1 and §3.2 — discharged by the architect. Verify, do not redo

**§3.2 — does the commit path need tombstones? Evidence says no.**
`latest_tombstone_by_id`'s consumers are `node_lifecycle/validation.rs:33`, `query.rs:33-41`, and
`lifecycle_cache/incremental.rs:189` — the last being **DC-64 persisting them, not using them for a
decision**. The decisions that genuinely need tombstones — restoration equivalence and `NodeIdReuse` — are
in `patch_algebra`, reached from `merge_evidence.rs`, i.e. the **merge** path. Established in the DC-64
trust-ladder ruling.

**So the commit path appears to carry a structure it does not consult.** If that holds, the question splits:
commit's cache may not need tombstones at all, and the merge path's needs become a separate, later question.
**Confirm it yourself before relying on it.**

**§3.1 — is `seen_ids` load-bearing? The factual half is settled; the judgment half is the increment.**

Commit-path consumers of `contains_seen_node_id`: **exactly one**, the mint-collision guard at
`node_id_gen.rs:124`. The other two (`patch_algebra/preimage.rs:79`, `:232`) are merge-path.

Node ids are **256 bits** (`fill_node_id_bytes(&mut [u8; 32])`). **A fresh draw will never collide with
history by chance** — not "rarely", never, at any repository size this project will see. So the guard is
not defending against collision-by-chance.

**What it is defending against is the increment's central question.** Candidates: a degraded or stubbed
entropy source; a deterministic test generator escaping into production (`NodeIdGenerator` is injected,
which makes this concrete rather than theoretical); a future non-random id scheme.

Note `mint_fresh`'s shape: it draws, redraws **once** on collision, then fails closed. **That is the shape
of a sanity check, not of a collision-avoidance algorithm** — a real collision-avoidance loop would not give
up after two draws. Weigh that.

**If the guard defends only against broken entropy, ask whether checking the entropy source is a better
control than remembering every id ever minted.** That reframing is the most valuable thing this increment
could produce.

## 2a. §3.2's discharge is WITHDRAWN — ruled 2026-08-03

**Your trace is correct and I confirmed every link.** `create_node` (`mutation.rs:37-46`) requires
`latest_tombstone_by_id` for any `seen_ids` hit; `apply_state_effect` (`effect.rs:34,49`) calls it on the
ordinary commit path; `patch_inverse.rs:245-253` reuses the original `node_id`, which is the producer that
reaches that branch. Production-reachable.

**How I got it wrong:** I grepped for `latest_tombstone` consumers and explicitly filtered out
`mutation.rs`, then reported the narrowed search as the complete consumer set. Full ruling:
`.git-exclude/reviewed/prikk-dc69-tombstone-ruling-v1.md`.

**It narrows rather than destroys.** Three facts bound it, and they change §3.3:

1. **Tombstones are consumed, not accumulated.** `create_node` removes the tombstone on successful
   restoration, so `latest_tombstone_by_id` grows only with deletions **never** restored — unlike
   `seen_ids`, which only grows. **The two need separate answers; the RFC's pairing of them was mine and
   was wrong.**
2. The requirement is on **replay**, not storage: the tombstone must exist when the restoring create is
   applied, meaning the earlier `DeleteNode` must be replayable before it.
3. A DC-64 cache anchored **after** a restoration carries no dependency on the old tombstone.

**The invariant §3.3 must preserve:**

> **A horizon may not sever a `DeleteNode` from a later restoring `CreateFile` of the same node id.**

Checkable, not a vague hazard. Any boundary mechanism must keep enough to replay such pairs, or refuse to
place a boundary that splits one.

## 3. What remains for you

**§3.3 — can a horizon become a boundary of obligation?** `lineage_horizon_id` is already threaded
everywhere. Whether "before this point the repository keeps a **proof** rather than the material" is
coherent, and what it costs the verification claim, is the shape a mechanism would take.
**Do not design this before §3.1's judgment half is answered** — it may be unnecessary or impossible.

**§3.4 — measure the cost at realistic history.** Every measurement so far varies **file count** at a short
lineage. Nothing has measured **long history with a small tree**, which is the shape that isolates
cumulative cost. One benchmark axis, on DC-59/62's pattern.

## 4. Traps

- **Treating a mechanism as the success case.** An established "no bound exists, here is why, here is the
  consequence" closes this increment.
- **Quietly relaxing DC-64's binding condition 1.** Criterion 5 exists because this is the obvious place to
  undo a safety condition another increment was told to hold. Preserve it, or renegotiate it explicitly
  with reasoning — never silently.
- **Truncating `seen_ids` before answering what the guard defends.** That is the change-of-safety-posture-
  disguised-as-optimisation the DC-64 ruling already named.
- **Designing the horizon boundary first** because it is the interesting part.
- **Answering only for commit.** If §3.2 holds, say so; if it does not, the merge path is in scope.

## 5. Definition of done

§3.1's judgment half and §3.3/§3.4 answered and reported; **a stated answer to "does prikk forget?"**
recorded where a user and the roadmap can see it; if a mechanism is proposed, what a verifier can still
check after material is dropped, stated explicitly; if route (c), the evidence that makes it a finding;
DC-64's condition 1 preserved or explicitly renegotiated; full gate set per `rfcs/EXECUTION-ORDER.md` §6
rule 9 with test counts before and after, **commands verbatim**.

## 6. Standing request

Five increments here were redesigned because implementation found what design review missed, and three
consecutive defects were found by running sequences rather than by inspection. **This increment is mostly
reading and thinking, which is the mode in which those misses happen most easily.** If something here
contradicts what the code actually does — including anything in §2 — stop and report it.
