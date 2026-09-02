# Amendment to `ignore-mechanism-handoff-v1.md` — the design is accepted; what must survive the re-land

**Written after reading `ignore-mechanism-report-v1.md` in full.** v2 was written from the CI failure
alone, before the report reached me — it is correct about the defect and says less than it should
about everything else.

**v2's required fix is unchanged. This adds what must not be lost while fixing it.**

---

## 1. The design is accepted. Do not re-derive it.

Every §4 decision is answered, and the answers are right. **Re-landing means fixing one path
conversion, not reopening any of these:**

- **The enumeration is complete and better evidenced than I asked for.** You classified every
  `read_dir`/`list_directory` in `prikk-store` by which root it walks and showed exactly two touch
  the live worktree — and that `checkout.rs`, `patch_checkout.rs` and `snapshot.rs` contain no walk
  at all. **That last part makes §4.1's constraint hold by construction rather than by a check**,
  which is a stronger result than the handoff asked for.
- **`.prikkignore` as an ordinary tracked file** — minimal, no new authoring path, no "is this file
  special" branch outside `ignore.rs`. Right call, stated.
- **Malformed file → exit `1`, not `2`.** Correct reading of RFC 121 §1: `2` is for what argument
  parsing rejects before repository work begins, and worktree content is not that.
- **`non-goals.md` checked and correctly declined**, with that page's own scope definition as the
  reason — deferred work is not a permanent refusal. Checking and declining with a reason is worth
  more than the edit would have been.

## 2. What must survive the fix — directory-level pruning is functional, not an optimization

Your decision 4 contains the finding I most want preserved, and it is the one a rewrite would
casually drop:

> `commit`'s walk fails closed on any symlink or unsupported entry kind it meets, and a real
> `node_modules/` is typically full of both — so without directory-level pruning, ignoring it would
> not actually let a real project commit at all.

**That is the difference between this increment solving the audit's problem and appearing to.** A
per-file ignore check that still descends into an ignored directory would pass every test you wrote
and fail on the first real `node_modules`.

**The path fix touches `walk_dir`, which is exactly where the pruning lives.** If the fix restructures
that function, **state in your report that a pruned directory is still never opened**, and keep a test
that proves it — an ignored directory containing an entry kind the walk refuses.

## 3. What v2 requires, restated in one line

The repo path must be built with `worktree_status.rs:274`'s `pathbuf_to_slash_string`-shaped
component-wise conversion, not `Path::to_str()`; audit every other `Path`→repo-path conversion in the
diff the same way; and add a Linux-runnable unit test asserting no produced repo path contains `\`.

## 4. Not required, worth a sentence in the report

`.prikkignore` is itself tracked and therefore travels in bundles and `sync`. That is conventional
and follows from decision 3, and it means a receiver's own future commits are shaped by the sender's
ignore file. **Harmless for applying patches** — the rule binds at discovery only, which control 4
demonstrates. **Say it out loud in the module doc anyway**, because it is the kind of consequence a
reader deserves to meet where the decision is recorded rather than deduce later.

## 5. Process

**The revert was my doing, not a verdict on this work.** I pushed both your commits without reading
them, CI went red on Windows, and reverting the mechanism was the fastest way back to green — see v2
§1. The disclosure commit is on `main` and stays.

Re-land as one commit on top of the revert. **CI is my control and I will run it this time.**
