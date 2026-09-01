# Amendment to `replay-baseline-handoff-v1.md` — two stale claims the fix created, and one observation

**v1 stands in full. `7a01168` stays as it is — nothing in it is reverted.**
**Architect review of `.git-exclude/review-request/worktree-status-baseline-repair-report-v1.md`,
2026-09-01, against `7a01168`.**

---

## 1. What I verified independently, and what held

**The requirement this increment existed to meet is met, and I checked it by reading the diff rather
than the report.** `node_authoring.rs`'s inline sequence is genuinely **deleted**, not left beside a
copy: `resolve_worktree_baseline` + `resolve_baseline_state` + `apply_queued_patch_envelopes` are
gone from that file, its imports narrowed to `resolve_folded_worktree_baseline`, and
`baseline_state` dropped its `mut`. There is exactly one place this sequence is written, and both
commands call it.

**The subtlest claim in the report is the one I most wanted to disprove, and it holds.**
`resolve_folded_worktree_baseline` deliberately does *not* use
`require_active_ref_for_non_empty_wal`'s refuse-outright behaviour, which is a behaviour-preservation
claim about `commit`. Verified by ordering: `author_inner` calls that stricter check at
`node_authoring.rs:258`, **before** the shared call at `:274`, and it errors on all three
non-owning shapes — `Valid(other)` → `LockConflict`, `Missing` and `Invalid` → `Integrity`
(`active.rs:227-236`). So for `commit` a non-empty queue is always already known to belong to this
ref, and the shared function's own ownership check is the always-true re-confirmation the report
says it is. **`commit`'s behaviour is unchanged.**

Reproduced live at `7a01168`: the §1 failure is gone; clean, modified, untracked and missing all
report correctly; and **the fold is real** — commit without sealing, then `worktree-status`, gives
`tracked files: 2` with the unsealed file counted as tracked and unchanged.

**Three other things I checked and found correct, so they are not findings:**

- **The cross-target claim.** My own `grep` over the diff for `cfg(target_os|unix|windows)` returns
  one hit, and it is **prose** — a README line quoting `#[cfg(target_os = "linux")]`. The report's
  "no platform cfgs in this diff" is right.
- **§4's pushback on reclassification.** Correct, and better than what v1 §4 asked for. The
  artificial refusal is not reclassified, it is **gone**; what remains are genuine-damage refusals
  (`Integrity` for a torn WAL tail, a damaged record, ambiguous active-ref metadata), which are
  correctly classified because the repository really is damaged in those cases.
- **The sixth doc site.** `docs/src/guide/status.md:23` was a real find beyond v1 §6's five, and
  reporting it rather than folding it into "the five already covered" is exactly right.

Gates re-run here against `7a01168`: fmt clean, clippy single invocation exit 0 / 0 warnings,
**1429/1429**, `git diff --check` clean, boundary/reference `valid: true`, 57/57.

## 2. Required — two claims this fix made false

**Both are in text the commit itself touched or invalidated, and both are the sweep's own class.**

### 2.1 `crates/prikk-cli/src/main.rs:545` — the message a user actually sees

```rust
Err("worktree has snapshot-baseline changes".to_string())
```

**This is printed on every dirty worktree.** Reproduced:

```
$ echo changed > readme.txt && prikk worktree-status
error: worktree has snapshot-baseline changes
worktree-status repository: …
  modified readme.txt — tracked file bytes differ from the baseline
```

The command no longer uses a snapshot baseline. **This is the single most visible piece of the stale
wording the increment removed everywhere else** — the report lists `commands.rs` and
`output/worktree.rs` as swept for exactly this phrase, and the string that reaches the user survived
in `main.rs`.

**Why it was missed, and it is worth naming because it generalises:** the report's §5 control 5
describes a broad pass for `"snapshot baseline"`/`"snapshot-backed"` phrasing, and its two reported
hits (`rollback-preview.md`, `checkout/checkout.md`) are both documentation. **The broad pass was
scoped to docs; the stale wording also lived in `crates/`.** A sweep for user-facing wording has to
cover the code that constructs it.

**I ran that sweep across the whole tree and this is the only live site.** Every other survivor is
correct and must not be touched: `rollback_preview.rs`, `rollback_draft.rs`, `patch_replay.rs:136`,
`output.rs:257` and `rollback-preview.md` describe the rollback-preview feature, which **is**
genuinely snapshot-based; `worktree_patch/tests.rs:178`'s `publish_snapshot_baseline` supports the
`snapshot_only_baseline_fails_closed` test that must keep existing; `CHANGELOG.md` and `rfcs/` are
dated records.

**While you are in that line:** `error:` for a dirty worktree is a questionable classification, but
it is pre-existing and RFC 121 owns the exit-code and error-surface work. **Change the wording only.**

### 2.2 `README.md:159` — the caveat this commit deleted is still advertised

The sentence you edited to add `worktree-status` reads:

> the full, durable list, **including one capability-gap caveat**, is in the [platform support
> reference](./docs/src/reference/platform-support.md)

**That caveat was `worktree-status`, and this commit removed it** — correctly — from
`platform-support.md` (the table row and the whole explanatory note). `grep -i "capability.gap"`
now matches **only this README line**; there is no such caveat left to read.

So the clause is false, and it became false inside the sentence being edited. Remove it, or replace
it with what is actually true of that reference now.

