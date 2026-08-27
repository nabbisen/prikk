# Recovery-surface directory listings: tolerate absence, report it explicitly, order deterministically

**Base:** `ae26862` or later `main`. **Under `003-landing-work-on-main.md`** — commit locally on
`main`, do not push, do not tag.

**On the siting of this handoff:** this is not RFC 108 work. It was found by RFC 108 increment 2, it
touches the same file increment 3 will touch (`unlock.rs`), and `rfcs/handoffs/`'s naming gate
requires a numbered directory while this fix has no governing RFC. **Filed here deliberately rather
than creating an unbacked number** — the gate's own module doc calls the one existing unbacked
directory (`consolidation`) a process gap, and repeating it would be worse than an imperfect
neighbour. **I have flagged the underlying problem to the owner separately.**

---

## 1. The defect, and what makes it two defects of different severity

Two production sites call `list_directory` on a required directory **without guarding for its
absence and without ordering the result**. Both are diagnostic or recovery surfaces — the code paths
that exist for damaged repositories.

**Site 1 — `unlock.rs:156`, `refs/locks`. No containment; the command dies.** Measured:

```
refs/locks   | unlock: ERR i/o error: directory is absent: refs/locks | verify: Ok(report)
refs/tmp     | unlock: Ok(0 locks)                                    | verify: Ok(report)
active       | unlock: Ok(0 locks)                                    | verify: Ok(report)
containers   | unlock: Ok(0 locks)                                    | verify: Ok(report)
trust        | unlock: Ok(0 locks)                                    | verify: Ok(report)
cache        | unlock: Ok(0 locks)                                    | verify: Ok(report)
```

**`prikk unlock` fails outright on a repository missing `refs/locks`** — the command whose entire
purpose is clearing a wedged lock, defeated by one missing directory. **This needs no second active
session and no future increment. It is live at `main` today.**

**Site 2 — `refs/verify.rs:274`, `refs/tmp`, inside `candidate_issues`.** The table above shows
`verify` returning `Ok(report)` — **do not read that as "unaffected."** `verify_refs` is invoked as
`pipeline.run(VerificationStage::Refs, verify_refs(layout))`, and `run` catches `Err` into
`StageOutcome { status: Failed { message } }` rather than propagating it. So the Refs stage fails with
`i/o error: directory is absent: refs/tmp`, and **under `stop_on_first_error` it sets `halted_by`,
halting every downstream stage.**

**Contained, but wrong in both directions:** the message describes an I/O failure rather than
diagnosing repository damage, and a missing debris-scan directory should not be able to stop the rest
of verification.

**Establish this yourself.** I inferred the halt from reading `run`; I did not observe it. If it turns
out `stop_on_first_error` is not set on this path, say so — that changes site 2's severity and I would
rather be corrected than have it repeated.

## 2. The fact that reframes the whole fix

**`required_directories()` is consulted only at `init`.** `layout.rs:635` says so in its own words,
and the consumer sweep confirms it: `init`, plus tests. **Nothing verifies the required directories
after a repository is created.**

**So today's failure is not a check. It is an accident** — a directory read that happens to blow up,
in the wrong place, with the wrong message, fatally on one surface and stage-halting on the other.

**This is why the fix cannot be only "tolerate absence."** If both sites simply return empty, a
repository missing `refs/locks` reports *"no locks are held"* — confidently, and wrongly — and nothing
anywhere says the directory is gone. **That converts a loud failure into a silent one, which is
strictly worse and is the opposite of this project's rule that absence must be explicit.**

## 3. What to build

**Both halves land in the same commit.** Shipping the tolerance without the explicit report is the
regression described above; I will not accept them separately.

### 3.1 Tolerance and ordering at both sites

The pattern already exists twice in-repo: `verify/objects.rs:360-373` and `layout.rs`'s
`active_session_names` — `inspect_entry` three-way match (`None` → empty, `Directory` → proceed,
`Some(_)` → `Integrity`), then `list_directory`, then a sort on `name.as_encoded_bytes()`.

