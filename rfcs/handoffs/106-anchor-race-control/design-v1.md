# RFC 106 — Anchor race control — design v1

**RFC:** `rfcs/accepted/106-anchor-race-control.md`. Read §0 and §1 first — §1 is why this is now
constructible when it was not four increments ago.

## 1. The boundary

`WindowsAuthority::verified_anchor_path` (`windows_authority.rs`) reads:

```rust
let current_path = prikk_ffi::current_path_of(&self.handle)?;   // re-derive from the retained handle
let file = windows::open_directory_no_follow(&current_path)?;   // open what is at that path NOW
let identity = windows::identity_no_follow(&file, &current_path)?;
if identity != self.identity { /* refuse */ }
```

**The window is between line 1 and line 2.** `current_path` is captured from the retained handle, then the
path is opened. A replacement installed at that path in between is what the identity comparison exists to
catch — and it is the only thing that catches it, because the re-derivation already happened.

**Put the barrier immediately after `current_path_of` returns and before `open_directory_no_follow`.**

**Report this placement before wiring**, per RFC §5, with your own reading of why a barrier there opens
the window under test and not an adjacent one. If you conclude the boundary is elsewhere, say so — I have
read this function several times this cycle and have been wrong about adjacent things twice.

## 2. The failpoint pair

Mirror `set_directory_create_barrier` / `wait_at_directory_create` exactly — same module, same shape:

- A `Point`/`TestPoint` variant, **`#[cfg(windows)]`**, mirroring how DC-98 gated five variants
  `#[cfg(any(linux, macos))]` once they had no Windows operation. This is the first Windows-only point;
  the pattern is established, the direction is new.
- `set_anchor_verification_barrier(Arc<Barrier>)` (crate-visible, test-driven) and
  `wait_at_anchor_verification()` (super-visible, called from production).
- **A no-op when no barrier is installed**, exactly as the directory-create pair is — production behaviour
  unchanged, which is RFC criterion 4.

`windows_authority.rs` currently has **zero** `failpoints::` calls; this adds the first. Check whether the
module's import surface needs anything, and whether the Unix build sees any of it.

## 3. The test

Two threads on a `Barrier::new(2)`:

- **Thread A** performs an operation that goes through `verified_anchor_path` and blocks at the barrier
  with `current_path` already captured.
- **Thread B** waits at the barrier, then **renames the anchor aside and creates a replacement directory
  at the original path**, then releases.
- **Thread A** resumes, opens the path — now the replacement — computes its identity, and must **refuse**.

**Assert the specific diagnostic, not `is_err()`.** The refusal message names the anchor as replaced;
`is_err()` would pass if the open failed for any unrelated reason. That distinction cost DC-97 two rounds
on G1 and DC-96 one on the rename control — it is the single most repeated lesson of this cycle.

**Rename `.prikk` itself, not an ancestor.** NTFS refuses to rename a directory containing an open handle,
so an ancestor rename fails and the race never occurs; renaming the held object itself succeeds, as
`full_verification_retains_wal_objects_trust_and_recovery_diagnosis_after_root_replacement` already
demonstrates.

## 4. The negative control — RFC criterion 2, and the reason the increment exists

Throwaway probe branch, per [[probe isolation]]: neutralise the identity comparison — the same `if false`
DC-99's probe used — and confirm **this new test goes red**, on a real Windows CI run.

**Aim at the comparison, not at `identity_of`.** Neutralising the FFI call would fail the open instead and
prove something else. That lesson has now cost three rounds across DC-97, DC-98 and DC-99.

**Expect this to be the only failure.** DC-99 established that no other test depends on the comparison, so
a probe that takes others down means something changed since — report it rather than absorbing it.

Delete the probe when it resolves; record its tip SHA in the report.

## 5. Then correct the record

RFC criterion 5. Two places currently state the guard is unproven, both written by this project
deliberately so the gap would not be silently carried:

- the comment at `verified_anchor_path`'s comparison
- `platform-support.md`'s anchor-replacement section

**Replace "not currently exercised by any test" with what now exercises it**, citing the run that watched
it fail. Leave the surrounding statement that the window itself remains open — that is still true and is
not what this increment changes.

## 6. Gates

The standing set per `EXECUTION-ORDER.md` §6 rule 9, both cross-target clippy runs, green three-platform
CI. Report the Windows test-count delta.

**Stop-and-report:** the barrier cannot be made to hold the window open; the race cannot be constructed
because the rename is refused after all; or the test passes with the comparison neutralised — which would
mean the barrier is at the wrong boundary and the test is measuring something else.
