# RFC (proposed) - DC-69 Lifecycle-State Retention

**Status.** **Proposed 2026-08-03.** Awaits owner acceptance.
**Authored by** the architect.
**Independence.** Author-reviewed — the standing ceiling. Compensated by §3's prerequisites being
answerable from the repository and by §2 permitting "no change" as an outcome.
**Numbering note.** DC-68 was used by a release-authority RFC that was created and reverted on 2026-08-02
(`97d269b`, reverted `deac3fd`). That number is left retired to avoid ambiguity in git history.
**Arises from** the architect's concepts note §2, 2026-08-02, and DC-64's binding condition 1.
**Requirement.** None names this. That is part of the finding.

## 1. The problem, measured

`NodeLifecycleState` (`crates/prikk-replay/src/node_lifecycle/types.rs:55-56`) carries:

- `seen_ids: BTreeSet<NodeId>` — **every node id ever minted**, alive or not
- `latest_tombstone_by_id: BTreeMap<NodeId, Tombstone>` — **every node ever deleted**

Neither is bounded by the current tree. Both grow with **cumulative history, forever**. DC-64's binding
condition 1 requires `seen_ids` to be persisted **complete and never truncated**, so every commit now
loads, validates, and rewrites a structure proportional to everything that has ever happened.

Measured at 10,000 files: `load` ~58 ms, `persist` ~29 ms, `from_replay` ~5.4 ms — **~93 ms per commit**,
and that portion does not shrink with DC-64's cache because it *is* the cache.

**This was recorded as a performance finding. That framing is too small.** It is an **architectural
ceiling**: a repository with a decade of churn does not have a slow commit, it has a commit whose cost
nobody has bounded. And it collides with the project's own positioning — a system of record that *cannot
forget* is what an auditor wants and what a long-lived engineering repository cannot afford. That tension
is currently unnamed anywhere.

## 2. What this increment is, and what it is permitted to conclude

**This is a design increment.** Its deliverable is an answer, not necessarily a mechanism.

**"Unbounded growth is inherent to the model" is a permitted and respectable outcome**, on DC-64's route-(c)
precedent — provided it is *established* rather than assumed, and provided the consequence is then stated
where users and the roadmap can see it, rather than left implicit in a struct definition.

## 3. What must be established before any design — blocking

The pattern that has caught four bad designs in this program. **The first question is the one that decides
the increment.**

### 3.1 Is `seen_ids` load-bearing, or belt-and-braces?

Its only production consumer on the commit path is the mint-collision guard
(`node_id_gen.rs:124`, `contains_seen_node_id`). But node ids are **256 bits of entropy**
(`fill_node_id_bytes(&mut [u8; 32])`), so a fresh draw will never collide with history by chance.

**So what is the guard actually defending against?** Candidates: a degraded or stubbed entropy source, a
deterministic test generator escaping into production, or a future non-random id scheme. Each implies a
different retention answer:

- If it defends only against **broken entropy**, then a bounded structure — or a different check entirely,
  e.g. verifying the entropy source rather than the output — may preserve the property at bounded cost.
- If anything **relies on `seen_ids` being complete for a correctness decision**, retention cannot touch it
  and the honest answer is likely route (c).

**This has never been asked.** I noted it while ruling on DC-64's trust question and did not follow it.

### 3.2 What is `latest_tombstone_by_id` for, and who needs it?

Its consumers are `validation.rs`, `query.rs`, and DC-64's cache persistence. The **restoration-equivalence
and `NodeIdReuse` decisions that need tombstones live in `patch_algebra`, reached from the *merge* path**,
not from commit — established in the DC-64 trust-ladder ruling.

**So the commit path may be carrying a structure it does not use.** Establish whether commit needs
tombstones at all. If not, the retention question splits cleanly: commit's cache need not carry them, and
the merge path's needs are a separate, later question.

### 3.3 Can a horizon become a boundary of obligation?

`lineage_horizon_id` is already threaded everywhere. Whether it can mean "before this point the repository
keeps a **proof** rather than the material" — and what that costs the verification claim — is the shape a
mechanism would take if one is warranted. **Do not design it before 3.1 and 3.2 are answered**; they may
make it unnecessary or impossible.

### 3.4 What does this cost at realistic history sizes?

Every measurement so far varies **file count** at a short lineage. Nothing has measured a repository with
long history and a **small tree** — which is the shape that isolates cumulative cost. **One benchmark axis.**

## 4. Acceptance criteria

1. §3's four questions answered and reported **before** any mechanism is proposed.
2. **A stated answer to "does prikk forget?"** — recorded in a place a user and the roadmap can see, not
   only in an RFC. Either bound exists, or it is stated that none does.
3. If a mechanism is proposed: it must not weaken the mint guard's actual property, whatever 3.1 finds it
   to be, and must say explicitly what a verifier can still check after material is dropped.
4. If route (c): the evidence that makes it a finding, plus the consequence stated per criterion 2.
5. DC-64's binding condition 1 is either **preserved or explicitly renegotiated with reasoning** — it is
   not silently relaxed by a retention mechanism.
6. Full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9, plus test counts before and after.

**Criterion 5 exists because this increment is the obvious place to quietly undo a safety condition** that
another increment was told to hold.

## 5. Non-goals

- DC-64's residual per-commit cost as a performance target. Related, separately tracked, not this.
- Merge-path tombstone needs, unless 3.2 shows they cannot be separated.
- Repository-format change. If the answer requires one, that is a finding to report.
