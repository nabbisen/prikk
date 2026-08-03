# DC-71 Non-Linux Build Conformance - Handoff

**Cleared to start.** Accepted by the project owner on 2026-08-04, at
`rfcs/accepted/DC-71-NON-LINUX-BUILD-CONFORMANCE.md`.
**Authored by** the architect.
**Size:** the fix is probably small. **The CI half is the increment** — see §2.
**Touches:** `crates/prikk-store/src/fsutil/anchored/*`, `.github/workflows/ci.yml`, and any public
portability claim that is currently wrong.

## 1. The ruling you are building against

> **Portable read-only is a REQUIREMENT.** Owner ruling 2026-08-04, answering *"should a macOS or Windows
> user be able to verify a prikk repository?"* — **yes**.

This matters because prikk's positioning is auditor-first: a reviewer verifies a repository they did not
author. **If `prikk verify` runs only on Linux, an auditor on a Mac can verify nothing.**

**DC-37's Linux-only *mutation* boundary stands and is not reopened here.** Read-only commands must build
and run off Linux; committing and sealing need not.

## 2. The defect is small. The reason it existed is the increment

`fsutil/anchored/{immutable,regular,read}.rs` import helpers that are `#[cfg(target_os = "linux")]`-gated
at their definition sites in `anchored.rs`, but the imports are **inconsistently gated** — `read.rs:11-12`
gates one, others do not. Someone began gating and stopped.

**Nothing builds a non-Linux target.** `ci.yml` is Linux-only, so this rotted with no signal, and *partial*
gating is exactly what silent drift looks like — a full absence would suggest a decision.

**A fix that repairs the files and adds no build check will rot again**, and next time a user finds it
rather than a trial build. Criterion 3 is the one that matters; treat the gating repair as the easy half.

## 3. Answer these before designing

- **Which targets must build?** macOS and Windows are different amounts of work. Only
  `x86_64-pc-windows-gnu` has been tried (it fails); macOS is untested.
- **Is `prikk-store` the only affected crate?** The finding names it. `prikk-cli`, `prikk-replay`, and the
  rest are unchecked.
- **What is the read-only command set, concretely?** `verify`, `log`, `doctor`,
  `checkout --plan-only`, …? **It has never been enumerated**, and it determines how much must compile.
  Enumerate it and put the list somewhere durable — the absence of that list is part of how the claim
  drifted.

## 4. Traps

- **Fixing the three files and stopping.** §2. Without CI the fix is a snapshot, not a property.
- **Adding `#[cfg]` until it compiles.** Gating an import that a read-only path genuinely needs turns a
  build error into a missing capability. The command set from §3 is what tells you which is which.
- **Reopening mutation.** DC-37's boundary is out of scope; if a read-only path appears to need a mutation
  helper, that is a finding to report, not a gate to add.
- **Claiming a target works because it compiles.** Criterion 2 asks for read-only commands *running* on a
  non-Linux target, demonstrated.
- **Leaving a portability claim uncorrected.** `README.md` was corrected on 2026-08-04; check for others,
  including `docs/` and DC-37 itself.

## 5. Definition of done

§3's three questions answered and reported; the named targets compile; the read-only command set runs on at
least one non-Linux target, demonstrated not asserted; **a CI job builds each supported non-Linux target**;
every public portability claim consistent with what now works; `MILESTONES.md`'s release-claim-mismatch row
closed; mutation still Linux-only; full gate set per `rfcs/EXECUTION-ORDER.md` §6 rule 9 with test counts
before and after, **commands reported verbatim**.

## 6. Standing request

This defect shipped in a release whose README claimed the opposite, and the claim was mine — written
without verification, hours before a trial build disproved it. **The tracked finding warning about exactly
this mismatch had existed since the original architecture review.** If something here contradicts what the
code actually does, stop and report it.
