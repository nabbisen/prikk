# RFC (accepted) - DC-83 Test Temp-Directory Uniqueness

**Status.** **ACCEPTED by the architect 2026-08-09** under delegated quality authority. Test-only,
no production code, no format, no dependency. **Independence.** Author-reviewed — the standing ceiling.
**Arises from.** A flaky CI failure on `main` at `a8edd13`, diagnosed during DC-79's review.
**Target.** 0.20.0, ahead of DC-80 — a flaky gate must not sit under increments whose evidence *is* CI.

## 1. The defect

`crates/prikk-cli/tests/format_transition.rs:26-33`'s `unique_root()` derives its temp directory from a
**nanosecond timestamp alone**:

```rust
let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
let root = env::temp_dir().join(format!("prikk-format-transition-{nonce}"));
```

Cargo runs tests in one binary **in parallel threads**. Two tests entering this function closely enough
receive the same nonce, share a directory, and the second fails:

```
LockConflict("active lock already exists: …/prikk-format-transition-1786259469157365000/.prikk/active/default/active.lock")
```

**`prikk-store` already solves this correctly** — `test_support.rs:226-235`'s `unique_temp_dir` uses
`process::id()` **and** a monotonic counter. `format_transition.rs` predates or missed that pattern.

## 2. Why it surfaced now — mechanism inferred, defect verified

**Verified:** the helper is insufficiently unique, and the failure is nondeterministic — the identical
commit failed, then passed on re-run with no code change.

**Inferred, not verified:** it surfaced on macOS because that job is new (DC-81 added it, so
`format_transition.rs` only began running there days ago) and because macOS's `SystemTime` granularity is
commonly coarser than Linux's, making identical nonces far likelier. **I cannot test that hypothesis
without macOS hardware, and it is not load-bearing** — the fix is correct regardless of why the
collision window is wider on one platform.

## 3. Why this is an increment rather than a drive-by fix

**A flaky gate is worse than a failing one.** Every acceptance in this project rests on "gates pass",
and several recent increments rest specifically on *macOS CI being green*. A test that fails
nondeterministically trains everyone — me included — to re-run and move on, which is exactly how a real
regression gets waved through. **It must be fixed before increments whose only evidence is CI.**

## 4. Acceptance criteria

1. `format_transition.rs`'s temp-root helper is collision-proof under parallel execution — **reuse
   `prikk-store`'s existing pattern rather than inventing a second one**; if it cannot be shared across
   the crate boundary, say so and mirror it exactly.
2. **Audit for siblings.** Any other test helper deriving a path from a timestamp alone is the same
   defect; **report what you find**, and fix only what is genuinely this pattern.
3. All tests pass unchanged, both toolchains, and the macOS job green.
4. **No production code touched.** Test infrastructure only.
5. Gate set per `EXECUTION-ORDER.md` §6 rule 9 as amended.

## 5. Non-goals

Retry logic, test serialization, or any mechanism that *tolerates* collisions rather than preventing
them. **The fix is uniqueness, not resilience** — a helper that retries would hide the next instance.
