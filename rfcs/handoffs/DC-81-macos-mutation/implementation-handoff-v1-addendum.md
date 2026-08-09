# DC-81 Handoff v1 — Addendum 1: §1 accepted, design cleared, one criterion of mine amended

**Date:** 2026-08-09. **Authored by** the architect.
**Responds to:** `.git-exclude/review-request/prikk-dc-81-prerequisite-questions-v1.md`.
**Review:** `.git-exclude/reviewed/DC-81-prerequisite-questions-review-v1.md`.

## 1. Accepted. Design is cleared.

**Q1 verified independently**: `ref_name_storage_key` is `to_hex(&sha256(ref_name.as_bytes()))`
(`layout.rs:386-388`); no `.join(ref_name)`/`Path::new(ref_name)`/`PathBuf::from(ref_name)` anywhere in
`crates/prikk-store/src`; non-ASCII rejected at `path.rs:48`. **Ref and tag names never reach the
filesystem as path components.** Your conclusion holds, and reversing a wrong first turn by tracing it —
rather than dropping it — is the same discipline as G6 and G9.

**One leg was incomplete, and the missing piece is in your favour.** The repository-path argument rests
on non-ASCII rejection, which does **not** cover ASCII case collisions — `README.md` and `readme.md` are
both ASCII, and APFS folds exactly that.

**The real protection is stronger than the one you cited.** `validate_no_path_collisions`
(`crates/prikk-replay/src/path.rs:39-57`) folds ASCII case and rejects folded duplicates, and
`state_root.rs:143` applies it over **all** state-root entries — the whole derived tree, not the incoming
batch. **A sealed repository therefore cannot contain a case-colliding pair**, so a Linux-authored
repository materialised onto APFS cannot hit one either. That was the failure mode I actually feared when
I wrote question 1, and it is closed by construction.

Your ASCII-only dependency note stands and should be carried forward.

## 2. Criterion 3 was unsatisfiable as I wrote it. Amended.

You showed *"the conformance suite passes on macOS unmodified"* cannot be met: the suite is
`#[cfg(all(test, target_os = "linux"))]` at the module level and does not compile on macOS, so relaxing a
gate is itself a modification. **Second time this cycle I have written a criterion that could not be met
as phrased**, and you surfaced it by testing the claim rather than inheriting it.

**Amended in the RFC:**

> The conformance suite's **assertion bodies** pass on macOS unchanged. Module gates may be relaxed and a
> per-implementor `#[test]` wrapper added per assertion — both mechanical. **Any change to an assertion
> body, or to what it asserts, is a finding to report.**

## 3. Carried into design

- **The two genuine ports you identified** — the FIFO tests (`mkfifoat`, non-blocking assumptions
  unverified on macOS) and `immutable_failpoints_retain_required_artifacts_and_retry`'s Linux errno
  mapping — are ports, not recompiles. Treat them as such.
- **A DC-76 doc overclaim I missed at its review:** `conformance.rs`'s module doc says a future platform
  is "checked by the *same* code", but both `#[test]` wrappers hardcode `&LinuxDurability`
  (`conformance.rs:61,110`). The `assert_*` helpers are properly generic. **Correct that doc comment
  while you are adding the macOS wrappers.**
- **Q3's measurement is follow-through, not optional.** Once the macOS job runs mutation, measure
  `fcntl_fullfsync` against `fsync` on the actual runner. A consensus that it is "materially slower" is
  not a number, and NFR-PERF-01 will eventually want one.

## 4. On Q3's sourcing, briefly

Citing Apple's own `fsync(2)` as primary, offering SQLite's **shipped** `F_BARRIERFSYNC` default as
stronger evidence than a forum claim, and **deliberately declining** the most quantitatively attractive
source because it was old and unofficial — that is the right instinct, and rarer than it should be.
Inventing a plausible multiplier would have been easy and undetectable.

## 5. Unchanged

§5's stop-and-report conditions. The CI job must exist and be green before any gate is relaxed in a
merged commit. Docs must not claim macOS mutation before then. Rule 9 as amended — eleven gates.
