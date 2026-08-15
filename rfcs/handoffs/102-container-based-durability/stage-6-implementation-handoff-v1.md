# RFC 102, Stage 6 — Implementation Handoff v1

**Authorized by the project owner 2026-08-15.** Design: `design-v1.md` **§15**, and §2's container rules.
**This is the last stage of RFC 102.**

Stage 5 merged at `87b5085` with green three-platform CI. Criterion 2 is closed for the repository.

## 1. Stage 6 is not what its name suggests — read §15.1 first

§7 recorded this stage as three words: *"Stage 6 — compaction."* Scoping it found the same problem
Stage 5 had, and worse.

**The A/B slots this stage was supposed to use are allocated on every container that does not need
compacting, and on none of the three that do.** §15.1 has the table. In brief: object containers are
content-addressed and immutable with no GC anywhere in the workspace, so they accumulate nothing; the ref
log must keep its history (DC-38, DC-69); and the real garbage is in **`ref_pointer_index`,
`received_index`, and `trust_policy_container`** — all last-entry-wins, all single-name, none with a B
slot.

**Your first job is to disprove that table.** It is the entire basis for this scope and it contradicts
the RFC's own §3.2. Derive it from the code, not from §15.1's prose. If any container is misclassified,
stop and report — the stage changes shape again.

## 2. Two steps, and the split is the point

**Step 1 — the generation resolver, with no compaction at all.**

Route every container access through one function that resolves which slot is live. It returns `A`
unconditionally, because the generation log is empty. **No behaviour change, no format bump, nothing
destroyed.** Sixteen non-test sites hardcode `ContainerSlot::A` today; only `index.rs:330` resolves from
data (`entry.slot`).

**Step 2 — compaction**, which by then only writes the new slot and appends a generation record. §4
already specifies the publish mechanism: *readers take the last complete generation record*.

**Why this order:** step 1 carries the whole "every reader must agree on which slot is live" problem —
the part touching sixteen call sites — into a step that **cannot lose data**. Step 2 is then small enough
to review closely, which is what §15.3's risk deserves. Step 1 merges before step 2 is written.

## 3. The corruption ruling — §15.3, and it is not negotiable

**Compaction is the first operation in this RFC that destroys data.** Every prior stage appended, or
truncated a file whose content was already dead.

§3's read path names a corrupt record and continues — **but the record stays on disk and stays
recoverable.** A compactor built the obvious way (read through the resync reader, write back what it
yields) omits those records from the new slot and abandons the old one. **Corruption becomes permanent
deletion, through the mechanism designed to survive corruption, and the operation reports success.**

**Compaction refuses to run on any container with a known-corrupt record.** No compacting around damage,
no repair, no partial progress. A refusal is recoverable; a deletion is not.

## 4. What must not change

- **The ref log container is not compacted.** DC-38's audit trail is its purpose, `scan.rs` validates
  `update_seq` against record order, and DC-69 ruled route (c) — *prikk does not forget*. There is no
  retention horizon to compact against, and inventing one is not this stage's decision.
- **The trust key container is not compacted.** `trust.rs:77` — TOFU history persists across removal,
  asserted by `a_changed_key_under_a_removed_and_readded_id_is_still_refused`.
- **Object and ref-log A/B slots stay allocated and unused.** They are forward reservations, not the
  `refs/by-id`/`refs/logs`/`refs/tmp` dead-allocation case. Do not retire them as a side effect.
- **Criterion 2 stays closed.** No new `atomic_replace` on any path, and no new name created outside
  `init`.
- **DC-95's classification**, on every path this stage touches.

## 5. Step 0 — report before any production code

§15.5's five items. In particular items 2–4: what a generation record contains (per-container-type or
global), what triggers compaction (no trigger exists today, and an automatic destructive operation needs
an argument rather than a default), and whether the format bump is one or two.

Four stages running have each found something in Step 0 that would have been expensive later. Stage 5's
found that its own name was wrong.

## 6. Acceptance criteria

1. **§15.1's table confirmed or corrected**, from the code.
2. **Step 1 changes no behaviour** — proven, not asserted.
3. **No container name created after `init`** — enumeration, extended to any new B slots.
4. **No durability-bearing write uses `atomic_replace`** — criterion 2 stays closed for the repository.
5. **Compaction refuses on a corrupt container**, demonstrated the way DC-95 proved things: damage a
   record, observe the refusal, restore.
6. **A crash between writing the new slot and appending the generation record leaves the old generation
   authoritative** — shown, not argued.
7. **The ref log and trust key containers are untouched by compaction**, with a test that would fail if
   a later change started compacting them.
8. **DC-41-grade recoverability re-earned** at the new state count (§9 criterion 5).
9. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus **green three-platform CI**.
10. **`docs/src/reference/` reflects what this stage ships.**

## 7. Standing

- **Work on a branch.** Branch → push → green CI → merge.
- **Report counts before and after** per rule 10. Baseline as of `87b5085`: `prikk-store` **703**,
  `prikk-object` 80, `prikk-replay` 44, `prikk-hash` 14, `prikk-crypto` 7, `prikk-release-policy` 83;
  **179 locked packages**. **Update that line in the same commit when you change it** — Stage 5 moved it
  by 15 across six rounds and no round updated it, including under my review.
- **Measure with rule 10's command.** The bare substring `0 filtered out` also matches `690 filtered
  out`; the separator is load-bearing.
- A stop-and-report remains a complete outcome. Stage 5 produced four of them and each was right.
