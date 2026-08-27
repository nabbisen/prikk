# RFC 108 increment 3d — per-active repair, gated per-active, with §D3.3's independence demonstrated

**Authority:** RFC 108 §D3.3, ACCEPTED 2026-08-27. **Base:** `952ac5c` or later `main`.
**Under `003-landing-work-on-main.md`** — commit locally on `main`, do not push, do not tag.

**This is the writer, and the last one in the arc.** Everything before it made readers plural so this
increment's gate could say something true.

---

## 1. What is broken today, measured, and getting worse

§D3.3 requires: *"show a Workspace's WAL recovering independently of every other."* **At `952ac5c` the
opposite is true**, and I re-measured it after 3c landed:

```
second=healthy                -> repair of default: Ok(truncated 8)
second=unreadable-wal         -> repair of default: REFUSED
second=missing-ref-metadata   -> repair of default: REFUSED
```

**One workspace's corruption wedges every other workspace's recovery.** When I first measured this at
`a8157cc` there was one such path; **3c's ref-metadata check added a second.** Every reader increment
that improved diagnostics cost recovery capability, because `repair_repository` gates on
`!before.is_healthy()` — undifferentiated whole-repository health.

**The severities are not the bug.** An unreadable WAL is a real error. **The gate's shape is the bug.**

## 2. Two rulings, because they are safety invariants rather than preferences

### 2.1 Never hold two active locks at once

Repair acquires an active's lock **only while repairing that active**, and releases it before moving
to the next. `ActiveLock` releases on `Drop`, so a loop body gives this for free.

**Three reasons, and the first is the requirement:** a wedged lock on B must not prevent repairing A,
which is precisely §D3.3. Holding N locks simultaneously reintroduces the coupling this increment
exists to remove. It also removes any lock-ordering deadlock question against a concurrent writer.

**A lock that cannot be acquired must not fail the run.** Report that active as skipped, with the
reason, and **continue to the others.** An operator recovering a damaged repository must not be told
"nothing could be repaired" because one workspace is busy.

### 2.2 Independence must hold in both directions

The obvious implementation attributes this arc's new per-active issues and leaves everything else
repository-wide. **That fixes one direction and leaves the mirror image broken:** `default`'s own
active-scoped issues — `PRIKK-DOCTOR-ACTIVE-REF-METADATA-MISSING` and the rest of
`add_active_wal_metadata_issues`, plus `default`'s WAL-replay stage failures — would stay
repository-wide, so **a problem in `default` would block repairing `second`.**

**Attribute `default`'s active-scoped issues to `default`.** They are exactly as active-scoped as the
non-default ones; only their code names differ.

**What stays repository-wide is everything genuinely repository-wide** — `push_missing_required_directory_issues`,
`ensure_no_incomplete_publication`, format mismatches, object and ref verification. **Those must still
block everything, and a repair that proceeds over them would be a worse bug than the one you are
fixing.**

## 3. The change

### 3.1 Attribution

Build the shape you proposed in 3c's report §6:

> add `pub active_session: Option<OsString>` to `DoctorIssue` … existing `::error`/`::warning`/`::info`
> call sites default to `None` — repository-wide, blocking everything, exactly today's behaviour.

**The default is what makes this safe**: every issue not deliberately attributed keeps today's meaning.
**Adjudicate the constructor shape** — a builder step, a parallel constructor, or something else — and
justify it against the fact that most call sites must not change.

### 3.2 The gate

`repair_repository` partitions `before`'s issues: any `Error` with `active_session: None` refuses the
whole run; an `Error` with `Some(name)` refuses **only that active's** repair.

**Name the refusals in the report** rather than silently repairing a subset. An operator must be able
to see that `second` was not repaired and why.

### 3.3 Per-active repair and the report

Repair iterates `layout.active_session_names()` — sorted, so the order is deterministic — and repairs
each eligible active under its own lock.

`DoctorRepairReport.wal_repair` is a scalar and `DoctorRepairReport` is `pub`, with two production
consumers in `prikk-cli/src/main.rs` and three test references. **Adjudicate whether to add a
per-active field or change the existing one**, against this project's aversion to gratuitous public
API breaks — and note that a scalar named `wal_repair` describing only `default` is at least honest,
where a scalar silently summing several actives would not be. **Recount those references; the figures
are mine.**

**CLI:** `--repair-wal-tail` keeps its meaning — repair what is repairable. **Do not add an active
selector**; there is still no way to create a second active, and a selector is user-facing surface
this arc has deferred throughout.

## 4. What must not change

- **On-disk layout.** `init` still creates `active/default/` only.
- **Repository-wide refusals.** §2.2's second half. A repair proceeding over genuine repository damage
  is the one regression that would make this increment net-negative.
- **`truncate_trailing_partial`'s own refusal** on a damaged record (`"WAL has a damaged record; repair
  does not modify it"`) — per-active repair must not weaken it.
- **`default`'s repair behaviour on a healthy repository** — same bytes truncated, same records
  preserved, same patch ids reported.

## 5. Controls

1. **§1's measurement inverts.** All three rows, quoted before and after. `second=unreadable-wal` and
   `second=missing-ref-metadata` must both become `Ok(truncated 8)` for `default`.
2. **The mirror direction** (§2.2). `default` broken, `second` repairable → `second` still repairs.
   **This is the control most likely to be missing**, because the bug it catches is invisible if you
   only test the direction §1 names.
3. **§D3.3's demonstration, as the RFC words it.** Two actives, each with its own trailing partial;
   repair; **each recovers its own records and neither touches the other's.** Assert on preserved
   record identity, not only counts — DC-66's own reasoning applies: "N records preserved" does not say
   whose work survived.
4. **A held lock on one active does not fail the run** (§2.1). Acquire `second`'s lock, run repair,
   and prove `default` still repairs and `second` is reported as skipped with a reason.
5. **Repository-wide damage still refuses everything.** Remove a required directory; repair must refuse
   entirely, not repair a subset.
6. **Every existing repair test still passes unchanged.** If an expectation must move, behaviour moved
   where §4 says it must not — **stop and report.**
7. **Full gate set against the exact final commit.**
8. **Per-job cross-platform CI.** This is mutation under locks on three filesystems. **Re-derive any
   platform gate for this diff** rather than inheriting an answer from a previous round.

## 6. The report

To `.git-exclude/review-request/`. Include §3.1's and §3.3's adjudications with reasoning, all eight
controls quoted, the recounted consumer figures, the full gate set, and **anything in this handoff that
was wrong.**

**Escalate rather than proceed if:** attributing `default`'s active-scoped issues turns out to change
behaviour §4 protects; or the per-active gate cannot distinguish repository-wide from active-scoped
errors without a second classification of the kind this project forbids.

## 7. After this

**3d completes RFC 108's mechanism.** `active/<name>/` is fully general, every diagnostic surface is
plural-correct, and recovery is independent.

**Nothing user-facing exists and nothing creates a second workspace** — §D5 leaves naming and CLI
surface, whether a Workspace may be shared, and the RFC 109 interaction deliberately unsettled. **Those
are the owner's calls, not the next increment.**
