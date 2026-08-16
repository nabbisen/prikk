# RFC (accepted) - DC-97 Windows Durability Evidence

**Status.** **ACCEPTED by the project owner 2026-08-16**, as the first item of the agreed order for
strengthening Windows mutation. Closes DC-87 acceptance criterion 4, which shipped open in 0.21.0.

**Author-review independence.** Designed and reviewed by the same agent; recorded, not elided.

## 0. The finding that reframes this increment

Criterion 4 was written as *"DC-76's nine negative controls demonstrated on Windows, or a reported reason
each one cannot be."* Scoping it turned up something larger.

**`crates/prikk-store/src/fsutil.rs:14-17`:**

```rust
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod caller_tests;
#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests;
```

**The entire filesystem-durability test surface is gated off on Windows** — `fsutil::tests` (742 lines,
including the G1-G9 conformance suite) and `fsutil::caller_tests` (446 lines: the directory, sync, and
validation matrices). None of it compiles on Windows, so none of the 909 tests that ran there touch it.

Two spot checks confirm the consequence rather than infer it, each against a control that ran:

- `concurrent_required_directory_creation_is_idempotent` — G8's control — **does not appear in the
  Windows run at all.**
- The only "conformance" on Windows is `dc67_ordinary_use_conformance.rs`, an unrelated integration test.

**So `platform-support.md`'s nine-guarantee table currently makes nine claims about Windows, and the suite
that tests those guarantees is switched off on Windows.** This is the same shape as every finding of
DC-87 Stage 2 — a green suite that is not asking the question on the platform in question — and it is
what criterion 4 exists to surface.

## 1. What is already settled

- **DC-76 allows a reported reason.** Criterion 4's own wording — *"or a reported reason each one cannot
  be"* — follows DC-76's precedent, where two controls could not be cleanly demonstrated and were
  reported as findings rather than dropped. **A reported reason is a passing outcome; a silent gap is
  not.**
- **Some guarantees are genuinely undemonstrable on Windows, and that is known in advance:**
  - **G6/G7** (regular-file validation, non-blocking opens) rest on a FIFO, created via `mkfifo(3)`
    (`test_support.rs:194-203`). Windows has no FIFO in this sense. Expect a reported reason.
  - **G9** (mode-bit isolation) is a documented no-op on Windows — NTFS has no POSIX execute bit — and
    conformance.rs's own doc already records it as *"not independently negative-controllable on Linux"*
    either. Expect a reported reason on both counts.
- **Concrete compilation obstacles**, visible now, so the increment is not a discovery exercise:
  `conformance.rs:27` imports `std::os::unix::fs::PermissionsExt` unconditionally; `:30-33` import
  `LinuxDurability`/`MacosDurability` with no Windows arm; each assertion helper has per-platform wrapper
  functions with no Windows arm.

## 2. The obstacle, stated as a problem

The gate at `fsutil.rs:14-17` is not arbitrary — the suite genuinely uses unix-only facilities in places.
The question is **which parts are unix-only by nature and which are unix-only by accident**, and the two
have been indistinguishable because the whole module is behind one gate.

**Removing the gate wholesale is not the goal.** A suite that compiles on Windows by having its Windows
assertions weakened to nothing would satisfy a checklist and prove less than the honest gap does.

## 3. Acceptance criteria

1. **Every guarantee in `platform-support.md`'s nine-guarantee table has, on Windows, either a
   demonstrated negative control or a written reason it cannot have one.** Nine rows, nine answers, no
   blanks.
2. **The reasons are specific and checkable** — naming the primitive or property that is absent, as the
   FIFO and execute-bit cases already do. *"Not applicable on Windows"* is not a reason.
3. **No Linux or macOS control is weakened** to make a shared code path compile. If a control must be
   split per platform, the non-Windows branch is unchanged, and the Windows branch asserts something real
   or is replaced by a reason under criterion 1.
4. **The gate at `fsutil.rs:14-17` reflects what is actually unix-only**, not the whole module. Whatever
   remains gated is gated for a stated reason.
5. **The Windows test count rises**, and the increment reports by how much. A Windows-enabled suite that
   adds no executing assertions has not enabled anything.
6. **`platform-support.md` states, per guarantee, whether it is negatively controlled on Windows.** The
   table currently implies a uniformity that does not exist; a reader should be able to see which rows are
   tested there and which rest on argument.
7. Green three-platform CI, per the standing rule.

## 4. Non-goals

- **Changing any durability guarantee, or any implementation.** This increment adds evidence and
  documentation. If it finds a guarantee that does not hold on Windows, **that is a finding to report and
  stop on**, not to fix here — exactly DC-76's own posture.
- **Making every control work on Windows.** Some cannot. Criterion 1 accepts a reason.
- **The other three items of the agreed strengthening order** — deleting the two orphaned weaker methods,
  `prikk unlock` liveness, and `FILE_ID_INFO`. Each is its own increment.
- **Enabling `caller_tests` if its matrices turn out to be genuinely unix-specific.** Report which, with
  reasons; do not force them.

## 5. Staging

One stage. **Report before changing anything**: walk all nine guarantees, classify each as demonstrable
or not on Windows with the reason, and submit that classification for review *before* writing test code.
The classification is the increment's substance; the code follows from it.
