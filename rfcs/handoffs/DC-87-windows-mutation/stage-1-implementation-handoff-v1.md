# DC-87 Stage 1 — Implementation Handoff v1

**Design:** `design-v1.md`, accepted by the project owner 2026-08-16. **RFC:** `DC-87-WINDOWS-MUTATION.md`
§4 (staging), §5 (acceptance), §6 (non-goals).
**Stage 2 (`WindowsDurability`) is not authorized.** This handoff covers Stage 1 only.
**Target.** 0.21.0.

## 1. What Stage 1 is, and why it is separate

DC-82's lesson in its own words: *"a behaviour-preserving refactor and a new-platform backend have
entirely different proofs. Bundled, a reviewer cannot tell which half a failure came from."*

**Stage 1 makes the seam platform-neutral on Linux and macOS, with no behaviour change**, so the Windows
backend is *written against* the final shape rather than migrated into it. **No Windows code is written
in this stage.**

## 2. The seam, concretely

`fsutil/anchored/directory.rs:21-52` is the whole problem in thirty lines:

```rust
pub(crate) struct MutationRoot {
    path: Arc<PathBuf>,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    directory: Arc<AnchoredDirectory>,      // OwnedFd
}
```

**`MutationRoot` is a different struct on different platforms**, and `same_authority` branches on `cfg`
to compare either the handle or the path. Every anchored operation presumes the `openat`-style handle is
available.

**Stage 1's job: one interface, per-platform implementors, one `MutationRoot`.** The Linux/macOS
implementor keeps the retained `OwnedFd` and `openat`. A future Windows implementor performs a
component-at-a-time walk (design-v1.md §2) — **it is not written here**, but the interface must not
presume it away.

## 3. The one question that can send DC-87 back

**Can "anchored authority" be expressed at this seam without presuming handle-relative resolution, while
leaving `DurabilityContract`'s eleven methods and nine guarantees unchanged?**

The Linux implementor resolves relative to a held fd. A Windows implementor cannot — it re-walks a path
string and checks each component. **If the interface can only be written by either exposing a directory
handle (which Windows lacks) or by changing the method set (which §6 forbids), stop and report.**

That is a design question that returns to the architect, **not a decision to take inside this stage**.
DC-87 §4 says so explicitly, and §6 lists any change to the guarantees or the method set as a non-goal.

**Report which it is before building the seam**, not after — this is the Step-0-shaped part of Stage 1.

## 4. What must not change

- **No behaviour change on Linux or macOS.** The proof is that **every existing test passes unchanged**.
  A test that needed editing is a behaviour change until you show otherwise.
- **`DurabilityContract`'s method set and the nine guarantees** (§6).
- **DC-71's property**: `prikk-store` still compiles for a target with no implementor, mutation there
  still fails at **runtime**, and read-only commands still work. `none.rs` stays.
- **No path-policy change** — DC-72 already rejects the Windows-hostile forms cross-platform (§6).
- **No new dependency and no `unsafe`.** Nothing in design-v1.md's mechanism needs either.

## 5. Measure the baseline before you touch anything

§5 criterion 6 asks for the production `target_os` count in `fsutil/`, before and after, with a reason
for each gate that remains irreducible.

**Measured 2026-08-16, before Stage 1: 87 production `target_os` occurrences across nine files** —
`contract.rs`, `anchored.rs`, and `anchored/{none,failpoints,immutable,linux,read,regular,directory}.rs`.

**Take your own baseline rather than trusting that number** — it is mine, taken with
`grep -rn target_os … | grep -v tests | wc -l`, and a different counting rule gives a different figure.
Say which rule you used. The delta is what matters, and it is only meaningful if both ends use one rule.

**DC-82's single-digit target was the architect's to miscalibrate once already.** Report the honest number;
you are not held to a figure set from outside the code.

## 6. Acceptance criteria

1. **`MutationRoot` is one struct on every platform**, with the platform difference behind an interface.
2. **Every existing test passes unchanged**, on Linux and on the macOS CI job. Any test that had to
   change is called out with why it is not a behaviour change.
3. **DC-76's nine negative controls still fail** when their guarantee is removed — the refactor did not
   quietly disconnect one.
4. **DC-71 demonstrated, not asserted**: compile for a no-implementor target, mutation fails at runtime,
   read-only works.
5. **`target_os` count reported** before and after, with the counting rule stated and each remaining gate
   justified.
6. **§3's question answered explicitly** — either the seam was drawn without presuming handle-relative
   resolution, or a stop-and-report.
7. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus **green three-platform CI**.

## 7. Standing

- **Work on a branch.** Branch → push → green CI → merge.
- **Report counts** per rule 10. Baseline at `9704f1b` (0.20.0): `prikk-store` **738**, `prikk` **117**,
  `prikk-release-policy` 83, `prikk-object` 80, `prikk-replay` 44, `prikk-hash` 14, `prikk-crypto` 7,
  `prikk-error` 0; **179 locked packages**. Report the figures; the architect updates the line at merge.
- **A stop-and-report is a complete outcome**, and §3 is the likeliest place for one. Four of RFC 102's
  stages produced one and each was right.
