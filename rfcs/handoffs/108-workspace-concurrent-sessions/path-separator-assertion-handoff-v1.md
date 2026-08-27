# Path-separator assertions turned `main` red — URGENT

**Priority: ahead of everything else.** `main` is red at `256a011`; one job, two tests.
**Base:** `256a011`. **Under `003-landing-work-on-main.md`** — commit locally on `main`, do not push.

---

## 1. What is red

`Windows mutation test suite`, two failures:

```
doctor_reports_a_missing_refs_locks_directory_even_though_unlock_tolerates_it ... FAILED
doctor_reports_a_missing_refs_tmp_directory_even_though_verify_tolerates_it   ... FAILED

assertion failed: ... issue.message.contains("refs/tmp")
```

`doctor/tests.rs:82` and `:107`. The message is built from `dir.display()`, which renders `refs\locks`
on Windows; the literal says `refs/locks`; the substring never matches.

**Your work is not wrong. The expectation is.** The `assert!(!report.is_healthy())` on the preceding
line **passed on Windows** — `doctor` correctly flagged the missing directory. The check iterates
`required_directories()` and behaves correctly on every platform. **I verified the derivation covers
all sixteen entries, and the silent-hole control catches exactly your three tests and nothing else.**
Only these two string comparisons are platform-dependent.

Your third test passed because it asserts on the wrong-type code and never spells a path.

## 2. The fix

**Build the expected fragment from the layout instead of writing it as text**, so it renders with the
platform's own separator — `layout.refs_dir().join("locks")` is already in the test, two lines above,
as the thing being removed. **The path is right there; do not re-type it as a string.**

**This is the same reconstruct-versus-derive mistake this arc has now hit three times** — `wal.rs`
rebuilding a relative path by `format!`, `active_session_names` rebuilding one from a lossy `String`,
and now a test rebuilding one from a literal. **Derive it from the value that already exists.**

**Check every other assertion you added in this commit for the same shape**, not only the two that
failed. A path literal that happens to contain no separator, or one on a code path Windows CI does not
reach, will pass today and fail the next time something moves.

## 3. Scope

**Only the assertions.** Do not change `push_missing_required_directory_issues`, the message text, the
`fsutil` primitive, or either listing site — all four are correct and CI proves it on three platforms
apart from these two lines.

**Do not add a `#[cfg]` to these tests.** They should run everywhere; they were always meant to. The
defect is the expectation, not the platform.

## 4. Controls

1. **The two tests pass locally** and still fail if `push_missing_required_directory_issues` is
   disabled — **re-run that check after editing them.** An assertion loosened until it passes
   everywhere is worse than one that fails on Windows, and rewriting a `contains` is exactly how that
   happens.
2. **Full gate set against the exact final commit.**
3. **Per-job CI.** `Windows mutation test suite` is the one that matters; it is the only job that ran
   these tests and failed.

## 5. Report

To `.git-exclude/review-request/`, briefly. Include:
1. What you changed and how the expected value is now derived.
2. Your sweep of the commit's other assertions (§2) — including "none others found" if that is the
   answer.
3. Control 1's result — both tests passing, and both still failing with the check disabled.
4. The full gate set at the final commit.
5. Anything in this handoff that was wrong.

**A structural fix — a typed path field on `DoctorIssue`, or a convention that path assertions compare
`Path` values rather than substrings — is a real question and I have raised it with the owner. It is
not part of this fix. Get `main` green.**
