# Two gates that do not exist: advisory monitoring in CI, and documentation — implementation handoff

**Authority:** `rfcs/proposed/126-verification-infrastructure-coverage.md` §3 and §4.
**Base:** current `main` (`42d0d16`). **Under `003-landing-work-on-main.md`.**

**Scope: §3 and §4 only** — RFC 126's cheap half. **§2 (oracle-backed property tests for the patch
algebra) and §5 (benchmarks, criterion in its own member) are not this increment.**

**Why these two together:** both are "a check that does not run", both are CI-shaped, and both close
a hole that lets everything else drift silently. Neither touches product code.

---

## 1. `cargo audit` runs in the gate set and not in CI

`cargo audit --no-fetch` is one of the standing gate commands in `EXECUTION-ORDER.md` §6 rule 9, so
it runs on every gate pass. **`grep -rn "cargo audit" .github/workflows/` returns nothing.**

Two distinct gaps follow, and the second is the one that matters:

1. Advisory checking depends on a human running the gate set.
2. **It uses `--no-fetch`, so it is only ever as current as whoever last updated their local
   database.** A new advisory against one of the 25 shipped crates arrives on its own schedule, not
   on ours — **this is the only check in the project that can find a problem with nobody changing
   the code.**

**Build a scheduled CI job that fetches.** Scheduled, not per-PR: the per-PR value is near zero (the
dependency set rarely changes) and the fetch cost is real.

### 1.1 A ruling, because it is a supply-chain decision and not yours to take

`cargo-audit` is not preinstalled on GitHub runners. The obvious shortcut is the third-party
`rustsec/audit-check` action.

**Do not use it. Install the tool: `cargo install cargo-audit --locked`.**

The workflows currently use `actions/*` and `dtolnay/rust-toolchain`. **Adding a third-party action
whose job is to tell us whether our dependencies are trustworthy puts a new opaque dependency on the
security-monitoring path itself** — the one place this project should not widen its trust surface for
convenience. A scheduled job can afford a slow install.

### 1.2 What it must do when it finds something

**Decide and state this explicitly rather than inheriting a default.** A scheduled job that fails
silently into a red badge nobody watches is the vacuous-gate shape RFC 127 exists to correct. At
minimum the failure must be attributable — the job named for what it checks, and its output naming
the advisory and the crate.

## 2. Documentation is never gated

### 2.1 `cargo doc` never runs in CI, and there are exactly 7 warnings

```
warning: public documentation for `derive_next_state_root` links to private item `LineageStateMemo`
… (5 more) …
warning: public documentation for `commit_worktree_changes_signed` links to private item `NodeIdGenerator`
warning: `prikk-store` (lib doc) generated 7 warnings
```

All 7 in `prikk-store` — `block_state.rs` ×5, `bundle.rs`, `worktree_patch.rs`. **Each renders a
public doc item's link as literal `[`Name`]` text** — a reader of the published API sees markup.

**Fix the 7, then gate: `cargo doc --workspace --no-deps` with
`-D rustdoc::private_intra_doc_links`.** Fixing first and gating second, in that order, in one
commit — a gate landed against a red tree is not a gate.

**On how to fix them**: linking to a private item from a public doc is the symptom; the cause is
usually that the doc explains a public thing by naming a private one. Prefer rewording so the public
doc stands on its own; demoting the link to plain text is acceptable where the private name is
genuinely the clearest referent. **Do not make an item `pub` to satisfy the lint** — that would widen
the crate's public surface, which the 2026-08-31 audit specifically praised as curated.

### 2.2 The book is never built on a PR

`docs.yml` triggers on **push to `main`, filtered to `docs/**`**. So a code change that falsifies a
documented claim triggers nothing, and the book is never built before merge.

**Build the book on pull requests touching `docs/**` or the CLI.** The CLI half is the point: RFC 118
binds documentation to `COMMANDS`, and the RFC 118 §8 doc-coverage gate runs under `cargo test` —
but nothing renders the book itself until after a merge to `main`.

**Do not change the deploy job's trigger.** Publishing stays push-to-`main`; this adds a build-only
check, and conflating the two would deploy from a PR.

## 3. Two constraints that have already cost this project a red `main`

**Both are recorded from RFC 122's own CI failure. Read them before writing a workflow line.**

1. **`command_scan`'s procedure lexer has no shell-keyword awareness.** `if`, `$( )` and `||` in a
   scanned workflow are unclassifiable and `reference-check` rejects them with
   `unclassified-procedure-command` / `unsupported-dynamic-command-head`. **Every step must be a bare
   command.** This is not a style preference; it is a gate that will refuse your commit.
2. **A `bash -e` command list cannot contain a command that signals findings by exit code.** Under
   RFC 121's ruled contract, `1` covers findings, not only failures. `cargo audit` exits non-zero on
   a *finding*, which for a scheduled advisory job is the entire point — so think about what the job
   should do with that before wiring it, and do not assume the conformance job's bare-command shape
   transfers.

**If these two constraints make a required shape impossible, stop and escalate** rather than
inventing a workaround — that is what happened in RFC 122, and escalating would have been cheaper
than the red `main`.

## 4. Controls

1. **`cargo doc` shown failing before the fix and passing after**, on the real tree — the 7 warnings
   named, then gone.
2. **The doc gate shown catching a new violation**: introduce a fresh private-item link in a scratch
   tree, show the gate refuse it, remove it.
3. **The book-build job proven to run on a PR-shaped event**, not merely added. State how you
   verified the trigger, since a `paths:` filter that never matches is the same vacuous gate.
4. **`reference-check` clean**, explicitly — §3's first constraint bites here more than anywhere
   else this year.
5. **Your enumeration of which workflow jobs you touched and why**, as a result. RFC 122's fix
   needed exactly four jobs and no others; state the equivalent here.

## 5. Gates

Full set from `EXECUTION-ORDER.md` §6 rule 9 against the final commit, **clippy as a single
invocation per target with the exit code captured explicitly**, plus `mdbook build` and
`cargo doc --workspace --no-deps`.

**No CI control** — that is the architect's at push time. Report the closest local evidence and say
plainly that it is not a substitute, as previous increments have.

One commit on `main`, local, **no push, no tag**.

## 6. Out of scope

RFC 126 §2 (algebra property tests) and §5 (benchmarks/criterion). Doctests for the kernel APIs —
also §4 of the RFC, but a separate increment. Coverage, mutation testing, sanitizers, Miri.
