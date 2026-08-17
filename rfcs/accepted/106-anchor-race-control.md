# RFC 106 — Anchor race control

**Status.** Accepted by the project owner 2026-08-17, as the residual DC-99 recorded rather than closed.
**Tracks.** Proving the Windows anchor identity comparison fires when the window it guards is actually hit.
**Touches.** `crates/prikk-store`'s failpoint surface and Windows test suite. No behaviour change.

**Author-review independence.** Designed and reviewed by the same agent; recorded, not elided.

## 0. Why this exists

DC-99 Stage 2 neutralised the identity comparison in `WindowsAuthority::verified_anchor_path`
(`if identity != self.identity` → `if false`) and ran the full suite on Windows. **936 passed — identical
to the unmodified branch. Not one test depends on it.**

The comparison is real and correct. DC-96 established the division: `GetFinalPathNameByHandle` re-derives
the anchor's current path from the retained handle and closes the *between-operations* case; the identity
check is what covers a replacement **racing the narrow window between that re-derivation and the open that
follows it** — a window `platform-support.md` documents as still open and which re-derivation structurally
cannot close.

**Neither DC-96 acceptance test constructs that race.** One returns at a rename NTFS refuses and never
walks; the other completes its rename before anything else runs, so re-derivation alone finds the retained
directory and identity confirms trivially.

**So prikk carries a security guard, on the anchoring property DC-96 exists to protect, that has never
been observed to do anything.** That is the second such guard this cycle — DC-97's G1 was the first — and
both were found by neutralising and watching, never by reading.

## 1. What is already settled

- **The mechanism exists.** DC-98 wired failpoint injection into Windows and demonstrated all nine
  controls. `set_directory_create_barrier` / `wait_at_directory_create` is exactly the shape required: a
  barrier held at a chosen boundary while another thread acts. **The increment that makes this
  constructible finished before DC-99 did, and neither of us connected them until DC-99's probe came back
  empty.**
- **DC-98 established per-platform gating of `Point` variants** — five were gated
  `#[cfg(any(linux, macos))]` once they had no Windows operation. This increment mirrors that with a
  Windows-only variant.
- **The race is constructible.** NTFS refuses to rename a directory containing an open handle, but permits
  renaming *the object whose own handle is held* — proven by
  `full_verification_retains_wal_objects_trust_and_recovery_diagnosis_after_root_replacement`, which
  renames `.prikk` successfully while its handle is retained. Only ancestors are refused. **So a second
  thread can replace the anchor mid-operation.**

## 2. The obstacle, stated as a problem

The window is **between two consecutive statements in one function**. No amount of test-side timing can
reliably land inside it; a scheduler-race test would pass or fail by luck, which DC-98 already ruled is
worse than none.

The window must therefore be **held open deliberately** — which is what a barrier failpoint is for, and
why this was not constructible before DC-98.

## 3. Acceptance criteria

1. **A test that constructs the race**: one thread held at the boundary between path re-derivation and the
   open that follows; a second thread renames the anchor aside and creates a replacement at the original
   path; the first thread resumes, opens the replacement, and **the operation is refused with the
   anchor-replaced diagnostic** — not merely with an error.
2. **Watched to fail with the identity comparison neutralised.** This is the whole point: the control DC-99
   could not provide. Observed on a real Windows CI run, per the bar every control in DC-97, DC-98 and
   DC-99 met.
3. **The new failpoint is Windows-only and gated**, so it is not dead code on Unix — DC-98's pattern
   mirrored, and enforced by `-D warnings` on the cross-target build rather than by a comment.
4. **No production path changes shape** to accommodate the injection point. If one would have to, that is a
   finding to report, not an edit to make.
5. **The two "not exercised by any test" statements are corrected** — the comment at
   `verified_anchor_path` and `platform-support.md`'s anchor-replacement section. Both currently say the
   guard is unproven; after this they must say what proves it, citing the run.
6. Green three-platform CI.

## 4. Non-goals

- **Closing the race.** The window stays open — Windows offers no `openat`-equivalent to close it by
  construction. This proves the guard fires when the window is hit; it does not narrow the window.
- **A Unix equivalent.** Linux and macOS hold a retained descriptor, so no identity comparison exists there
  to control. Adding one for symmetry would be inventing a guarantee to test.
- **Changing the comparison itself.** It is correct; it is unproven. Only the second changes here.

## 5. Staging

One stage. **Report the boundary placement before wiring**, as DC-98 Stage 2 did — the exact statement the
barrier sits between, and why a barrier there opens the window under test rather than an adjacent one. A
failpoint at the wrong boundary yields a test that passes for the wrong reason, which is the sharper
version of what this increment exists to fix.
