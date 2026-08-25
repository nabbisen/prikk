# MSRV — write the rise policy, and gate its transcriptions

**Base:** current `main` (`da2b242`, CI green). **Under `003-landing-work-on-main.md`.**
**Closes:** ROADMAP's "MSRV policy — overdue, not pending" theme.

The floor is declared and cannot go lower: `rust-version = "1.85"` is the edition-2024 minimum.
**What is missing is the rule for when it may rise — and a gate, because that one fact is currently
transcribed into at least seven live places.**

---

## 1. The policy — adopt the ROADMAP's own proposal

Write it as the project's rule, in substance as ROADMAP already proposes:

> **MSRV rises only when a dependency or language requirement forces it, never for convenience, and a
> rise is a minor-version event naming the requirement that forced it.**

**Add what a rise obligates**, which the proposal implies but does not state: a `### Breaking change`
entry in `CHANGELOG.md` at the cut, **naming the dependency or feature that forced it** — a rise
whose cause is not recorded is indistinguishable from a rise for convenience, which is the thing the
rule forbids.

**Home: `docs/src/reference/release-compatibility.md`**, which already owns the MSRV paragraph and
the three `+1.85.0` commands. It is user-facing, which is correct — consumers need this rule more
than maintainers do.

**This is the one piece of authored judgment here. If you want to state it differently in substance —
not wording — stop and escalate**, because it is a project rule and the owner has already seen this
proposal. Wording is yours.

## 2. The gate — one declaration, every transcription bound

`Cargo.toml`'s `rust-version` is **the declaration**. Everything else is a copy, and nothing checks
that the copies agree.

**Live sites I found. Re-derive this list yourself and report anything I missed:**

| Site | What it holds |
|---|---|
| `Cargo.toml:28` | `rust-version = "1.85"` — **the authority** |
| `.github/workflows/ci.yml:33` | `dtolnay/rust-toolchain@1.85.0` — **the one that actually binds CI** |
| `.github/workflows/ci.yml:29` | job name `msrv-1.85.0` |
| `docs/src/contributing/development.md` | prose line plus three `cargo +1.85.0` commands |
| `docs/src/reference/release-compatibility.md` | prose line plus three `cargo +1.85.0` commands |
| `rfcs/EXECUTION-ORDER.md` §6 rule 9 | the `cargo +1.85.0 test` gate command |

**Two spellings, deliberately:** the manifest says `1.85`, every other site says `1.85.0`. **Normalize
and compare semantically — do not require identical strings**, and do not "fix" one spelling into the
other: `rust-version = "1.85"` and a toolchain pin of `1.85.0` are both correct in their own contexts.

**Historical sites must NOT be bound.** `MILESTONES.md`, `rfcs/README.md`,
`rfcs/IMPLEMENTATION-STATUS.md`, and anything under `rfcs/done/` record what was true then —
DC-46 is history, not a live assertion. **`rfcs/EXECUTION-ORDER.md` is the exception: it lives under
`rfcs/` but is live**, because §6 rule 9 is the gate command every increment is required to run
verbatim. **Say how you distinguished the two**, since a path-prefix rule alone gets this wrong.

**Home: `release-policy reference-check`.** It already scans
`docs/src/contributing/development.md` and `docs/src/reference/release-compatibility.md` (its own
`REQUIRED_LIVE_PATHS`), and already has `scan_yaml` for the workflow. **If you conclude a sibling
subcommand is cleaner than extending `reference-check`, say why** — but do not build a third
mechanism when an existing one already reads two of the six.

**`tools/release-policy` is not shipped and may use its dependencies freely.** This is not
`prikk-cli`; do not hand-roll anything here.

## 3. The gate must fail on a real rise, not just a typo

**The test that matters is the one that runs when someone raises the MSRV for real.** Change
`Cargo.toml` to a higher version and the gate must name **every** site still holding the old one — so
whoever raises it gets a checklist, not a single first failure.

**Report all mismatches, not the first.** A gate that stops at the first stale site turns one edit
into six review rounds.

## 4. Out of scope

- **Raising the MSRV.** It stays at 1.85.
- **Changing any `cargo +1.85.0` command's shape**, or the CI job's structure.
- **Binding historical documents** (§2).
- **The `python_baseline_commit` field** in the oracle's observation document — a leftover from RFC 119
  track B's Python removal. **Noticed, not yours, do not touch it here** — tell me if it is stale and
  I will schedule it.

## 5. Delete the ROADMAP theme

Once the policy exists, the theme is delivered. **Delete the whole "MSRV policy" section**, matching
the precedent set by patch aggregation (`10a2a13`) and structured output (`3717220`). Its closing line
— *"Writing the policy itself is a separate increment"* — is exactly what this increment is.

## 6. Controls

1. **The gate fires on a raised MSRV**: set `rust-version` to a higher version, show the gate names
   **every** stale site, quote the output, revert.
2. **The gate fires on a drifted transcription**: leave `Cargo.toml` alone and change **one** site
   (the CI toolchain pin is the highest-value one), show it is caught, revert.
3. **Spelling normalization is real**: show `1.85` in the manifest and `1.85.0` in a pin pass together,
   and that a genuine mismatch (`1.85` vs `1.86.0`) does not.
4. **Historical documents are not bound**: show the gate passes with `MILESTONES.md`'s `1.85`
   references untouched, and say what would happen if the MSRV rose — **those references must not
   become failures.**
5. **The gate passes unmodified**, and the full suite is green.

**Quote every failure.** If a mutation fails to apply or a control passes without your assertion
firing, **say so**.

## 7. What to report

1. **Your re-derived site list** (§2), including anything I missed.
2. **How you distinguished live from historical**, and where `EXECUTION-ORDER.md` landed.
3. **Where the gate lives**, and why, if not `reference-check`.
4. **All five controls** (§6), quoted.
5. **Full gate set against the exact commit, after the last edit** — including `mdbook build`, since
   this touches `docs/src/`.
6. **Every numbered requirement's disposition, including ones that went without incident.**
7. Anything here that was wrong.

**Stop and escalate, do not guess**, if: you would state the policy differently in substance (§1); the
live/historical split turns out to need a judgment call rather than a rule; or a site holds the MSRV in
a form that cannot be checked without executing it — **a gate that shells out to a toolchain is not
what this is.**
