# DC-76 Filesystem Durability Contract — Handoff v1

**Cleared to start on §1 only.** Accepted by the project owner 2026-08-08, at
`rfcs/done/DC-76-FILESYSTEM-DURABILITY-CONTRACT.md`. **Authored by** the architect.
**Touches:** `crates/prikk-store/src/fsutil/anchored*` and its callers. **Adds no platform.**

## 1. Four questions, answered and reported before any design

Write `prerequisite-questions-v1.md` beside this file. This pattern has widened the recorded scope in
DC-65, DC-72, DC-73, and DC-75 — **four increments running. Assume it will here too.**

1. **Enumerate what the store actually requires of a filesystem** — a table: each distinct guarantee, the
   call sites needing it, the primitive providing it today. Build it from the **93 `target_os = "linux"`
   gates** (28 `anchored.rs`, 25 `directory.rs`, 15 `read.rs`, 13 `regular.rs`, 11 `immutable.rs`,
   1 `failpoints.rs`), not from the RFC's summary of them.
2. **Which gates are genuinely Linux-specific, and which are incidentally gated?** Some may already hold
   on any `unix`. **Report the split. Change nothing.** This is the most valuable single output here —
   it is what sizes the macOS increment that follows.
3. **Is DC-41's crash matrix expressible against the contract**, or does it reach into Linux specifics?
   Decides whether the conformance suite is shared across platforms or forks per platform.
4. **Does the contract's shape force a dependency question now?** `rustix` is Unix-only, and
   `ALLOWED_THIRD_PARTY` gives `prikk-store` exactly `getrandom` and `rustix`
   (`tools/release-policy/src/boundary/placement.rs`). **If the contract cannot be expressed without
   naming a Windows primitive, stop and report** — that is an owner decision.

## 2. Why this increment exists, so you can judge trade-offs yourself

The owner wants mutation on macOS and Windows **as soon as possible, with clean architecture and a safe
process.** The obstacle is not that the platforms are hard — it is that **DC-37's guarantee is stated
only as an implementation**, so "does macOS satisfy it?" can only be answered by reading 93 call sites.

You are making the guarantee answerable. **Each contract operation should state the guarantee, not the
syscall** — "atomically replace" rather than "renameat"; "refuse symlink traversal" rather than
"`O_NOFOLLOW`". A contract phrased in Linux syscalls would defeat the purpose.

## 3. The risk here is silent weakening, and it is the whole review

A pure refactor fails by quietly dropping a guarantee while every test still passes, because the tests
never pinned it. **That is exactly DC-73's mode defect** (materialization that already succeeded was
silently wrong) **and DC-74's refusal tests** (four of five survived removing the gate they existed to
pin — I found that by negative control, and I will run the same kind here).

**So criterion 4 is not decoration:** show the conformance suite **fails** when a guarantee is removed —
one stated negative control per guarantee. "Linux passes" is not evidence. "Linux stops passing when
`NOFOLLOW` is dropped" is.

## 4. Hard limits

- **No `target_os` gate is relaxed.** Enabling a platform is the next increment.
- **No new dependency**, `ALLOWED_THIRD_PARTY` untouched.
- **Every existing test passes unchanged** — 888 at time of writing, including DC-41's crash matrix. **A
  test that must change is a finding to report, not an edit to make quietly.**
- Nothing above the filesystem boundary: `patch_replay`, the object format, all untouched.

## 5. If you find a weak guarantee, that is the deliverable

If §4's enumeration turns up a durability guarantee **weaker than DC-37 claims**, that is very likely the
most valuable thing this increment produces. **Report it; do not fix it here.** Same standing rule as
every increment since DC-74: report, don't absorb.

## 6. Gates

`EXECUTION-ORDER.md` §6 rule 9, **verbatim** — including `--locked`, `--no-fetch`, and `cargo +1.85.0`.
Test counts before and after.
