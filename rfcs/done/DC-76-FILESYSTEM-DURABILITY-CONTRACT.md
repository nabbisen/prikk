# RFC (accepted) - DC-76 Filesystem Durability Contract

**Status.** **ACCEPTED by the project owner 2026-08-08**, who approved this increment's scoping —
**mechanical extraction first, Linux as the sole implementation, no behaviour change** — before it was
written. **Independence.** Author-reviewed, the standing ceiling; compensated at implementation review.
**Arises from.** The owner's 2026-08-08 direction: *"I want to release mutation expansion on Windows and
macOS as soon as possible (with clean architecture and safe process)."* This is the enabling increment.
**Target.** 0.20.0, item 1 of the accepted five-item sequence.

## 1. Why this exists, and why it is not the macOS increment

Mutation is Linux-only. **93 `target_os = "linux"` gates** across `crates/prikk-store/src/fsutil/anchored*`
— 28 in `anchored.rs`, 25 in `directory.rs`, 15 in `read.rs`, 13 in `regular.rs`, 11 in `immutable.rs`,
1 in `failpoints.rs`. DC-37 made that deliberate: durability is a security guarantee, not a convenience.

**But the guarantee is currently stated only as an implementation.** There is no single place that says
what prikk requires of a filesystem, so "does macOS satisfy DC-37?" cannot be answered except by reading
93 call sites and reasoning about each. That is the obstacle to the owner's goal, and it is a
documentation-and-structure problem before it is a platform problem.

**This increment adds no platform.** It states the contract, builds a conformance suite, and leaves Linux
as the sole implementation. **The soundness proof is that Linux still passes everything, unchanged.**

## 2. The shape

- **A durability contract** — one explicit interface naming what the store requires: anchored open that
  refuses symlink traversal, atomic replace, file durability, directory durability, and whatever §4's
  enumeration adds. Each operation states **the guarantee**, not the syscall.
- **A conformance suite** every implementation must pass, including the applicable parts of **DC-41's
  crash matrix**. "Does platform X satisfy DC-37?" becomes a test result.
- **Linux implements it**, using `rustix` exactly as today.

## 3. The risk this increment actually carries

**A pure refactor's failure mode is silent behaviour change** — a guarantee quietly weakened while every
test still passes because the tests never pinned it. That is precisely the shape of DC-73's mode defect
(materialization that already "succeeded" was silently wrong) and of DC-74's refusal tests (four of five
survived removing the gate they existed to pin).

**So the conformance suite must be shown to fail when a guarantee is removed.** An assertion that Linux
passes is not evidence; an assertion that Linux *stops* passing when `NOFOLLOW` is dropped is.

## 4. Blocking prerequisites — answer and report before designing

1. **Enumerate what the store actually requires**, as a table: each distinct filesystem guarantee, the
   call sites that need it, and the primitive providing it today. Build it from the 93 gates, not from
   this RFC's summary. **Three increments running, this pattern has found the recorded scope too narrow;
   assume it has here too.**
2. **Which gates are genuinely Linux-specific, and which are incidentally gated?** Some may already hold
   on any `unix`. **Report the split; change nothing.** This is the single most valuable output for the
   macOS increment that follows.
3. **Is DC-41's crash matrix expressible against the contract**, or does it reach into Linux specifics?
   This decides whether the conformance suite can be shared across platforms or forks per platform.
4. **Does the contract's shape force a dependency question now?** `rustix` is Unix-only, and
   `ALLOWED_THIRD_PARTY` permits `prikk-store` exactly `getrandom` and `rustix`
   (`tools/release-policy/src/boundary/placement.rs`). **If the contract cannot be expressed without
   naming a Windows primitive, say so and stop** — that is an owner decision, not an implementation one.

## 5. Acceptance criteria

1. §4 answered and reported before any design.
2. The contract exists as **one** explicit interface; every mutation path goes through it.
3. **No observable behaviour change.** Every existing test passes **unchanged** — 888 at time of writing,
   including DC-41's crash matrix. Any test that must change is a finding to report, not an edit to make
   quietly.
4. **The conformance suite is shown to fail when a guarantee is removed** — a stated negative control per
   guarantee, not a claim that the suite is thorough.
5. `ALLOWED_THIRD_PARTY` **untouched**; no new dependency.
6. **No `target_os` gate is relaxed.** Enabling a platform is the next increment, not this one.
7. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, verbatim, with test counts before and after.

## 6. Non-goals

- **macOS and Windows implementations.** Next increments; macOS first.
- Changing any durability guarantee. If §4 finds one that is weaker than DC-37 claims, **that is a
  finding to report** — it may be the most valuable thing this increment produces.
- Touching `patch_replay`, the object format, or anything above the filesystem boundary.
