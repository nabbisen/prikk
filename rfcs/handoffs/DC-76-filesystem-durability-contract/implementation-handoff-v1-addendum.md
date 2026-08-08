# DC-76 Handoff v1 — Addendum 1: §1 accepted, proceed to design

**Date:** 2026-08-08. **Authored by** the architect.
**Responds to:** `prerequisite-questions-v1.md` (`b437d17`). **Review:**
`.git-exclude/reviewed/DC-76-prerequisite-questions-review-v1.md`.

## 1. Accepted. Design is cleared.

I re-derived every load-bearing claim from `rustix` 1.1.4 on disk rather than accept it. **All confirmed:**
no Windows filesystem backend (`windows_syscalls.rs` exists only under `event/` and `io/`, never `fs/`);
`openat`/`mkdirat` gated `not(redox)`; `linkat`/`unlinkat` `not(any(espidf, redox))`; `renameat`
genuinely ungated (`src/fs/at.rs:264`, only `#[inline]`); `renameat_with` explicitly including `apple`;
no `apple` exclusion anywhere. G5 and G9 spot-checked against the repository and accurate.

**§2's conclusion stands and is now verified: the gates are incidental, and macOS is a porting and
verification task rather than a redesign.** That is the most valuable thing this report produced.

The nine-guarantee reframe — 93 gates being `#[cfg]` scaffolding around nine facts, not 93 independent
ones — is the right way to see it, and it should shape the contract directly.

## 2. One factual error, and it lands in your favour

> "…a call `rustix` does not currently wrap"

**`rustix` does wrap `F_FULLFSYNC`**: `rustix::fs::fcntl_fullfsync`, `src/fs/fcntl_apple.rs:24`, backed by
`fcntl(fd, F_FULLFSYNC)` at `backend/libc/fs/syscalls.rs:2242`.

**You reached the right §4 answer while holding a belief that would have undermined it.** Had `rustix`
*not* wrapped it, macOS would have needed raw `libc` — which is **not** in `ALLOWED_THIRD_PARTY` either,
so "No for macOS" would have been wrong. It isn't. macOS genuinely stays inside `getrandom` + `rustix`.

**Your caveat is also better-evidenced than you had it.** `rustix`'s own docs say it: `src/fs/fd.rs:253`
states `fsync` does not ensure persistent storage on Apple and points to `fcntl_fullfsync`.

**So the open question is now a specification.** G3 on macOS must use `fcntl_fullfsync`, not `fsync`.

## 3. Use G3 as the worked example when you write the contract

G3 stated as a **guarantee** — "once this returns, content and directory entry both survive a crash" — is
satisfied by `fsync` on Linux and `fcntl_fullfsync` on macOS. Stated as a syscall it would already be
wrong, before a second platform exists.

**That is the whole argument for this increment, and it arrived before the contract was written. Make G3
the example the others are shaped against.**

## 4. One process note

You cited the gates as `src/backend/libc/fs/{at,types}.rs`. **`src/backend/libc/fs/at.rs` does not
exist** — the public API is `src/fs/at.rs`, the backend is `src/backend/libc/fs/syscalls.rs`. Every value
you quoted is correct and I re-derived them all, so nothing is wrong in substance. But in a project whose
standard is "verified by reading," a path that cannot be opened weakens otherwise exemplary evidence.
**Cite what you opened.**

## 5. What design now owes

RFC §5 unchanged. The two I will check hardest:

- **Criterion 3 — no observable behaviour change.** Every existing test passes **unchanged**. A test that
  must change is a finding to report, not an edit to make quietly.
- **Criterion 4 — the conformance suite must be shown to FAIL when a guarantee is removed.** One stated
  negative control per guarantee, nine in total. I will run these myself, as I did on DC-74 where four of
  five refusal tests survived removing the gate they existed to pin.

Still hard limits: no `target_os` gate relaxed, no new dependency, `ALLOWED_THIRD_PARTY` untouched,
nothing above the filesystem boundary.

**Windows is with the owner** and is not required until Windows is scoped. Do not design for it here.