**This is the class of error worth being deliberate about:** a fix that deletes a documented
limitation must sweep for the places that *advertise* the limitation, not only the place that
*states* it.

## 3. Required — the sweep, once more, as a result

You searched for the command's own name and for snapshot phrasing in `docs/`. **Extend both to
`crates/` and `README.md` and report what you searched**, since §2.1 and §2.2 are both outside the
first sweep's range. If a third site exists, that is the finding — name it rather than fixing it
silently.

## 4. Observation — a queue owned by another ref is silent, and "untracked" invites deletion

**Not a defect in what v1 asked for, and the report's reasoning is sound as far as it goes.** I am
raising it because I reproduced the user-visible consequence and it is data-loss-adjacent.

`resolve_folded_worktree_baseline` skips folding when the active queue belongs to a different ref —
correct for the *baseline*: another ref's queue is genuinely not part of this ref's state. But
**nothing in the report says so**, and the queued file then appears as ordinary `untracked`:

```
$ prikk status
queued patches: 1 targeting heads/main
$ prikk worktree-status --ref heads/feature
tracked files: 1   untracked files: 1
  untracked b.txt — worktree file is not in the baseline
```

`b.txt` is **committed but unsealed work**, and this command calls it untracked with no
qualification. A user who reads "untracked" as "stray file" can delete queued work. `prikk status`
carries the fact, so the information exists — it just is not where the user is looking.

**Required, with the shape left to you:** when the active WAL is non-empty and owned by a different
ref, `worktree-status` must say so. A field on `WorktreeStatusReport` plus one output line is the
obvious shape; deciding it in the report rather than in the CLI keeps the fact derived once. **Do
not reclassify the file** — `untracked` is accurate relative to this ref's baseline, and changing
that would be wrong. This is about naming the context, not relabelling the change.

## 5. Controls

1. **§2.1 reproduced before and after** — the dirty-worktree message, quoted from the real binary.
2. **§2.2 quoted before and after**, with `grep -i "capability.gap"` shown returning nothing
   afterwards.
3. **§3's extended sweep as a result** — what you searched, across which trees, and what you found.
4. **§4 demonstrated end to end**: a queue on one ref, `worktree-status` on another, showing the new
   signal. And a test, so it cannot regress silently.
5. **`commit` unchanged**: re-run the full `worktree_patch::tests` suite, as the first report did.

## 6. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9 against the final commit, **clippy as a single
invocation per target with the exit code captured explicitly**, plus `mdbook build`. Cross-target
clippy judged from your own diff.

One commit on `main`, local, **no push, no tag**. **RFC 122 closes when this lands.**

---

## 7. Added after CI went red — the §5 CI step failed on both non-Linux runners

**`15ecd2f` failed CI**: `non-linux read-only conformance` red on **macos-latest and
windows-latest**. I have restored `main` to green myself rather than leave it broken; the diagnosis
below is the part you need.

### 7.1 `worktree-status` is right; the CI step was wrong

```
=== prikk worktree-status ===
error: worktree has snapshot-baseline changes
  modified readme.txt — tracked file bytes differ from the baseline
##[error]Process completed with exit code 1.
```

**Reproduced locally by rebuilding the fixture from `ci.yml`'s own authoring script.** The fixture's
worktree is **deliberately ahead of `heads/main`**: the `heads/docs` commit appends a third line to
`readme.txt` and is never merged back. `heads/main` has two lines, the worktree has three. So
`worktree-status` reports one modified file **correctly** — this is not a defect in the rewire.

**The step was the defect.** Every other command in that set exits 0 on success;
`worktree-status` signals **findings** with exit 1 — RFC 121's ruled contract makes 1 cover findings,
not only failure. Running it bare under `bash -e` fails the job for working exactly as designed.

**This generalises, and it is the part worth carrying forward: a conformance harness that treats
every non-zero exit as breakage cannot contain a command that reports findings by exit code.** RFC
121 ruled the contract without anyone noticing this consequence, and v1 §5 told you to add the
command to that job without saying how. **Both omissions are mine, not yours.**

### 7.2 The tolerant shell form is not available here — I tried it and a gate refused

I first wrote the obvious fix: capture the exit code, accept 0 or 1, reject anything else, and still
require a real report on stdout. It works (verified against the rebuilt fixture, including a negative
control). **`reference-check` then rejected the workflow** with ten
`unclassified-procedure-command` / `unsupported-dynamic-command-head` errors: `command_scan`'s
procedure lexer has no shell-keyword awareness, so `if`, `$( )` and `||` are unclassifiable in a
scanned file. **The gate was right and I had forgotten its own documented limitation.**

### 7.3 What is on `main` now

```
../target/debug/prikk worktree-status --ref heads/docs
```

`heads/docs`'s sealed tip is exactly what the worktree holds, so this exits **0**, stays a bare
command the scanner accepts, exercises the full baseline derivation and worktree comparison end to
end, and additionally proves a closed ref is still readable. A comment at the site records both
constraints so nobody "simplifies" it back and breaks CI again.

**This is a workaround shaped by a lexer limitation, and you may have a better idea.** If you do,
propose it in your report rather than changing it silently — and note that widening `command_scan`
to understand shell keywords is its own increment, not this one.

### 7.4 What this adds to your controls

**Control 6, new and non-negotiable: report per-job CI status for your final commit**, naming the
two non-Linux conformance jobs explicitly. v1's controls asked for the CI *change* and not for
evidence that it passes, which is exactly how this reached `main` red.

