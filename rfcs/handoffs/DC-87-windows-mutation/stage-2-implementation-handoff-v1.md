# DC-87 Stage 2 — Implementation Handoff v1

**Design:** `design-v1.md` §2–§4, accepted 2026-08-16. **RFC:** `DC-87-WINDOWS-MUTATION.md` §4–§6.
**Stage 1 merged** at `6055d19` with green three-platform CI.
**Target.** 0.21.0. **This is the last stage of DC-87.**

## 1. What Stage 1 bought, so this stage does not re-derive it

- **`MutationRoot` is one struct on every platform.** The platform difference lives behind
  `PlatformAuthority` (`directory.rs`) and `AnchoredReader` (`read.rs`).
- **Platform branches inside shared function bodies: zero.** Stage 1 took them from twenty to none.
  **A Windows backend is a third `impl` block, not a third arm in twenty places** — that is the whole
  reason Stage 1 existed, and it is the property this stage must not undo.
- **`DurabilityContract`'s eleven methods and nine guarantees are unchanged**, and stay unchanged (§6).

**If you find yourself adding an inline `#[cfg]` branch to a shared function, stop.** That is Stage 1
being undone, and it means either the seam is wrong or the thing belongs in an implementor. Report it.

## 2. What to build

**Three implementors, one per trait, plus the dispatch arms:**

- `WindowsDurability` implementing `DurabilityContract`
- a Windows `PlatformAuthority` implementing the anchored-authority trait
- a Windows `AnchoredReader`
- the `cfg` arms in `ACTIVE_DURABILITY` and `ACTIVE_READER`

**The design is settled — `design-v1.md` §2 for the authority walk, §3's table for all eleven methods,
§4 for the `init`-time exemption. Do not redesign it. Do report if building it shows the design wrong.**

**Three things from §3 that are easy to get wrong and invisible when you do:**

1. **`FILE_SHARE_DELETE` is a whole-backend rule, not local to `remove_if_present`.** Every open this
   backend performs, for any purpose including reads, must request it. **Enforce it in one place** — a
   single open helper — so it cannot be omitted per-call-site. A single open that forgets it makes an
   unrelated deletion elsewhere fail with no visible connection to the cause.
2. **The two documented no-ops must be documented, not silent.** `set_permission_bits` states that
   prikk's recorded mode is unaffected and a later Linux checkout restores it faithfully;
   `durable_directory_entry` names the unclean-shutdown marker as what makes it safe. **A method
   returning `Ok(())` with no comment is indistinguishable from one that forgot.**
3. **The three rename-shaped methods get honest doc comments**, not copied Linux ones.
   `atomic_replace`'s weaker guarantee is acceptable *only because* its only callers are rebuildable
   caches — say so, so the next person adding a caller reads it.

## 3. The CI job — this is what makes Windows mutation provable

**Criterion 3 wants a `Windows mutation test suite` job mirroring `macOS mutation test suite`
(`ci.yml`).** Mirror it including **`cargo fetch --locked`** — DC-81's addendum-2 B1 found that without
it, `boundary::tests::workspace_and_product_boundaries_hold` fails, because it runs
`cargo metadata --locked --offline` and needs every target's graph cached.

**Do not treat a green compile as evidence.** Yesterday's macOS symlink defect passed eleven local gates
and eleven cross-target clippy runs, and was caught only by a real mutation job on the real platform. The
same class of defect is more likely here, not less.

**One stale comment to fix while you are in the file:** `ci.yml`'s fixture job says *"A fixture repository
is authored on Linux (repository mutation is Linux-only)"*. That was already stale for macOS and becomes
more so here.

## 4. Criterion 2 is the one that will be tempting to fudge

**Every one of the nine guarantees is either held on Windows or documented as weaker in
`docs/src/reference/platform-support.md`, with the reason and the operational consequence.**

`design-v1.md` §3's table is the starting point, not the deliverable — it states the intent; the
deliverable states what you actually built. **"The method returns `Ok`" is not evidence the guarantee
behind it holds**, and §5 says so explicitly.

`platform-support.md` already documents the G1 anchoring gap and the non-Unix read-path gaps. **Extend
that section; do not start a parallel one.**

## 5. What must not change

- **No inline `#[cfg]` in shared function bodies** (§1).
- **No change to the method set or the nine guarantees.** If the port appears to need one, **stop and
  report** — §4 and §6 both say this is a design question that returns here.
- **No path-policy change.** DC-72 already rejects the Windows-hostile forms cross-platform (§6).
- **No repair of the read-path gaps.** Documented, and their own work (§6).
- **No new dependency and no `unsafe`.** Nothing in the design needs either; if you conclude otherwise,
  that is a §3.5 escalation with the crate, feature set, transitive count and lock effect reported
  **before** adding it — and `ALLOWED_THIRD_PARTY` is the architect's to amend.
- **`none.rs` stays.** Targets that are neither Linux, macOS nor Windows must still fail at runtime, not
  at build time.

## 6. Acceptance criteria

1. **Repository mutation works on Windows** — `init`, `commit`, `seal`, `branch`, `tag`, `merge`,
   `trust`, `compact`, `unlock`.
2. **A green `Windows mutation test suite` CI job** (§3).
3. **Every one of the nine guarantees held or documented weaker** (§4).
4. **DC-76's nine negative controls demonstrated on Windows**, or a reported reason each cannot be —
   DC-76's own precedent, where two could not be cleanly demonstrated and were reported rather than
   dropped.
5. **DC-71 preserved, demonstrated:** a no-implementor target still compiles, mutation there still fails
   at runtime, read-only still works.
6. **An interrupted `init` on Windows is refused or completed, never silently half-formed** — §4's
   exemption demonstrated, not inherited.
7. **No cross-platform history divergence**: a repository authored on Linux, mutated on Windows, and
   verified on Linux produces **identical object ids and a clean `verify`**. End to end, not reasoned
   about. This is criterion 7 and it is the one that proves the port rather than the code.
8. **`target_os` count reported** before and after, per file, with the counting rule stated. **Report the
   number; it is not a target.** Stage 1's total moved by six while the thing that mattered went twenty
   to zero — the count cannot distinguish an inline branch from an implementor gate, and I ruled on it
   wrongly once already.
9. **`docs/src/reference/` reflects what ships** — three pages currently say mutation is Linux/macOS
   only, and `platform-support.md` needs §4's guarantee statements.
10. Full gate set per `EXECUTION-ORDER.md` §6 rule 9, plus green CI on **all three platforms including
    Windows mutation**.

## 7. Standing

- **Work on a branch.** Branch → push → green CI → merge.
- **Report counts** per rule 10. Baseline at `6055d19`: `prikk-store` **738**, `prikk` **117**,
  `prikk-release-policy` 83, `prikk-object` 80, `prikk-replay` 44, `prikk-hash` 14, `prikk-crypto` 7,
  `prikk-error` 0; **179 locked packages**. Report the figures; the architect updates the line at merge.
- **A stop-and-report is a complete outcome.** §5 names three places it would be the right one.
