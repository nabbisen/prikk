# Amendment v2 — the stale citation your report found, and how to fix it

**Amends:** `ignore-documentation-handoff-v1.md`, whose work is **accepted and pushed** (`1bee0e4`,
CI 15/15, book deployed). **Base:** current `main`. **This is a one-line source-comment change.**

---

## 1. You found it; here is the ruling you asked for

Your report declined to edit `crates/prikk-store/src/ignore.rs:30`'s `main.rs:344` citation because
the handoff's scope was the page, `SUMMARY.md`, two cross-links and two help lines. **That was the
right call** — reporting a finding outside a stated scope beats quietly widening the diff. Fix it now.

**Confirmed independently.** `main.rs:344` is `Some(id) => println!("heads/main RefState: {id}")`.
The deferred-configuration comment is `ActivePatchThresholds`'s, now at `main.rs:389`.

**I surveyed how far it spread, because you found one instance and there are six.** Five are in
`rfcs/done/` and `rfcs/handoffs/` — **leave every one of them alone.** `reference/msrv.rs`'s own
module doc states this project's rule: those locations *"record what was true when they were written
and must never be bound to the current authority."* Correcting a historical record to match today is
a worse defect than the stale number.

**`crates/prikk-store/src/ignore.rs:30` is the only live site.**

## 2. Do not fix it by writing `main.rs:389`

That is the same defect with a fresher number, and it will drift again the next time anything is
inserted above it — which is exactly what happened here, most recently via AUD-10's argument hoisting.

**RFC 118: derive, never transcribe. Cite the thing, not its coordinates.** Name
`ActivePatchThresholds` and let a reader search for it; a symbol survives edits above it, a line
number does not.

**While you are in that sentence, check the rest of it.** Re-verify every citation in the paragraph
you touch, not only the one flagged — that is where a rewrite launders a second stale reference into
looking fresh.

## 3. What I fixed myself, so you do not duplicate it

Your second finding — `docs/src/guide/worktree-status.md`'s Claim-to-Source Anchors table still
pointing at `github.com/nabbisen/prikk` — **is fixed in my own commit alongside this amendment.** **Three**
anchors repointed to `prikk-vcs/prikk` — I first wrote "two", because `grep -c` counts matching
*lines* and one line carries two URLs. The assertion in my own edit script caught it. Worth
carrying: a count taken from `grep -c` is a line count, not an occurrence count.

**Do not extend that sweep.** Ten other tracked files carry the old URL and every one must keep it:
`CHANGELOG.md` and three RFCs under `accepted/`/`done/` are historical records, and **seven are
frozen oracle fixtures under `release/`** whose contents are pinned by the 57 `check` cases. Editing
those would fail the gate, and correctly.

## 4. Gates and reporting

Full set from `rfcs/EXECUTION-ORDER.md` §6 rule 9 against your final commit. **`mdbook build` does
not apply** — this changes no `docs/src/` page.

Local commit on `main`; **no push.** Report to `.git-exclude/review-request/`, stating what the
rewritten sentence now cites and the result of the paragraph-wide citation re-check.
