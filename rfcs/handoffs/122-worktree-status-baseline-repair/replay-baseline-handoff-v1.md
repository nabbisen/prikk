# `worktree-status` on the replay baseline — implementation handoff

**Authority:** `rfcs/proposed/122-worktree-status-baseline-repair.md`.
**Base:** current `main` (`7aef8b5`). **Under `003-landing-work-on-main.md`.**
**The repository moved to `prikk-vcs/prikk` on 2026-09-01 (RFC 129) — confirm your remote before you
start.**

**Severity: the only High-severity finding of the 2026-08-31 external audit that is a live product
defect.** `prikk worktree-status` fails on every repository this CLI can create.

---

## 1. Reproduce it first

Do this before reading further, so the rest of this handoff describes something you have seen:

```sh
mkdir wt && cd wt && prikk init
export PRIKK_AUTHOR_KEY_ID=dev-author PRIKK_AUTHOR_SEED=00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff
export PRIKK_MAINTAINER_KEY_ID=dev-maintainer PRIKK_MAINTAINER_SEED=111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000
prikk trust maintainer add --key-id dev-maintainer \
  --public-key a00899dfd3357aee69729405913f9324dfc033cec04a2215239eda64ae6d9d91
echo hello > readme.txt && prikk commit -m genesis && prikk seal --allow-no-audit
prikk worktree-status
```

```
error: integrity error: checkout target for heads/main does not contain a snapshot blob
```

## 2. Cause, traced

`worktree_status.rs:88` calls `prepare_snapshot_checkout_plan`, which refuses unless the target block
carries a `snapshot_blob_ref` (`checkout.rs:93-97`). **No CLI path ever sets one** — `seal.rs:168`
and `seal_from_accepted.rs:224` do not, and only a test helper does
(`worktree_status/tests.rs`, `publish_snapshot_block`). Meanwhile `commit` moved to the patch-replay
baseline (`patch_replay.rs:255`) and `worktree-status` never followed.

## 3. What to build

**Rewire `worktree_status` onto the same derivation `commit` uses.** `commit`'s path is
`node_authoring.rs:273` → `resolve_worktree_baseline(layout, ref_name)` →
`resolve_baseline_state(layout, &object_store, baseline_block, horizon)` → `NodeLifecycleState`.

**Requirement, and it is the point of the whole fix: one derivation backs both commands.** Do not
reimplement the baseline computation in `worktree_status.rs`. If a shared helper has to be extracted
to make that true, extract it — a second implementation that agrees today is the defect being fixed,
one release later.

**A design question I am not deciding for you, with my recommendation.** `commit` folds already-queued
unsealed patches onto the sealed baseline before authoring (DC-66, `node_authoring.rs:295-300`).
`worktree-status` must choose whether to do the same.

- **Fold the queue (recommended).** `worktree-status` then answers *"what would the next `commit`
  author?"*, which is the question a user is asking, and it agrees with `commit` by construction.
- Sealed baseline only. Answers *"what differs from published history?"* — defensible, but it makes
  `worktree-status` and `commit` disagree about the same worktree, which is this defect's own shape
  in a new place.

**If you take the recommendation, say so and show the queued-patch case in a test. If you take the
other, the report must say why** — this is exactly the kind of choice that should not be made
silently.

## 4. Reclassify the error

Whatever refusals remain must not be `PrikkError::Integrity`. **`integrity error:` is what this
product says when a repository is damaged**, and a user who reads it will reasonably reach for
`doctor`, backups, and this project's recovery references. An unsupported-state or
not-applicable refusal is a different thing and must read as one.

## 5. CI must run it — the fix is worthless without the gate

`.github/workflows/ci.yml:228-231` excludes `worktree-status` from the non-Linux read-only
conformance job, with a comment explaining exactly why. **Add it to that job and delete the
exclusion comment**, so the command is exercised on `windows-latest` and `macos-latest` against the
genesis-and-seal fixture — the same fixture shape `commit`/`seal` actually produce.

## 6. The documentation sweep — this is the hard half, and it is five sites, not one

RFC 122 §4 told you to sweep rather than fix a fixed list. I have since found the sites; **treat this
as the floor, not the ceiling, and say in your report what you searched.**

| Site | What it says now | After |
|---|---|---|
| `README.md:256` | Lists `prikk worktree-status [path] [--ref REF]` under Useful Commands **with no caveat** | The command works; nothing to caveat. Verify it now belongs in README:154-160's CI-verified read-only list too |
| `ROADMAP.md:177` | *"**`worktree-status` cannot run** against any repository the CLI produces"* — one of three reasons editor/IDE integration is deferred | Becomes false. **Remove that reason and leave the other two** (no current-branch pointer, no `diff`). Do not claim the theme is unblocked — it is not |
| `.github/workflows/ci.yml:228-231` | The exclusion comment | Gone with the exclusion (§5) |
| `docs/src/reference/platform-support.md:214` | Table row: *"currently unreachable against an ordinarily-authored repository"* | Plain `Read-only` |
| `docs/src/reference/platform-support.md:229-235` and `:243` | A full note tracing the defect to source, plus *"minus `worktree-status`, per the note above"* in the CI description | Both go. §243 must describe what CI actually runs after §5 |
| `docs/src/guide/worktree-status.md` | *"PR-018 adds read-only worktree status against **snapshot-backed baselines**"*, and *"compares the current worktree with the **snapshot manifest** referenced by the selected ref"* | Describes the replay baseline. **This page is in `DECLARED_DOCUMENTS` (`commands/tests.rs:57`)**, so RFC 118 §8 rule (A) checks its command names — run that gate |

**One thing you must not fix.** `platform-support.md:235` says the gap is *"recorded in
`MILESTONES.md`"*. **It is not** — `grep -i worktree MILESTONES.md` returns nothing. That citation is
false today and independent of your change. **`MILESTONES.md` is the owner's file and is not yours or
mine to edit**; delete the false citation from `platform-support.md` along with the rest of the note,
and **report the discrepancy** rather than trying to reconcile it.

## 7. Controls

1. **The §1 reproduction, before and after.** The exact command, on an ordinary
   `init`/`commit`/`seal` repository, failing at base and succeeding on your commit.
2. **A test that fails without the rewire.** Not a test that happens to pass — show it red first.
3. **The queue case** from §3: commit without sealing, then `worktree-status`, and assert the
   behaviour you chose, with the choice named.
4. **A modified/missing/untracked case each**, so the report is not merely "it does not error".
5. **The doc sweep as a result**: what you searched, what you found, and whether §6's five sites were
   all of them.
6. **`cargo test -p prikk --bin prikk commands`** after touching the guide page (§6).

## 8. Gates

The full set from `EXECUTION-ORDER.md` §6 rule 9 against your final commit, **clippy as a single
invocation per target with the exit code captured explicitly**. Cross-target clippy: check your own
diff for `#[cfg(target_os)]`/`#[cfg(unix)]`/`#[cfg(windows)]` rather than against this sentence.
`mdbook build` too — §6 touches four pages.

One commit on `main`, local, **no push, no tag**.

## 9. Scope discipline

**No `diff` command. No pathspec filtering. No current-ref or HEAD concept. No change to what `seal`
writes** — do not make seals emit snapshot blobs to satisfy the old code path; the snapshot baseline
is the thing being left behind, not restored.
