# RFC 122 — `worktree-status` runs on no repository this CLI produces

**Status.** **COMPLETE, 2026-09-01.** Rewired onto the replay baseline `commit` shares (`7a01168`),
CI conformance repaired (`e6882c7`), and the two stale claims the fix itself created corrected
(`bc443e8`). CI green on all 15 jobs, including both non-Linux read-only conformance runners.

**What shipped beyond the original scope**, both found in review rather than named in the RFC:
`main.rs`'s user-facing dirty-worktree message still said *"snapshot-baseline"* (the sweep had been
scoped to `docs/`, and the stale wording also lived in code), and `README.md` still advertised the
"one capability-gap caveat" that this work had just deleted. **And a queue owned by a different ref is
now surfaced** — a `queued_elsewhere` field and an output note warning that an "untracked" file may be
committed-but-unsealed work belonging to another ref, so a reader does not delete it. No verdict was
reclassified: `Untracked` is accurate relative to the ref being checked; the note adds the context.

**One reason it survived so long, recorded because it generalises:** the defect was known and written
down in three places (`ROADMAP.md:177`, the CI exclusion comment, `platform-support.md`) and fixed in
none — while `README.md` advertised the command with no caveat at all. Being recorded is not being
tracked.

Raised as **High** by the external architecture audit of 2026-08-31
(`audit-2026-08-31-task-1a.md` §3, task-2 V1); reproduced independently at `3a8d730`
(`.git-exclude/reviewed/external-audit-20260831-review-v1.md` §1.1).

**Tracks.** A functional defect on the mainline surface, plus the documentation that still advertises
the broken command.

---

## 1. Reproduction

Ordinary tutorial sequence — `init`, `trust maintainer add`, `commit`, `seal --allow-no-audit` — then:

```
$ prikk worktree-status
error: integrity error: checkout target for heads/main does not contain a snapshot blob
exit=1
```

**Every repository this CLI can create fails this way.** There is no sequence of supported commands
that produces a repository on which `worktree-status` succeeds.

## 2. Cause

`worktree_status.rs:13,157` routes through `prepare_snapshot_checkout_plan`, which requires a
`snapshot_blob_ref` on the checkout target. **Ordinary seals never write one** (`seal.rs:168`,
`seal_from_accepted.rs:224`). `commit` itself moved to the patch-replay baseline
(`patch_replay.rs:255`) and `worktree-status` did not follow.

So the command computes the same comparison `commit` performs — worktree against the ref's current
state — but asks for it through a baseline mechanism the product stopped emitting.

## 3. Why it survived

**It is not undetected. `ROADMAP.md:177` already records it:**

> **`worktree-status` cannot run** against any repository the CLI produces

— in the section explaining why editor and IDE integration is deferred. The audit's claim that it
"rotted undetected" is the one thing in that finding this project can correct.

What *is* undetected is narrower and worse:

- **CI does not run it.** It is absent from the read-only conformance job's command list and from
  dc67. A rewire could regress tomorrow with every gate green.
- **`README.md:256` lists it under Useful Commands with no caveat** — and, tellingly, it is absent
  from the README's own CI-verified read-only list a hundred lines above. A reader has no way to tell
  those two lists apart.
- **`docs/src/guide/worktree-status.md`** describes snapshot-backed baselines accurately, which
  technically covers the failure, and leaves no reader able to discover that ordinary repositories
  are not snapshot-backed.

## 4. Scope

1. **Rewire onto the replay baseline** — `resolve_worktree_baseline` / the replay manifest, the same
   comparison `commit` already performs, so one derivation backs both and they cannot diverge again.
2. **Reclassify the error.** `integrity error` is wrong: nothing is corrupt. An unsupported-state
   refusal is what this is, and the difference matters because `integrity error` is what this product
   says when a repository is damaged.
3. **Add it to the CI read-only conformance job** — the fix is worthless without the gate that keeps it.
4. **Correct the documentation sweep:** `README.md:256`, `docs/src/guide/worktree-status.md`, and
   `ROADMAP.md:177`, which becomes false the moment this lands. **Sweep for a fourth site rather than
   fixing these three** — this project has been bitten by range-bound correction lists twice recently.

## 5. Dependency worth naming

`ROADMAP.md`'s editor/IDE section gives three reasons integration is deferred: no current-branch
pointer, no `diff`, and this. **This RFC removes one of the three.** It does not unblock that theme —
the other two are real — but the roadmap prose must stop citing a reason that no longer holds.

## 6. Non-goals

No `diff` command. No pathspec filtering. No current-ref concept. This RFC restores a command that
already exists to the baseline the rest of the product uses, and nothing more.
