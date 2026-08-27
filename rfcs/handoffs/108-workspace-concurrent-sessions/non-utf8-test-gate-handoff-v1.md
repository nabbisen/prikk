# RFC 108 increment 2 — narrow the non-UTF-8 test's platform gate — URGENT, `main` is red

**Priority: ahead of everything else in this arc.** `main` is red at `ce3a52a`; one job, one test.
**Base:** `ce3a52a`. **Under `003-landing-work-on-main.md`** — commit locally on `main`, do not push.

---

## 1. What is red, and exactly why

`macOS mutation test suite`, one failure out of 788:

```
---- unlock::tests::list_held_locks_reports_a_lock_under_a_non_utf8_session_name stdout ----
Error: Io("Illegal byte sequence (os error 92)")
```

**`EILSEQ`, raised while building the fixture** — `create_dir_all` on the `b"bad\xFFname"` directory.
**APFS refused to create the directory.** The test never reached its assertion.

**Everything else is green, including the part that could not be checked locally:**

```
Windows mutation test suite:
  test unlock::tests::..._non_utf8_session_name ... ok
```

**Your `#[cfg(windows)]` variant ran and passed on NTFS with the unpaired surrogate.** Your reasoning
there was right, and the `OsString` fix is confirmed working on both Linux and Windows.

**The production fix is not in question. Do not revert it.** Only the `#[cfg(unix)]` gate is wrong.

## 2. The actual defect in the gate

**`unix` is an OS family; the test needs a filesystem property.** Gating on `unix` asserts that every
unix filesystem will hold any byte sequence as a name. Linux/ext4 will. macOS/APFS will not — it
enforces UTF-8.

**Note what this means about cfg gates generally, because it is the reusable part:** this diff *has*
platform gates, and they are still wrong. The project's standing caution is that the *absence* of
`#[cfg(target_os)]` does not prove portability. **This is its mirror: the presence of a gate proves
nothing either, unless the gate names the property the code actually depends on.** Neither the host
clippy run nor the `x86_64-apple-darwin` cross-target clippy could catch this — both passed. A
cross-compile proves the code builds for a target; it says nothing about what that target's
filesystem accepts.

## 3. What to do

**Narrow the gate so the test runs where the byte sequence is constructible, and make the exclusion
explicit.** The shape is yours to adjudicate, but two constraints are not negotiable:

1. **No silent skip.** A test that quietly returns early when the filesystem refuses will rot into
   one that never runs anywhere and still reports green. If you choose a runtime-detect-and-skip
   approach over a narrower `cfg`, **the skip must be visible in the test output**, and you must say
   how a reader would ever notice it stopped running.
2. **The exclusion must be documented at the test, with its evidence.** Not "macOS is different" — the
   specific fact, that APFS returned `EILSEQ` for this name in CI run `33037284343`, so the directory
   cannot exist and this defect is unreachable on that filesystem by this mechanism.

**Consider whether the `#[cfg(windows)]` variant's gate has the same weakness.** It passed on
`windows-latest`/NTFS. Does it assert something about ReFS or a network filesystem that you have not
established? **If you cannot establish it, say so rather than widening or narrowing on a guess** —
that is what you did correctly with the Windows variant the first time.

## 4. The product fact worth recording

Your test discovered something real, and it should not be lost as merely a CI failure:

**The silent-lock-drop defect is reachable on Linux (proven locally) and on Windows (proven in CI),
and on APFS it is unreachable by this mechanism because the name cannot exist.** That is a portability
fact established by running, not by reasoning, and it belongs in the code near the test or the
enumeration — wherever a future reader would look before widening the gate back.

## 5. Gates and report

Full gate set against the exact final commit, per EXECUTION-ORDER §6 rule 9, **including the
cross-target clippy runs** — and state plainly in the report that they are not evidence for this
class of problem, so nobody later mistakes their green for coverage.

Report to `.git-exclude/review-request/`, briefly — this is a narrow fix. Include:
1. The gate you chose and why, against §3's two constraints.
2. Your answer on the `#[cfg(windows)]` gate (§3), including "not established" if that is the answer.
3. Where you recorded §4's fact.
4. The full gate set at the final commit.
5. Anything in this handoff that was wrong.

**I will push and read per-job CI. `main` does not go green until that job does.**
