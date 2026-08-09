# RFC (accepted) - DC-82 Mutation Dispatch Collapse

**Status.** **ACCEPTED by the project owner 2026-08-09** — "allowed if it's a cleaner process."
**It is, and the reason is specific:** a behaviour-preserving refactor and a new-platform backend have
entirely different proofs. The refactor's is *"every test unchanged, gate count down"*; a Windows
backend's is not. Bundled, a reviewer cannot tell which half a failure came from. **DC-76's B1 was found
exactly because the refactor was isolated enough to diff against its parent.**
**Independence.** Author-reviewed — the standing ceiling.
**Target.** 0.20.0, **before the Windows increment**. **No behaviour change.**

## 1. Why now

DC-81 §6 set the target: call sites unconditional, single-digit gate count, fallback preserved.
**DC-81 moved the other way — `fsutil/` went 110 → 135, `anchored.rs` alone 31 → 44** — because each of
ten mutation functions' two-armed dispatch became three-armed.

**That was not a defect on the developer's part: the architect recorded §6 mid-increment and never
handed it over.** But extrapolated, Windows takes `anchored.rs` to roughly 57. **The target is
unreachable by widening call sites**; it needs the dispatch collapsed.

**Cheaper now than later.** Two implementors is a simpler collapse than three, and doing it before
Windows means the Windows backend is written against the final shape rather than migrated into it.

## 2. Candidate shape — to evaluate, not to inherit

Offered so §3 starts from a proposition rather than a blank page. **It is not a ruling**, and the
architect's design assertions this cycle have needed correction repeatedly.

**Make "unsupported" an implementor rather than an arm.** Today each call site branches three ways, the
third returning `unsupported_mutation()`. If a `NoDurability` type implements `DurabilityContract` by
returning that error from every method, then a single gated type alias selects the active implementor and
**every call site becomes unconditional**:

- `#[cfg(target_os = "linux")]` → `LinuxDurability`
- `#[cfg(target_os = "macos")]` → `MacosDurability`
- `#[cfg(not(any(...)))]` → `NoDurability`

Gates then live at the alias and the module declarations, not at ten call sites — and each future
platform adds **one** line rather than ten arms.

**If this shape does not survive contact with the code, report that.** It is a proposition.

## 3. Blocking prerequisites

1. **Does the candidate shape actually work** against the existing signatures, or do the methods differ
   in a way that resists one alias? Report before designing.
2. **Where must gates genuinely remain?** Enumerate them, and say why each is irreducible.
3. **Does the fallback still behave identically?** `unsupported_mutation()` must remain a **runtime**
   error, so `prikk-store` still *compiles* on FreeBSD, illumos, and any target with no implementor, and
   read-only commands still work there. **That is DC-71's guarantee, and it is the one thing this
   increment must not break.**

## 4. Acceptance criteria

1. §3 answered and reported before design.
2. **Zero `target_os` references at `anchored.rs`'s ten mutation call sites.**
3. **Production-code `target_os` count in `fsutil/` in single digits.** Test-module gates may remain;
   report production and test counts separately, and the before/after total.
4. **No behaviour change.** Every test passes **unchanged** on Linux *and* on the macOS CI job. A test
   that must change is a finding to report.
5. **DC-71 preserved, demonstrated not asserted:** show `prikk-store` still compiles for a target with no
   implementor, and that mutation there fails at runtime rather than at build time.
6. **DC-76's nine negative controls still fail when their guarantee is removed** — the refactor must not
   quietly unpin them.
7. Gate set per rule 9 **as amended** — the canonical nine plus macOS and Windows clippy.

## 5. Non-goals

Windows. Any behaviour change. Any change to the nine guarantees or to `DurabilityContract`'s method
set — if the collapse appears to require one, **stop and report**.
