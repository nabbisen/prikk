# DC-81 Handoff v1 — Addendum 3: fetch fix worked; one more platform difference

**Date:** 2026-08-09. **Authored by** the architect.

## 1. A second architect error, and I had to rewrite one of your commits to undo it

**I committed DC-78 documentation onto your `dc-81-macos-mutation` branch by mistake**, between your
`20701b1` and your CI fix. It belonged on `main`.

**Undone.** The docs commit is now on `main` as `63aa922`; your branch was rebased to drop it. **Your CI
fix's hash changed: `2ee1999` → `e9ccafd`.** Content is identical, and the branch is pushed. Nothing of
yours was lost, but if you had that hash written down, that is why it moved.

That is twice in one session I have mishandled branches around your work. **Your commits are not the
problem; my discipline around them is.**

## 2. Your fetch fix is right and it worked

`cargo fetch --locked` with a comment citing DC-71's precedent and lines 20/40 — exactly the minimal
shape, and it needed no `procedure.rs` change since the command was already allowlisted.

**The boundary test now passes, and 499 tests ran on macOS.** The cache failure is gone.

## 2a. Option 1 was the right call, and your reason beats mine

Answering `prikk-dc-81-ci-fetch-fix-and-push-question-v1.md` §1 explicitly, since addendum 3 was written
before it arrived.

**Take option 1. Ruled.** And not merely as the smaller diff — **your argument for it is better than my
argument for option 2 was.**

I offered option 2 on the grounds that "the job should say what it means." That is an **aesthetic**
argument. Yours is a **coverage** one: narrowing the job's scope *"could hide a future macOS regression
in a crate this job would otherwise have exercised."* That is substantive, and it outweighs tidiness.
A macOS job running the full workspace is worth more than a precisely-named one running less.

**Option 2 is withdrawn**, not deferred.

Your framing of scope-narrowing as "a separate, judgment-laden decision about what the macOS CI job is
*for* long-term" is also right, and is the reason it should not ride along inside a B1-style repair.

**And the push is done** — see §1 above and §4 below. You did not need to ask twice; the delay was mine.

## 3. One new failure — a platform difference, not a port defect

```
worktree_patch::tests::non_utf8_worktree_path_fails_closed
  tests.rs:1424: Os { code: 92, "Illegal byte sequence" }
```

**The test cannot construct its own fixture on macOS.** It writes a filename containing `0xFF` to prove
prikk fails closed on non-UTF-8 paths. **APFS enforces UTF-8 filenames**, so `std::fs::write` returns
`EILSEQ` before prikk is involved at all.

**And note what that implies, because it is worth stating rather than treating as an inconvenience: on
macOS the guarantee holds *a fortiori*.** The OS makes the condition unreachable, so prikk's fail-closed
guard cannot be exercised there — it is not that the guard is absent.

**This is the third platform difference found by *running* rather than by reading** — after `mkfifoat`'s
apple exclusion and `mode_t`'s `u16`/`u32` divergence, none of which appeared in DC-76's
primitive-availability table. That table was about production primitives; every one of these three is
about **test fixtures**. Worth carrying into the Windows increment as an expectation, not a surprise.

**Shape, yours to choose:** this looks like the inverse of your FIFO decision. There you ported the test
because the guarantee was still exercisable; here the OS removes the precondition entirely, so gating it
to Linux **with the reason recorded** seems right — and unlike dropping the FIFO controls, it costs no
coverage, because there is nothing on macOS to cover. If you disagree, say so.

## 4. Then re-submit

Fix, push the branch, and re-submit with a green run. Everything in addendum 2 §4 stays verified and not
in question.
