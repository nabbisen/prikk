# DC-83 Test Temp-Directory Uniqueness — Handoff v1

**Cleared to start immediately** — small, test-only. Accepted 2026-08-09,
`rfcs/done/DC-83-TEST-TEMP-DIR-UNIQUENESS.md`. **Ahead of DC-80.**

## 1. The defect, already diagnosed

`crates/prikk-cli/tests/format_transition.rs:26-33`'s `unique_root()` builds its temp directory from a
**nanosecond timestamp alone**. Parallel tests in the same binary can collide, share a repository, and
the loser fails with `LockConflict` on `.prikk/active/default/active.lock`.

**Confirmed nondeterministic:** the identical commit failed on `main`, then passed on re-run with no code
change.

**`prikk-store` already has the right pattern** — `test_support.rs:226-235`'s `unique_temp_dir` uses
`process::id()` **plus** a monotonic counter. **Reuse it rather than invent a second one.** If the crate
boundary prevents sharing, mirror it exactly and say why.

## 2. Two things beyond the one-line fix

**Audit for siblings.** Any other helper deriving a path from a timestamp alone has the same defect.
**Report what you find** — and fix only what genuinely matches this pattern, not everything that looks
adjacent.

**Do not make it resilient.** No retry, no serialization, no tolerating a collision. **The fix is
uniqueness.** A helper that retries would silently absorb the next instance of this bug, which is worse
than the bug.

## 3. Why this is an increment and not a drive-by

**A flaky gate is worse than a failing one.** Every acceptance here rests on "gates pass", and several
recent increments rest specifically on macOS CI being green — including work I accepted on that basis. A
nondeterministic failure trains all of us to re-run and carry on, which is precisely how a real
regression gets waved through. That is why it goes ahead of DC-80.

## 4. Bar

All tests pass unchanged on both toolchains, macOS job green, **no production code touched**, gates per
rule 9 as amended.
