# RFC 108 increment 3b — surface per-active state: `verify` names it out of scope, `doctor` reports it

**Authority:** RFC 108 §D3.3/§D3.4, ACCEPTED 2026-08-27. **Base:** `cc00659` or later `main`.
**Under `003-landing-work-on-main.md`** — commit locally on `main`, do not push, do not tag.

**This is not "doctor repair goes plural."** That is 3c. §1 says why, and the reason came from the
RFC rather than from the code.

---

## 1. Scoping, and a correction to my own reasoning

Reading the code, plural `verify` looked like the obvious next step: `wal_records`,
`trailing_partial_wal_bytes` and `active_wal_metadata_status` are scalars from a single-WAL read, and
making them plural is tractable — **17 references outside `verify.rs`, 11 of them tests, two
production consumers (`doctor.rs`, `prikk-cli/src/output/verification.rs`).**

**The accepted design says not to.** §D3.4:

> **`verify` reports Workspaces as out of scope, explicitly.** Unsealed work is outside sealed history
> by construction; `verify`'s claim is about sealed history. **Silence would be the wrong answer** —
> the project's own rule is that absence must be explicit. A named line saying "N workspaces, not
> verified here, by construction" is the shape.

**`verify`'s subject is sealed history. A workspace's unsealed WAL is not sealed history.** Verifying
other actives there would widen a claim the RFC deliberately scoped.

**And §D3.3 requires the opposite for recovery:**

> Under seal-late, unsealed Workspace work is protected *only* by its WAL. **Invariant 7 stops being
> an aspiration.** A design increment must show a Workspace's WAL recovering independently of every
> other.

**Those are consistent, and together they decide the split:** verification of sealed history stays
singular and says so; **recovery must be per-active.** `doctor` is the surface for repository health,
not `verify` — and increment 2 already set the precedent with
`push_missing_required_directory_issues`, a doctor-level check that deliberately does not route
through `verify`.

**So this increment is the readers, again before the writers** — the same principle that ordered
increment 2 before this arc's mutation work.

## 2. The change

### 2.1 `verify` names the workspaces it does not cover

A named element in the report — **not silence, and not a verification** — stating how many active
sessions exist and that this report covers `default` only, by construction.

**The count comes from `layout.active_session_names()`.** Do not add a second enumeration.

**Adjudicate the surface shape and justify it:** this touches text output and `--format json`
(RFC 118 stage 5). **Whether it is a `VerificationStage`, a report field, or something else is
yours** — the criterion is that a reader of either output form cannot miss it, and that `--format
json`'s consumers get a value they can act on rather than prose to parse.

**If you conclude this cannot be added without changing the JSON schema in a way that breaks an
existing consumer, stop and report.**

### 2.2 `doctor` reports per-active WAL state

Today doctor's WAL health comes from `verification.trailing_partial_wal_bytes` — a scalar for
`default`. **With a second active present, doctor is silent about its WAL**, which is the same
silent-hole shape increment 2 removed from `unlock`.

Doctor should enumerate actives and report each one's WAL state.

**The crux, and it is yours to adjudicate:** doing this means doctor reads `default`'s WAL itself
rather than taking verify's scalar, so **two surfaces read the same WAL for `default`.** Argue it
either way, but argue it:
- **For:** one uniform source inside doctor, no asymmetry between `default` and the rest, and the
  duplication is *by design* because §D3.4 keeps verify out of this.
- **Against:** two readers of one fact is the transcription shape this project keeps punishing.

**A third option exists and may be better than both: a shared read that both call.** If you take it,
say where it lives and why that is not just moving the duplication.

**Do not change what `verify` itself computes or reports about `default`'s WAL.** Its scalar has two
CLI consumers and is verify's own business.

### 2.3 Two carried items, both small

- **Widen `active_relative_path_builders_match_repository_relative_for_non_utf8_names` from
  `#[cfg(target_os = "linux")]` to `#[cfg(unix)]`.** Its real dependency is
  `std::os::unix::ffi::OsStrExt`; it writes no non-UTF-8 name to disk, so increment 2's APFS `EILSEQ`
  justification does not apply. **I verified `cfg(unix)` compiles and passes locally; macOS CI is the
  test of that reasoning.**
- **Fix `repair_repository`'s refusal message.** It says *"doctor repair refused because repository
  verification has errors"*, but increment 2's required-directory check is a doctor-level check that
  is not part of `verify_repository`. **A reader goes looking at `verify` output and finds nothing.**
  The refusal itself is correct and must not change — only the message.

## 3. What must not change

- **On-disk layout.** `init` still creates `active/default/` only.
- **No mutation goes plural.** `repair_repository` still repairs one WAL. **That is 3c**, and it
  carries §D3.3's independence demonstration.
- **`verify`'s existing claims about `default`** — same fields, same values, same stage outcomes.
- **No existing test assertion changes its expected value**, except where §2.3's message fix requires
  it. **Name that one explicitly; every other expectation must hold.**

## 4. Controls

1. **With one active, `verify` and `doctor` output is unchanged except for the new element.** Quote
   both before and after.
2. **With a hand-planted second active, both surfaces name it** — `verify` in its out-of-scope
   accounting, `doctor` in its per-active WAL report. Nothing creates a second active, so plant it.
3. **A second active with a trailing partial WAL is reported by doctor.** This is the silent hole
   §2.2 exists to close; prove it is closed, and prove the test fails without the change.
4. **`--format json` stays parseable and its existing keys keep their meanings.** State explicitly
   whether any key changed.
5. **Full gate set against the exact final commit.**
6. **Per-job cross-platform CI**, including the `cfg(unix)` widening's macOS result — **that job is
   the evidence for §2.3's first item, and I am asking you to report it rather than assume it.**

## 5. The report

To `.git-exclude/review-request/`. Include §2.1's and §2.2's adjudications with their reasoning, the
JSON answer from control 4, all six controls quoted, the full gate set, and **anything in this
handoff that was wrong** — including my 17-reference figure, which I measured but did not act on.
