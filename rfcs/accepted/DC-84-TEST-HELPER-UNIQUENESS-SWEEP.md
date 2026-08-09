# RFC (accepted) - DC-84 Test Helper Uniqueness Sweep

**Status.** **ACCEPTED by the architect 2026-08-09** under delegated quality authority. Test-only.
**Independence.** Author-reviewed — the standing ceiling.
**Arises from.** DC-83's §2 finding, which the architect asked for and which turned out to implicate the
very pattern the DC-83 handoff cited as correct. **Target.** 0.20.0, not urgent.

## 1. What DC-83 established

`prikk-store`'s `unique_temp_dir` (`test_support.rs:226-235`) is `process::id()` +
`monotonic_suffix()`, and **`monotonic_suffix()` (`:237-242`) is just
`SystemTime::now().duration_since(UNIX_EPOCH).as_nanos()`** — no counter, despite the name.

`process::id()` is constant across every thread of one process, so **it cannot disambiguate two racing
threads.** The pair therefore closes only *cross-process* collisions, not the same-process thread race
DC-83 fixed.

**Measured, by DC-83:** 64 threads on a barrier, 128,000 samples — **214 collisions** on bare
nanoseconds, **zero** with an `AtomicU64` counter. This is not theoretical.

**Scope of exposure:** `unique_temp_dir` backs **580 `prikk-store` tests**, and thirteen further helpers
across `prikk-cli`'s test suites share the PID-plus-timestamp shape.

## 2. Scope

1. **Add a real atomic counter** to `unique_temp_dir` and the thirteen sibling helpers DC-83 enumerated,
   matching the shape DC-83 landed in `format_transition.rs`.
2. **Rename `monotonic_suffix`.** It returns a wall-clock timestamp; the name says counter. **It caused a
   real downstream error** — the architect cited the function as a correct pattern on the strength of its
   name, in the handoff that opened DC-83. Names that mislead a careful reader once will do it again.
3. **Prefer one shared helper over fourteen copies** if the crate boundary allows it. If it does not,
   say so — do not manufacture a shared crate for this.

## 3. Acceptance criteria

1. No test-path helper derives uniqueness from a clock alone, or from a clock plus values constant within
   a process.
2. **Demonstrate it**, in DC-83's shape: a barrier-synchronized multi-thread sampling test showing zero
   collisions. One demonstration for the shared helper is enough — **do not add one per call site.**
3. All tests pass unchanged, both toolchains, macOS job green.
4. **No production code touched.**
5. Gate set per `EXECUTION-ORDER.md` §6 rule 9 as amended.

## 4. Non-goals

Retry or serialization — the fix is uniqueness, as in DC-83. Any change to what the tests assert. Any
production path: `monotonic_suffix` and every helper here are `#[cfg(test)]`-scoped, and this increment
must not become a reason to touch anything that ships.