**You will be writing the third and fourth copies of that pattern. Adjudicate whether it should be
one helper instead**, and justify either answer. RFC 118's principle is that one declaration should
have one home and consumers should read it — but a helper that fits three call sites badly is worse
than four honest copies. **The criterion is whether all four want identical behaviour, including the
`Some(_)` arm's error text, which is currently site-specific.**

### 3.2 Ordering is a deliberate output change, unlike last time

Increment 2's handoff forbade changing existing output order. **This one requires it**, because
arbitrary order is the defect. `unlock`'s per-ref locks and `verify`'s candidate-debris issues will
both come back in a stable sequence. **State it plainly in the report as an intended behaviour
change**, and check whether any existing test asserts on the current arbitrary order — if one does,
that assertion may change, and this is the one increment where that is expected rather than a
stop-and-report trigger.

### 3.3 The missing directory must be reported by something

**This is the design question and it is yours to propose, not mine to hand you.** Once the listings
tolerate absence, some surface must say "a required directory is missing" — otherwise §2's silent
hole opens.

Candidates, and the criterion is which surface a person actually consults when a repository is
damaged, not which is easiest to reach:
- **`doctor`**, which already reports repository health issues with remediation advice
- **a `verify` stage**, which already has an outcome vocabulary for failures
- **the listing sites themselves**, which is where the information is but not where a reader looks

**Whatever you choose, `required_directories()` must be the source of the list.** Do not hand-write a
second inventory of directory names — that is the transcription RFC 118 exists to prevent, and this
project has found that failure repeatedly.

**If you conclude the reporting belongs in a surface this increment should not touch, stop and report
rather than shipping half.**

## 4. Out of scope

- **Repair.** Recreating a missing directory is a mutation and belongs with `doctor`'s repair path,
  which is already increment 3's subject. Detect and report here; do not fix the repository.
- **The other four `list_directory` callers.** `verify/objects.rs` (both) is already guarded and
  sorted; `worktree_files.rs:41` walks the worktree, not `.prikk`, and is a different question;
  `layout.rs:471` is increment 2's own, already correct. **If you find one of these is not as I have
  described it, that is a finding — report it.**
- **RFC 108 increment 3.** Unchanged and still queued behind this.

## 5. Controls

1. **Site 1, before and after.** `prikk unlock` on a repository missing `refs/locks` — currently
   `ERR i/o error: directory is absent: refs/locks`, afterwards a successful listing **plus** the
   explicit missing-directory report from §3.3.
2. **Site 2, before and after** — the Refs stage's own outcome, quoted, both ways. **Include the
   `stop_on_first_error` finding from §1.**
3. **Ordering is load-bearing.** Remove only the sort and show the test failing. **Increment 2 taught
   this the hard way: a sort test can pass on a filesystem that happens to return sorted order, so a
   test that passes without the sort is not a control.**
4. **The silent-hole control — the important one.** Prove that a repository missing a required
   directory cannot come back as a clean report from any surface you touched. **A gate that reports
   success over an unreadable read is the failure mode this whole increment exists to remove; do not
   reintroduce it while removing it.**
5. **Full gate set against the exact final commit**, per EXECUTION-ORDER §6 rule 9.
6. **Per-job cross-platform CI.** This is directory-listing behaviour on three filesystems.
   **Increment 2 went red on macOS for exactly this class of assumption** — a test that plants or
   removes a path is filesystem-dependent whether or not it carries a `cfg`. Name CI as unavailable
   locally rather than claiming it.

## 6. The report

To `.git-exclude/review-request/`. Include §3.1's adjudication, §3.2's order-change disclosure and
any assertion it moved, §3.3's proposal with its reasoning, all six controls quoted, the full gate set,
and **anything in this handoff that was wrong** — including the §1 inference I explicitly did not
verify.
