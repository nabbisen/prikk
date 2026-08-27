# RFC 108 increment 3c — generalise active ref-name metadata, close 3b's recorded gap

**Authority:** RFC 108 §D3, ACCEPTED 2026-08-27. **Base:** `3268f6b` or later `main`.
**Under `003-landing-work-on-main.md`** — commit locally on `main`, do not push, do not tag.

**This is not per-active repair.** That is 3d. §1 says why, and it is the same reason that ordered
every prior split in this arc.

---

## 1. Scope, and why repair still waits

3b closed the WAL-replay half of doctor's per-active reporting and **recorded, in code, the half it
could not close**: a non-default active whose WAL has records but whose ref-name metadata is missing
or malformed is invisible. Measured then, and it is still true:

```
target=default   healthy=false   ["PRIKK-DOCTOR-ACTIVE-REF-METADATA-MISSING"]
target=second    healthy=true    []
```

**The blocker is real and is why 3b did not close it:** `read_active_ref_metadata(layout)` takes no
name. It reads `default_active_ref_name_path()` unconditionally, and no generalised accessor exists.

**Readers before writers, one more time.** 3d makes repair per-active and gates it per-active; a
repair that acts on one active must be able to see *that active's* metadata problems. **Building the
reader first is what makes 3d's gate able to say anything true.**

## 2. The change

### 2.1 The accessor, and the shape you must adjudicate

`read_active_ref_metadata` is **`pub`, re-exported from `lib.rs`, with roughly ten production call
sites** across `prikk-cli` (`main.rs`, `seal.rs`) and `prikk-store` (`rollback_draft.rs`, `verify.rs`,
`refs/evidence.rs`, `node_authoring.rs`, `active.rs`). **Recount these; the figure is mine and
approximate.**

**Two shapes, and increment 1 already ran this experiment:**
- **Add a `name` parameter to the existing function** — a public API break, rippling to every caller
  and owing a CHANGELOG entry.
- **Add a parameterised function and reimplement the existing one as the `default` wrapper** —
  exactly what increment 1 did with `default_active_dir`/`default_queue_wal_path`, which took **zero
  call-site edits**.

**I expect the second and am not ruling it** — argue whichever you take. **If you take the first, say
what the API break buys**, because a break here is a real cost to a published crate.

`layout` needs the matching path accessor: generalise `default_active_ref_name_path` the same way
`active_session_dir` was generalised, **accepting the same `impl AsRef<Path>` name type 3a
established** so an `OsString` from `active_session_names()` reaches it without a lossy step.

### 2.2 Read only — and say so rather than generalising by symmetry

**Generalise the read path only.** `write_active_ref_metadata` and `remove_active_ref_metadata` are
mutation surfaces with no caller that needs a non-default name yet.

**Symmetry is not a reason.** This project has a dead-surface-consolidation history, and a
generalised writer nothing calls is dead surface the day it lands. **Record in code that write and
remove stay `default`-only, and what would justify generalising them** — the same discipline 3b used
for the gap you are now closing.

### 2.3 Doctor closes the gap

`push_non_default_active_session_wal_issues` gains the ref-name-metadata check for non-default
actives, reaching parity with what `verification.active_wal_metadata_status` gives `default`.

**Do not re-derive `default`'s coverage.** 3b's boundary holds: `default` keeps its existing path.

**Delete the recorded gap when you close it.** A record that outlives the gap it describes is the
staleness-by-omission class this project has repeatedly had to clean up. **The doc comment must end up
describing what is true after this commit**, not carrying a fixed problem forward.

## 3. What must not change

- **On-disk layout.** `init` still creates `active/default/` only.
- **No mutation goes plural.** `repair_repository` still repairs one WAL, still gates on whole-repository
  health. **3d.**
- **`default`'s existing metadata reporting** — same code, same severity, same message.
- **No existing test assertion changes its expected value.** If one must, behaviour moved — stop and
  report.

## 4. Controls

1. **The measurement above flips.** A non-default active with WAL records and absent ref-name metadata
   is reported; quote the before and after. **Prove the test fails without the change.**
2. **Malformed, not just missing.** `default` distinguishes `MissingForNonEmptyWal` from
   `InvalidForNonEmptyWal`. **Establish what parity means for a non-default active and defend it** —
   matching both arms, or matching one and recording why.
3. **An empty non-default WAL with no metadata stays quiet.** The condition is *records present,
   metadata absent*. **A check that fires on a healthy second active is worse than no check**, and it
   is the easy mistake here.
4. **Zero call-site edits, or an exact count with the reason** (§2.1).
5. **Full gate set against the exact final commit.**
6. **Per-job cross-platform CI.** If you add a test that plants a non-UTF-8 name, **gate it on the
   property it depends on, re-derived for this diff** — `linux` when a name reaches the filesystem,
   `unix` when it does not. This arc has got that wrong in both directions; do not take either prior
   answer as a default.

## 5. The report

To `.git-exclude/review-request/`. Include §2.1's and §2.2's adjudications with reasoning, §4.2's
parity decision, all six controls quoted, the full gate set, the recounted call-site figure, and
**anything in this handoff that was wrong.**

## 6. What 3d will carry, so you can see where this is going

**Per-active repair, gated per-active.** Measured at `a8157cc` and still true: an unreadable WAL in a
second active makes `repair` refuse to truncate `default`'s repairable tail — **one workspace's
corruption wedges every other workspace's recovery**, which §D3.3 forbids.

**That gate fix needs something this increment does not build: a way to attribute a `DoctorIssue` to
an active session**, so repository-wide errors can block everything while per-active errors block only
their own active. **Do not build it here** — but if you see the shape it should take while working in
`doctor.rs`, say so in the report. **It is the third time this arc's design has been improved by the
implementer noticing something the handoff did not.**
