# DC-99 Windows Capability Parity — design v1

**RFC:** `rfcs/accepted/DC-99-WINDOWS-CAPABILITY-PARITY.md`. Read §3 first — both stages turn on it.

**Report the API semantics before wiring, per stage.** What each call returns in each case is a fact to
establish, not to derive from documentation prose. Two rounds per stage: investigate, then implement.

## Stage 1 — `prikk unlock` liveness

### The shape

`prikk-ffi` gains one function. Suggested contract, not a mandated signature:

```rust
#[cfg(windows)]
pub enum ProcessLiveness { Exists, DoesNotExist, Indeterminate }

#[cfg(windows)]
pub fn process_liveness(pid: u32) -> ProcessLiveness;
```

**`prikk-ffi` must not depend on `prikk-store`**, so it cannot return `PidLiveness` directly. Map at the
`unlock.rs` call site, where the `#[cfg(not(any(linux, macos)))]` stub is today.

The obvious primitive is `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid)`, plus something to
distinguish *open handle to a live process* from *open handle to a terminated one*.

### What to establish before writing it

1. **Which error does `OpenProcess` return for a PID that does not exist**, and which for one that exists
   but is not queryable? The Unix analogue is `ESRCH` versus `EPERM`, and RFC criterion 3 requires the
   second to reach `AppearsRunning`. **Confirm the codes; do not assume the mapping.**
2. **How to detect a terminated-but-open process.** `GetExitCodeProcess` returning `STILL_ACTIVE` (259) is
   ambiguous — a process may exit with 259. `WaitForSingleObject(handle, 0)` returning `WAIT_TIMEOUT` is
   the usual unambiguous answer. **Establish which you are using and why it is not ambiguous.**
3. **Whether `PROCESS_QUERY_LIMITED_INFORMATION` is enough**, or a broader right is needed — the narrower
   right is preferable and is why it is named here, but verify it works for a process owned by another
   user, since that is exactly the access-denied case criterion 3 is about.
4. **Close the handle.** A leaked process handle in a command that runs on every `prikk unlock` is a
   defect; say how it is closed on every path including the error paths.

### §3's rule, concretely

**Only a positively established absence returns `DoesNotExist`.** Everything else — an unexpected error,
a wait result you did not anticipate, a failure to determine — returns `Indeterminate`, which maps to
`PidLiveness::Unknown`.

**Write the mapping as an exhaustive match with no catch-all that reaches `DoesNotExist`.** If a future
Windows version adds an error code, the fail-safe direction must be structural, not a comment.

### Criterion 2 is the visible outcome

`unlock/tests.rs`'s two tests currently split: strong assertion on `cfg(any(linux, macos))`, `Unknown`
elsewhere. **Both splits come out.** If either cannot, that is a finding — it means the Windows answer is
not actually equivalent, and I want to know that rather than have it papered over with a retained gate.

Add a Windows-reachable test for the access-denied path if you can construct one honestly; if you cannot,
say why rather than leaving criterion 3 asserted only by code reading.

## Stage 2 — 128-bit anchor identity

### The shape

`GetFileInformationByHandleEx(handle, FileIdInfo, &mut info, size_of::<FILE_ID_INFO>())`, yielding
`FILE_ID_INFO { VolumeSerialNumber: u64, FileId: FILE_ID_128 }`.

`FileIdentity` becomes two-form. The representation is yours; the requirement is **§3's second rule: a
cross-form comparison must be impossible or false by construction.** An enum with two variants and a
derived `PartialEq` gives that for free, since variants of different shape never compare equal. A struct
with an "is_128" flag does not, and would be a discipline rather than a guarantee.

### What to establish before writing it

1. **What `GetFileInformationByHandleEx(FileIdInfo)` returns on an unsupported filesystem** — which error
   code, so the fallback triggers on that specifically rather than on any failure. A fallback that
   triggers on *every* error would silently downgrade a real fault.
2. **Whether the CI runner's temp filesystem is NTFS** — if it is, the fallback path is untested by
   default, and the increment must say so rather than let an untested branch ship. A test that forces the
   fallback (or a stated reason one cannot be constructed) is worth more than the fallback itself.
3. **Whether `FILE_ID_128`'s 16 bytes compare meaningfully as bytes**, or whether any part is
   reserved/unstable.

### The negative control

Criterion 4 requires the two DC-96 acceptance tests to be **watched to fail with identity comparison
neutralised** — the same probe pattern as DC-97's G1 and DC-98's nine. Throwaway branch, per
[[probe isolation]]: detached worktree, primary tree confirmed clean afterwards, deleted when the
question resolves.

**Aim it at the guarantee, not the scaffolding.** The recurring lesson from DC-97's G1 and DC-98's G8 is
the question *which line, if deleted, makes the property false?* Here that is the identity comparison in
`verified_anchor_path`, not `identity_of` itself — neutralising the FFI call would fail the open, not the
comparison, and would prove something different from what criterion 4 asks.

## Both stages

### Gates

The standing set per `EXECUTION-ORDER.md` §6 rule 9, both cross-target clippy runs, green three-platform
CI before merge. Report the Windows test-count delta; it is CI-only and I will read it from the run.

### Stop-and-report

- A liveness answer that cannot be made to differ between a live and an absent PID on Windows — that is
  the stub in a new costume.
- `FILE_ID_INFO` unavailable on the CI runner, making Stage 2 unverifiable there. Report it; do not ship
  an unexercised path and call the row closed.
- Either stage requiring a change to `PidLiveness`'s contract or to `DurabilityContract`. Neither should
  need one; if one does, that is a design question, not an implementation detail.
